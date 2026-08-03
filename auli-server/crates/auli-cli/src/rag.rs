// RAG orchestration: embed the question, retrieve from the entity's servicos + faqs
// collections (scored), narrow each set by proximity, assemble the prompt, call the LLM,
// and log the exchange. The server embeds ONLY the question here; documents were embedded
// ahead of time by `auli update`. Retrieval reads immutable `ReadStore`s — no writes, no locks.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use auli_anon::{Anonimizador, TEXTO_FALLBACK_ERRO};
use auli_contract::{ConsultaPackPayload, render_consulta_block};
use auli_core::corpus::{FAQS, PARECERES, SERVICES};
use auli_retrieval::{Engine, Hit};
use tracing::{debug, error, info, trace, warn};

use crate::config::config;
use crate::entities::get_entity;
use crate::error::Result;
use crate::llm;
use crate::util::run_blocking;

// Per-kind adaptive selection. `score` is a cosine DISTANCE — lower is closer. `floor` is the
// always-keep count; `band` is the max distance ABOVE the best match still admitted.
//
// Defaults preserve parity with the old fixed-take behavior: `band = ∞` keeps every retrieved
// doc up to the ceiling (`Collection::n_results`). To enable adaptive narrowing, run real
// questions, read the per-kind score arrays printed below, and lower each band to just above
// where genuine matches separate from filler.
// As de serviços/FAQs são `pub(crate)` porque a ferramenta MCP `consultar_servicos_faqs` usa as
// MESMAS constantes — é o que faz a paridade com o chat ser automática: calibrar uma banda aqui
// move as duas faces junto. As de pareceres seguem privadas (o MCP não passa por este caminho).
pub(crate) const SVC_FLOOR: usize = 0;
pub(crate) const SVC_BAND: f32 = f32::INFINITY;
pub(crate) const FAQ_FLOOR: usize = 0;
pub(crate) const FAQ_BAND: f32 = f32::INFINITY;
const PAR_FLOOR: usize = 0;
const PAR_BAND: f32 = f32::INFINITY;

// Expansão por grafo (só no tipo Pareceres): quantos pareceres relacionados por co-citação de
// dispositivos anexar ao contexto, e o mínimo de dispositivos em comum para um parecer contar como
// relacionado. Opt-in por entidade — sem `dispositivos-index.json` (do `canonizar`), nada é
// anexado e o contexto fica idêntico ao de hoje.
const MAX_RELACIONADOS: usize = 3;
const MIN_DISPOSITIVOS_COMUNS: usize = 2;

/// Tempos por fase de uma consulta RAG, em milissegundos de relógio de parede — precisão
/// suficiente para a pergunta que este medidor responde ("onde foi o tempo?").
/// `retrieve_ms` inclui a montagem do contexto (nos pareceres: a leitura dos corpos no disco) —
/// é o custo total de "ter o RAG pronto" depois do embed.
///
/// As fases NÃO somam `total_ms`: `total` cobre também o que não é cronometrado (anonimização,
/// resolução de entidade, montagem do prompt, restauração da resposta). A diferença é essa cola.
#[derive(Debug, Clone, Copy, Default)]
pub struct TemposConsulta {
    pub embed_ms: u64,
    pub retrieve_ms: u64,
    pub llm_ms: u64,
    pub total_ms: u64,
}

impl TemposConsulta {
    /// Linha humana única, usada no log de auditoria. Formato estável de propósito:
    /// `grep -h "TEMPOS" logs/*.txt` vira uma série temporal de graça.
    pub fn linha(&self) -> String {
        format!(
            "embed: {} ms · retrieve+montagem: {} ms · llm: {} ms · total: {} ms",
            self.embed_ms, self.retrieve_ms, self.llm_ms, self.total_ms
        )
    }
}

/// Which corpus a question targets. Sent by the UI as an integer `type` (see `dto::Question`).
/// The default (and any missing/unknown code) is `ServicosFaqs`, preserving the original behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    /// Serviços + FAQs — the default RAG path.
    ServicosFaqs,
    /// Pareceres only.
    Pareceres,
}

impl QueryType {
    /// Map the wire code (`1`/`2`) to a `QueryType`. `None` or any unexpected value falls back to
    /// `ServicosFaqs`, so a malformed `type` degrades gracefully instead of failing the request.
    pub fn from_code(code: Option<u8>) -> Self {
        match code {
            Some(2) => QueryType::Pareceres,
            _ => QueryType::ServicosFaqs,
        }
    }

    /// Rótulo curto para o cabeçalho do log de auditoria.
    pub fn label(self) -> &'static str {
        match self {
            QueryType::ServicosFaqs => "servicos+faqs",
            QueryType::Pareceres => "pareceres",
        }
    }
}

/// Retrieve + narrow one collection, delegating to the engine. The collection may be absent (an
/// entity not in the registry) — then we contribute nothing, preserving the chat's tolerant
/// semantics: never fail the question over a missing collection. The (CPU-bound) scan runs on a
/// blocking worker thread. Note the engine's score array is logged by `search_embedded` itself,
/// keyed by collection name rather than by `label`.
///
/// Devolve os `Hit` inteiros (payload **e** score): o score alimenta a seção `ADERÊNCIA` do
/// registro de auditoria (ver [`montar_aderencia`]). Antes era descartado aqui.
pub(crate) async fn retrieve(
    engine: Arc<Engine>,
    collection: String,
    label: &'static str,
    embedding: Vec<f32>,
    n_results: usize,
    floor: usize,
    band: f32,
) -> Result<Vec<Hit>> {
    run_blocking(move || {
        match engine.search_embedded(&collection, &embedding, n_results, floor, band) {
            Ok(hits) => Ok(hits),
            Err(auli_retrieval::Error::ColecaoAusente(_)) => {
                warn!("coleção '{label}' ausente para esta entidade — ignorando");
                Ok(vec![])
            }
            Err(e) => Err(e.to_string().into()),
        }
    })
    .await
}

/// Os payloads dos hits, na ordem — o que `montar_rag_*` consome. Existe porque a montagem do
/// contexto é PURA (`&[String]`) e o score só interessa ao log: separar aqui mantém as travas de
/// paridade byte a byte apontando para o texto, e nada mais.
pub(crate) fn payloads(hits: &[Hit]) -> Vec<String> {
    hits.iter().map(|h| h.payload.clone()).collect()
}

/// Render retrieved docs into the RAG context block, one entry per doc (1-based index).
fn render(docs: &[String], fmt: impl Fn(usize, &str) -> String) -> String {
    docs.iter()
        .enumerate()
        .map(|(i, doc)| fmt(i + 1, doc))
        .collect()
}

// Montagem do contexto RAG — funções PURAS (sem I/O, sem Engine), extraídas do
// `exec_all_question` no G2 para que a paridade do formato tenha trava automatizada. Recebem
// documentos JÁ PRONTOS para renderizar: para pareceres, os blocos já montados por
// `bloco_parecer` (que é quem lê a árvore `docs/`). Ver o item 8 do G2 na TAREFA-MCP.
//
// A partir da ferramenta MCP `consultar_servicos_faqs`, `montar_rag_servicos_faqs` e `retrieve`
// são também a implementação da face MCP — é o que garante saída byte a byte igual à do chat.

/// Contexto do tipo `ServicosFaqs`: serviços numerados + FAQs numeradas, nesta ordem.
pub(crate) fn montar_rag_servicos_faqs(svc_docs: &[String], faq_docs: &[String]) -> String {
    let rag_service = render(svc_docs, |i, doc| format!("\n## servico\n{i}\n{doc}\n"));
    let rag_faq = render(faq_docs, |i, doc| format!("\n// Resultado: {i}\n{doc}\n"));
    format!("{}\n{}", rag_service, rag_faq)
}

/// Contexto do tipo `Pareceres`: um bloco numerado por parecer recuperado, e — quando a expansão
/// por grafo devolve algo — uma seção `## PARECER RELACIONADO` com os pareceres que citam os mesmos
/// dispositivos. Com `relacionados` vazio (default/sem grafo), a saída é BYTE-A-BYTE a de antes.
fn montar_rag_pareceres(blocos: &[String], relacionados: &[String]) -> String {
    let principal = render(blocos, |i, bloco| format!("\n## PARECER\n{i}\n{bloco}\n"));
    if relacionados.is_empty() {
        return principal;
    }
    let rel = render(relacionados, |i, bloco| {
        format!("\n## PARECER RELACIONADO\n{i}\n{bloco}\n")
    });
    format!("{principal}{rel}")
}

// ---- Seção ADERÊNCIA do registro de auditoria ----
//
// O quão perto da pergunta ficou CADA documento escolhido. Vive só no log: o bloco RAG que vai ao
// prompt do LLM não muda um byte (é o mesmo `rag` nos dois lugares, e as travas de paridade acima
// existem exatamente para isso). Serve a duas perguntas do auditor: "por que este documento
// entrou?" e "onde cortar a banda?" — daí as duas unidades na mesma linha.

/// Uma linha da seção: o rótulo do documento (mesmo índice 1-based do bloco RAG) e sua distância
/// cosseno. `None` = o documento não veio de busca vetorial (hoje só a expansão por grafo dos
/// pareceres relacionados, que é por co-citação de dispositivos).
pub(crate) type Aderencia = (String, Option<f32>);

/// Rotula os scores de UMA coleção com o índice que o documento tem no bloco RAG.
pub(crate) fn aderencia(
    rotulo: &str,
    scores: impl IntoIterator<Item = Option<f32>>,
) -> Vec<Aderencia> {
    scores
        .into_iter()
        .enumerate()
        .map(|(i, s)| (format!("{rotulo} {}", i + 1), s))
        .collect()
}

/// Dados → texto da seção. `aderência = 1 − distância` (maior = mais perto), e a distância crua
/// segue ao lado porque é a unidade de `SVC_BAND`/`FAQ_BAND`/`PAR_BAND`: a calibragem se lê
/// direto daqui, sem conversão de cabeça.
pub(crate) fn montar_aderencia(itens: &[Aderencia]) -> String {
    itens
        .iter()
        .map(|(rotulo, score)| match score {
            Some(d) => format!("{rotulo} · aderência {:.3} · distância {d:.3}\n", 1.0 - d),
            None => format!("{rotulo} · por co-citação de dispositivos (sem busca vetorial)\n"),
        })
        .collect()
}

/// Remonta o bloco de um parecer a partir do payload leve gravado no pack (G3): lê o corpo da árvore
/// `docs/` e delega ao contrato para montar o MESMO bloco de sempre.
///
/// Degradação graciosa (D1 do plano): se o corpo não puder ser lido/parseado, `error!` no log (com o
/// `doc_path`) e o bloco sai com o `resumo` no lugar do corpo, precedido de `[corpo indisponível — ver
/// link]`. Payload que não desserializa (não deveria passar pelo boot) cai no mesmo caminho seguro.
/// Nunca derruba a query — o passo de rede do LLM domina, então a leitura síncrona dos k arquivos é
/// irrelevante na latência.
/// Lê o corpo via a função livre do motor (`auli_retrieval::ler_corpo`), e não pelo método do
/// `Engine`, de propósito: assim esta função continua testável com um diretório temporário, sem
/// construir um `Engine` (que carregaria o BGE-M3). O `Engine` só fornece a raiz — `docs_root()`.
fn bloco_parecer(payload_json: &str, docs_root: &Path, entity_id: &str) -> String {
    let payload: ConsultaPackPayload = match serde_json::from_str(payload_json) {
        Ok(p) => p,
        Err(e) => {
            error!(
                "payload de parecer não desserializa ({e}) — pack incompatível passou pelo boot?"
            );
            return payload_json.to_string();
        }
    };
    match auli_retrieval::ler_corpo(docs_root, entity_id, &payload.doc_path) {
        Ok(corpo) => render_consulta_block(&payload, &corpo),
        Err(e) => {
            error!(doc_path = %payload.doc_path, "corpo indisponível ({e}) — degradando para o resumo");
            let corpo = format!("[corpo indisponível — ver link]\n{}", payload.resumo);
            render_consulta_block(&payload, &corpo)
        }
    }
}

pub async fn exec_all_question(
    engine: Arc<Engine>,
    anonimizador: Arc<Anonimizador>,
    question: String,
    entity: Option<String>,
    query_type: QueryType,
) -> Result<String> {
    debug!("Executando consulta: {}", question);

    let t_total = Instant::now();
    let mut tempos = TemposConsulta::default();

    // Anonimiza a pergunta uma vez, fail-closed (em erro usa o placeholder fixo, nunca o texto cru).
    // O `mapping` fica em memória, no escopo da requisição, para restaurar a resposta do LLM —
    // NUNCA é persistido.
    let (pergunta_anon, mapping) = match anonimizador.anonimizar(&question) {
        Ok(a) => (a.texto, Some(a.mapping)),
        Err(e) => {
            warn!("anonimização falhou: {e}");
            (TEXTO_FALLBACK_ERRO.to_string(), None)
        }
    };

    // stdout: sem IP e com a pergunta anonimizada (stdout costuma ser capturado).
    info!(
        entity = entity.as_deref().unwrap_or("rs"),
        query_type = ?query_type,
        "Consulta: {}", pergunta_anon
    );

    // Resolve the target entity. Unknown entity -> return the error text as the answer.
    let cfg = match get_entity(entity.as_deref()) {
        Ok(cfg) => cfg,
        Err(e) => {
            warn!("{}", e);
            return Ok(e);
        }
    };
    info!("Entidade: {} ({})", cfg.id, cfg.name);

    // Embed the question once (off the async worker thread), reuse for both retrievals.
    let t = Instant::now();
    let embedding = {
        let e = engine.clone();
        let q = question.clone();
        run_blocking(move || e.embed(&q).map_err(|err| err.to_string().into())).await?
    };
    tempos.embed_ms = t.elapsed().as_millis() as u64;

    // Assemble the RAG context for the requested query type. The system-prompt/LLM/log tail below is
    // shared across types (the entity prompt is reused for every type).
    // NOTA: o early-return de pareceres vazios está DENTRO do `match` e sai antes daqui — hoje sem
    // registro de auditoria, e continua assim (não instrumentado).
    let t = Instant::now();
    // `aderencia` acompanha `rag` em paralelo: mesmos documentos, mesma ordem — mas só o `rag` vai
    // ao prompt do LLM.
    let mut itens_aderencia: Vec<Aderencia> = Vec::new();
    let rag = match query_type {
        QueryType::ServicosFaqs => {
            // Retrieve this entity's servicos + faqs concurrently, both through the engine.
            let svc_fut = retrieve(
                engine.clone(),
                cfg.collection(SERVICES.kind),
                "svc",
                embedding.clone(),
                SERVICES.n_results,
                SVC_FLOOR,
                SVC_BAND,
            );
            let faq_fut = retrieve(
                engine.clone(),
                cfg.collection(FAQS.kind),
                "faq",
                embedding,
                FAQS.n_results,
                FAQ_FLOOR,
                FAQ_BAND,
            );
            let (svc_hits, faq_hits) = tokio::try_join!(svc_fut, faq_fut)?;
            info!(
                "Foram selecionados {} serviços e {} faqs",
                svc_hits.len(),
                faq_hits.len()
            );

            itens_aderencia.extend(aderencia("servico", svc_hits.iter().map(|h| Some(h.score))));
            itens_aderencia.extend(aderencia("faq", faq_hits.iter().map(|h| Some(h.score))));
            montar_rag_servicos_faqs(&payloads(&svc_hits), &payloads(&faq_hits))
        }
        QueryType::Pareceres => {
            let par_hits = retrieve(
                engine.clone(),
                cfg.collection(PARECERES.kind),
                "par",
                embedding,
                PARECERES.n_results,
                PAR_FLOOR,
                PAR_BAND,
            )
            .await?;

            // No pareceres vectorized for this entity yet — answer with a friendly notice instead of
            // prompting the LLM on empty context (which would invite a hallucinated answer).
            if par_hits.is_empty() {
                return Ok(
                    "A consulta de Pareceres ainda não está disponível para esta entidade."
                        .to_string(),
                );
            }

            // Expansão por grafo: pareceres que citam os MESMOS dispositivos dos recuperados — sinal
            // complementar ao do vetor (acha conexão jurídica que a similaridade textual erra).
            // Opt-in por entidade: sem `dispositivos-index.json`, `relacionados` fica vazio e o
            // contexto é idêntico ao de antes. Bloqueante (lê o índice + varre o pack + lê corpos).
            let relacionados = {
                let engine = engine.clone();
                let id = cfg.id.clone();
                let seeds: Vec<String> = par_hits
                    .iter()
                    .filter_map(|h| {
                        serde_json::from_str::<ConsultaPackPayload>(&h.payload)
                            .ok()
                            .map(|p| p.numero)
                    })
                    .collect();
                run_blocking(move || {
                    let nums = engine.pareceres_relacionados(
                        &id,
                        &seeds,
                        MAX_RELACIONADOS,
                        MIN_DISPOSITIVOS_COMUNS,
                    );
                    let blocos = nums
                        .iter()
                        .filter_map(|n| engine.bloco_por_numero(&id, n).ok().flatten())
                        .collect::<Vec<String>>();
                    Ok::<Vec<String>, crate::error::Error>(blocos)
                })
                .await?
            };

            // G3: cada doc recuperado é o payload LEVE (JSON, sem corpo). Remonta o bloco de sempre
            // lendo o corpo da árvore `docs/` da entidade (`<docs_root>/<id>/<doc_path>`).
            let blocos: Vec<String> = par_hits
                .iter()
                .map(|h| bloco_parecer(&h.payload, engine.docs_root(), &cfg.id))
                .collect();
            info!(
                "Foram selecionados {} pareceres (+{} relacionados por grafo)",
                blocos.len(),
                relacionados.len()
            );

            itens_aderencia.extend(aderencia("parecer", par_hits.iter().map(|h| Some(h.score))));
            // Os relacionados entram sem número: vieram por co-citação, não por vetor. Ficam na
            // lista mesmo assim para que a seção espelhe o bloco RAG documento a documento.
            itens_aderencia.extend(aderencia(
                "parecer relacionado",
                relacionados.iter().map(|_| None),
            ));
            montar_rag_pareceres(&blocos, &relacionados)
        }
    };
    tempos.retrieve_ms = t.elapsed().as_millis() as u64;

    // System prompt = base prompt (per query type) + RAG context, closed with the original delimiter.
    let base_prompt = match query_type {
        QueryType::ServicosFaqs => &cfg.system_prompt,
        QueryType::Pareceres => &cfg.pareceres_prompt,
    };
    let system_prompt = format!("{}{}'''", base_prompt, rag);
    trace!("System instructions with RAG: {}", system_prompt);

    // Fronteira do LLM: com o flag ligado (default), envia a pergunta ANONIMIZADA e restaura a
    // resposta antes de devolvê-la ao usuário; com o flag desligado, envia a original (comportamento
    // anterior). Os documentos do RAG são conteúdo público e NÃO passam por anonimização.
    let anonimizar = config().anonimizar_llm;
    let entrada_llm = if anonimizar {
        &pergunta_anon
    } else {
        &question
    };
    let t = Instant::now();
    let resposta_llm = llm::chat(&system_prompt, entrada_llm).await?;
    tempos.llm_ms = t.elapsed().as_millis() as u64;

    // Restaura os placeholders (`[CNPJ_1]` → valor original) antes de devolver — o usuário vê o valor
    // real, mas o LLM só viu o placeholder. Sem mapping (anonimização falhou) ou flag off: sem troca.
    let answer = match (anonimizar, &mapping) {
        (true, Some(m)) => anonimizador.restaurar(&resposta_llm, m),
        _ => resposta_llm,
    };

    // Resposta em `debug!` (não `info!`): evita despejar a resposta crua no stdout por padrão.
    debug!("Resposta: {}", answer);

    tempos.total_ms = t_total.elapsed().as_millis() as u64;
    // Linha estruturada no tracing: campos separados para filtrar/agregar no journal.
    info!(
        entity = %cfg.id,
        tipo = query_type.label(),
        embed_ms = tempos.embed_ms,
        retrieve_ms = tempos.retrieve_ms,
        llm_ms = tempos.llm_ms,
        total_ms = tempos.total_ms,
        "tempos da consulta"
    );

    log_question(&RegistroConsulta {
        entidade: &cfg.id,
        tipo: query_type.label(),
        original: &question,
        sanitizada: &pergunta_anon,
        answer: Some(&answer),
        aderencia: &montar_aderencia(&itens_aderencia),
        rag: &rag,
        tempos,
    })?;

    Ok(answer)
}

/// Uma consulta a registrar no log de auditoria — tudo que entra no arquivo, menos o carimbo de
/// hora (que é do gravador). Existe por **segurança de chamada**: cinco dos campos são `&str` e
/// trocar dois de lugar compila em silêncio, gravando a pergunta no lugar do contexto. Com o
/// struct, cada campo é nomeado no ponto de chamada e a transposição vira erro de compilação.
///
/// Ambas as faces montam este mesmo valor: o chat com `answer: Some(…)`, o MCP com `None` (não
/// chama LLM). Ver a doutrina de conteúdo em [`format_log_record`].
pub(crate) struct RegistroConsulta<'a> {
    pub entidade: &'a str,
    /// Rótulo do tipo no cabeçalho: `servicos+faqs`/`pareceres` no chat, `mcp:<ferramenta>` no MCP.
    pub tipo: &'a str,
    /// Texto CRU do usuário — contém PII por decisão (ver [`format_log_record`]).
    pub original: &'a str,
    pub sanitizada: &'a str,
    /// `None` = nenhum LLM foi chamado (caminho MCP): a seção `RESPOSTA` é omitida.
    pub answer: Option<&'a str>,
    /// Seção de proximidade já montada por [`montar_aderencia`]; vazia = sem busca vetorial.
    pub aderencia: &'a str,
    /// O contexto RAG — a MESMA string que foi ao prompt do LLM.
    pub rag: &'a str,
    pub tempos: TemposConsulta,
}

/// Monta o registro estruturado do log de auditoria: cabeçalho + seções rotuladas
/// (pergunta original, pergunta anonimizada, resposta e, por fim, o contexto RAG).
///
/// **O log em disco é DELIBERADAMENTE ÍNTEGRO: ele contém PII.** `PERGUNTA (ORIGINAL)` guarda o
/// texto cru do usuário, e `RESPOSTA` guarda a resposta já RESTAURADA (placeholders trocados de
/// volta pelos valores reais). É intencional: sem o par original/anonimizada lado a lado não há
/// como auditar o próprio anonimizador — o que ele pegou e, principalmente, o que deixou passar.
///
/// Isso **revisa** o §4 (Fase 2, "obrigatório") do plano do `auli-anon`, que mandava persistir só
/// o texto anonimizado. A contrapartida é que a proteção do log passa a ser de **acesso e
/// retenção**, não de conteúdo: `log_question` grava com 0700/0600, `logs/` está fora do git, e a
/// retenção é operada (`docs/auli_operations.md` §7). O que o `auli-anon` garante segue intacto e é
/// outra coisa: o LLM externo só vê placeholders (Fase 3) e o stdout em nível `info` só mostra a
/// pergunta anonimizada. **Quem tem leitura deste diretório tem leitura de PII** — tratar como tal
/// ao copiar, versionar ou compartilhar.
/// Pura (sem I/O) para ser testável.
///
/// O `stamp` fica de fora do [`RegistroConsulta`] porque é o único campo que a função pura NÃO
/// pode produzir: quem carimba a hora é o `log_question`, que também a usa no nome do arquivo.
fn format_log_record(stamp: &str, reg: &RegistroConsulta) -> String {
    let RegistroConsulta {
        entidade,
        tipo,
        original,
        sanitizada,
        answer,
        aderencia,
        rag,
        tempos,
    } = reg;
    let tempos = tempos.linha();
    let regua = "=".repeat(64);
    let secao = |titulo: &str| -> String {
        let base = format!("----- {titulo} ");
        let faltam = 64usize.saturating_sub(base.chars().count());
        format!("{base}{}", "-".repeat(faltam))
    };
    // `answer: None` = caminho MCP, que não chama LLM (D-MCP-5): quem redige a resposta é o
    // assistente do usuário, e ela nunca passa por este processo. A seção é OMITIDA em vez de
    // gravada vazia — vazio seria ambíguo com "o LLM devolveu string vazia". Com `Some`, o
    // registro é byte a byte o de sempre (ver `log_record_has_header_and_sections_in_order`).
    let resposta = match answer {
        Some(a) => format!("{}\n{a}\n\n", secao("RESPOSTA")),
        None => String::new(),
    };
    // Mesma disciplina do `answer`: seção vazia é OMITIDA, não gravada em branco. Vazio aqui
    // significa "esta chamada não fez busca vetorial" (só o `obter_parecer`, que vai pelo número),
    // e uma seção em branco seria lida como "buscou e não achou nada".
    let aderencia = match aderencia.is_empty() {
        false => format!(
            "{}\n{aderencia}\n",
            secao("ADERÊNCIA (proximidade da pergunta)")
        ),
        true => String::new(),
    };
    format!(
        "{regua}\n\
         CONSULTA · {stamp} · entidade: {entidade} · tipo: {tipo}\n\
         TEMPOS · {tempos}\n\
         {regua}\n\n\
         {}\n{original}\n\n\
         {}\n{sanitizada}\n\n\
         {resposta}\
         {aderencia}\
         {}\n{rag}\n\
         {regua}",
        secao("PERGUNTA (ORIGINAL)"),
        secao("PERGUNTA (ANONIMIZADA)"),
        secao("CONTEXTO RAG (documentos recuperados)"),
    )
}

/// `pub(crate)` porque a face MCP grava pelo MESMO caminho — mesmo diretório, mesmas permissões
/// (0700/0600), mesmo formato. Duplicar o gravador criaria dois lugares para acertar a doutrina de
/// §7.0 do `auli_operations.md`; `answer: None` é a única diferença (o MCP não chama LLM).
pub(crate) fn log_question(reg: &RegistroConsulta) -> std::io::Result<()> {
    // Diretório de logs configurável; default `./logs` (relativo ao CWD). O start_server.sh aponta
    // para a raiz do repo (`$ROOT/logs`) para não depender de onde o binário é lançado.
    let log_dir = std::env::var("AULI_LOG_DIR").unwrap_or_else(|_| "./logs".to_string());
    fs::create_dir_all(&log_dir)?;
    // O conteúdo é DELIBERADAMENTE íntegro (ver `format_log_record`), então a proteção é de
    // ACESSO: diretório 0700, arquivos 0600. `create_dir_all` não reaplica modo num diretório que
    // já existe, daí o `set_permissions` explícito a cada gravação — barato e idempotente, e
    // conserta sozinho um `logs/` herdado com modo frouxo.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&log_dir, fs::Permissions::from_mode(0o700))?;
    }
    let agora = chrono::Local::now();
    let path = format!("{}/{}.txt", log_dir, agora.format("%Y-%m-%d_%H-%M-%S"));
    let stamp = agora.format("%Y-%m-%d %H:%M:%S").to_string();
    let content = format_log_record(&stamp, reg);
    let mut opts = OpenOptions::new();
    opts.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600); // só vale na CRIAÇÃO; o nome tem timestamp, então todo arquivo é novo
    }
    let mut file = opts.open(&path)?;
    debug!("Log da consulta gravado em {}", path);
    writeln!(file, "{}", content)
}

#[cfg(test)]
mod tests {
    use super::{
        QueryType, RegistroConsulta, TemposConsulta, aderencia, bloco_parecer, format_log_record,
        montar_aderencia, montar_rag_pareceres, montar_rag_servicos_faqs,
    };
    use auli_contract::{ConsultaPackPayload, mddoc};
    use std::path::Path;

    fn payload(doc_path: &str) -> String {
        serde_json::to_string(&ConsultaPackPayload {
            numero: "PARECER Nº 1".into(),
            assunto: "ICMS – crédito".into(),
            resumo: "Resumo do parecer.".into(),
            link: "http://x/1".into(),
            doc_path: doc_path.into(),
        })
        .unwrap()
    }

    #[test]
    fn bloco_parecer_le_corpo_da_arvore_e_monta_o_bloco() {
        // A raiz agora é `<docs_root>/<entity_id>/<doc_path>` (o motor resolve o `<id>`), então a
        // árvore do teste ganha o nível da entidade.
        let dir = std::env::temp_dir().join(format!("auli-rag-g3-ok-{}", std::process::id()));
        let pdir = dir.join("sc").join("docs/pareceres");
        std::fs::create_dir_all(&pdir).unwrap();
        let header = mddoc::DocHeader {
            numero: "PARECER Nº 1".into(),
            assunto: "ICMS – crédito".into(),
            link: "http://x/1".into(),
            sinopse_info: None,
        };
        std::fs::write(
            pdir.join("parecer-no-1.md"),
            mddoc::render_doc(&header, None, "É o corpo integral."),
        )
        .unwrap();

        let bloco = bloco_parecer(&payload("docs/pareceres/parecer-no-1.md"), &dir, "sc");
        // Bloco de sempre, com o corpo lido da árvore.
        assert_eq!(
            bloco,
            "## pergunta\nPARECER Nº 1\nICMS – crédito\n\n## resposta\nÉ o corpo integral.\nLink: http://x/1"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bloco_parecer_degrada_para_o_resumo_quando_o_arquivo_falta() {
        let dir = std::env::temp_dir().join(format!("auli-rag-g3-miss-{}", std::process::id()));
        // Nada materializado: o doc_path aponta para arquivo inexistente.
        let bloco = bloco_parecer(&payload("docs/pareceres/parecer-no-1.md"), &dir, "sc");
        assert!(
            bloco.contains("[corpo indisponível — ver link]"),
            "bloco: {bloco}"
        );
        assert!(
            bloco.contains("Resumo do parecer."),
            "usa o resumo no lugar do corpo"
        );
        assert!(bloco.contains("Link: http://x/1"), "preserva o link");
        // Nunca propaga erro — a query segue.
    }

    #[test]
    fn bloco_parecer_com_payload_invalido_nao_derruba() {
        // Pack incompatível que (hipoteticamente) passou pelo boot: não desserializa → serve cru.
        let bloco = bloco_parecer("isto não é json", Path::new("/inexistente"), "sc");
        assert_eq!(bloco, "isto não é json");
    }

    // NOTA: `ler_corpo_falha_em_arquivo_inexistente` e a bateria de `select_by_proximity`
    // (empty_input, default_band, finite_band, floor_overrides_band, band_zero) migraram para o
    // `auli-retrieval` no G1, junto com o código que testam. Não foram enfraquecidos nem
    // duplicados aqui.

    #[test]
    fn query_type_label_is_stable() {
        assert_eq!(QueryType::ServicosFaqs.label(), "servicos+faqs");
        assert_eq!(QueryType::Pareceres.label(), "pareceres");
    }

    #[test]
    fn tempos_linha_pina_o_formato() {
        // Contrato de grep, não estética: `grep -h TEMPOS logs/*.txt` depende deste formato.
        let t = TemposConsulta {
            embed_ms: 1,
            retrieve_ms: 2,
            llm_ms: 3,
            total_ms: 6,
        };
        assert_eq!(
            t.linha(),
            "embed: 1 ms · retrieve+montagem: 2 ms · llm: 3 ms · total: 6 ms"
        );
    }

    #[test]
    fn log_record_has_header_and_sections_in_order() {
        let rec = format_log_record(
            "2026-07-16 14:23:05",
            &RegistroConsulta {
                entidade: "rs",
                tipo: "pareceres",
                original: "CNPJ 11.222.333/0001-81 pode aderir?",
                sanitizada: "CNPJ [CNPJ_1] pode aderir?",
                answer: Some("Sim, o CNPJ 11.222.333/0001-81 atende."),
                aderencia: "parecer 1 · aderência 0.700 · distância 0.300\n",
                rag: "## PARECER\n0\n...",
                tempos: TemposConsulta {
                    embed_ms: 12,
                    retrieve_ms: 3,
                    llm_ms: 950,
                    total_ms: 970,
                },
            },
        );

        // Cabeçalho com metadados (data, entidade, tipo) — sem IP.
        assert!(rec.contains("CONSULTA · 2026-07-16 14:23:05 · entidade: rs · tipo: pareceres"));

        // A linha TEMPOS entra ENTRE o cabeçalho CONSULTA e a régua — posição, não só presença.
        let i_consulta = rec.find("CONSULTA · 2026-07-16").expect("linha CONSULTA");
        let i_tempos = rec.find("TEMPOS · embed: 12 ms").expect("linha TEMPOS");
        let i_regua = rec[i_consulta..]
            .find("====")
            .map(|d| i_consulta + d)
            .expect("régua após CONSULTA");
        assert!(
            i_consulta < i_tempos && i_tempos < i_regua,
            "TEMPOS deve ficar entre CONSULTA e a régua"
        );

        // As cinco seções, na ordem: original → anonimizada → resposta → aderência → contexto RAG.
        // A aderência vem ANTES do contexto de propósito: primeiro por que cada documento entrou,
        // depois o texto dele.
        let i_orig = rec.find("PERGUNTA (ORIGINAL)").expect("seção original");
        let i_anon = rec
            .find("PERGUNTA (ANONIMIZADA)")
            .expect("seção anonimizada");
        let i_resp = rec.find("RESPOSTA").expect("seção resposta");
        let i_ader = rec.find("ADERÊNCIA").expect("seção aderência");
        let i_rag = rec.find("CONTEXTO RAG").expect("seção rag");
        assert!(i_orig < i_anon && i_anon < i_resp && i_resp < i_ader && i_ader < i_rag);
        assert!(rec.contains("parecer 1 · aderência 0.700 · distância 0.300"));

        // A original mantém o PII; a anonimizada tem o placeholder; a resposta fica como veio.
        assert!(rec.contains("CNPJ 11.222.333/0001-81 pode aderir?"));
        assert!(rec.contains("CNPJ [CNPJ_1] pode aderir?"));
        assert!(rec.contains("Sim, o CNPJ 11.222.333/0001-81 atende."));
    }

    /// O registro do caminho MCP: mesmo cabeçalho, mesmas seções de pergunta e de contexto — e
    /// **sem** a seção RESPOSTA, porque nenhum LLM é chamado ali (D-MCP-5). Omitir é diferente de
    /// gravar vazio: uma seção `RESPOSTA` em branco seria lida como "o modelo não respondeu".
    #[test]
    fn log_record_do_mcp_omite_a_secao_resposta() {
        let rec = format_log_record(
            "2026-08-02 19:48:01",
            &RegistroConsulta {
                entidade: "rs",
                tipo: "mcp:consultar_servicos_faqs",
                original: "como parcelar ICMS em atraso?",
                sanitizada: "como parcelar ICMS em atraso?",
                answer: None,
                aderencia: "servico 1 · aderência 0.550 · distância 0.450\n",
                rag: "\n## servico\n1\n...",
                tempos: TemposConsulta {
                    embed_ms: 14,
                    retrieve_ms: 2,
                    llm_ms: 0,
                    total_ms: 17,
                },
            },
        );

        assert!(
            !rec.contains("RESPOSTA"),
            "a seção RESPOSTA não pode existir no caminho MCP: {rec}"
        );
        // O resto do registro é o de sempre, na mesma ordem.
        let i_orig = rec.find("PERGUNTA (ORIGINAL)").expect("seção original");
        let i_anon = rec
            .find("PERGUNTA (ANONIMIZADA)")
            .expect("seção anonimizada");
        let i_rag = rec.find("CONTEXTO RAG").expect("seção rag");
        assert!(i_orig < i_anon && i_anon < i_rag);
        assert!(rec.contains("tipo: mcp:consultar_servicos_faqs"), "{rec}");
        // `llm: 0 ms` preserva o contrato de grep da linha TEMPOS (ver `tempos_linha_pina_o_formato`).
        assert!(rec.contains("llm: 0 ms"), "{rec}");
    }

    #[test]
    fn aderencia_numera_como_o_bloco_rag_e_converte_a_distancia() {
        // Índice 1-based, igual ao do bloco RAG — é o que permite cruzar as duas seções do log.
        // `aderência = 1 − distância`, e a distância crua fica ao lado (unidade das bandas).
        let itens = aderencia("servico", [Some(0.25f32), Some(0.5)]);
        assert_eq!(
            montar_aderencia(&itens),
            "servico 1 · aderência 0.750 · distância 0.250\n\
             servico 2 · aderência 0.500 · distância 0.500\n"
        );
    }

    #[test]
    fn aderencia_sem_score_marca_a_origem_por_grafo() {
        // Pareceres relacionados vêm por co-citação de dispositivos, não por vetor: não têm
        // distância nenhuma, e inventar um número (0.0? 1.0?) seria mentir no registro.
        let itens = aderencia("parecer relacionado", [None]);
        assert_eq!(
            montar_aderencia(&itens),
            "parecer relacionado 1 · por co-citação de dispositivos (sem busca vetorial)\n"
        );
        // Sem documento nenhum, a seção sai vazia — e `format_log_record` a omite.
        assert_eq!(montar_aderencia(&[]), "");
    }

    #[test]
    fn log_record_omite_a_secao_aderencia_quando_nao_houve_busca() {
        // Caminho do `obter_parecer`: o documento é achado pelo número exato, sem vetor.
        let rec = format_log_record(
            "2026-08-02 19:48:01",
            &RegistroConsulta {
                entidade: "sc",
                tipo: "mcp:obter_parecer",
                original: "CONSULTA COPAT nº 0091/17",
                sanitizada: "CONSULTA COPAT nº 0091/17",
                answer: None,
                aderencia: "",
                rag: "{\"numero\":\"CONSULTA COPAT nº 0091/17\"}",
                tempos: TemposConsulta {
                    retrieve_ms: 1,
                    total_ms: 1,
                    ..Default::default()
                },
            },
        );
        assert!(!rec.contains("ADERÊNCIA"), "{rec}");
    }

    #[test]
    fn query_type_from_code_maps_2_to_pareceres_and_everything_else_to_default() {
        assert_eq!(QueryType::from_code(Some(2)), QueryType::Pareceres);
        assert_eq!(QueryType::from_code(Some(1)), QueryType::ServicosFaqs);
        assert_eq!(QueryType::from_code(None), QueryType::ServicosFaqs);
        // Unknown code degrades to the default rather than erroring.
        assert_eq!(QueryType::from_code(Some(9)), QueryType::ServicosFaqs);
    }

    // ---- Trava de paridade do formato do contexto RAG (item 8 do G2) ----
    //
    // Estes dois testes pinam BYTE A BYTE a string que vai ao prompt do LLM e ao log de auditoria.
    // São a razão de `montar_rag_*` existir: o G2 reescreve o caminho vivo do chat e, sem eles, a
    // única verificação de paridade seria o diff manual de log. Se um destes asserts falhar num
    // refactor futuro, o contexto do RAG mudou — o que muda a resposta do modelo.

    #[test]
    fn montar_rag_servicos_faqs_pina_o_formato() {
        let svc = vec!["SERVICO A".to_string(), "SERVICO B".to_string()];
        let faq = vec!["FAQ X".to_string()];
        assert_eq!(
            montar_rag_servicos_faqs(&svc, &faq),
            "\n## servico\n1\nSERVICO A\n\n## servico\n2\nSERVICO B\n\n\n// Resultado: 1\nFAQ X\n"
        );
    }

    #[test]
    fn montar_rag_servicos_faqs_com_listas_vazias() {
        // Caso real: entidade sem faqs. O separador `\n` entre os dois blocos permanece — é o que
        // o formato sempre produziu, e mudá-lo mudaria o prompt.
        assert_eq!(montar_rag_servicos_faqs(&[], &[]), "\n");
        assert_eq!(
            montar_rag_servicos_faqs(&["SO SERVICO".to_string()], &[]),
            "\n## servico\n1\nSO SERVICO\n\n"
        );
    }

    #[test]
    fn montar_rag_pareceres_pina_o_formato() {
        // Sem relacionados: BYTE-A-BYTE o formato de sempre (a expansão por grafo não pode mudar o
        // contexto quando não há grafo/entidade sem índice).
        let blocos = vec!["BLOCO UM".to_string(), "BLOCO DOIS".to_string()];
        assert_eq!(
            montar_rag_pareceres(&blocos, &[]),
            "\n## PARECER\n1\nBLOCO UM\n\n## PARECER\n2\nBLOCO DOIS\n"
        );
        assert_eq!(montar_rag_pareceres(&[], &[]), "");
    }

    #[test]
    fn montar_rag_pareceres_anexa_secao_relacionados() {
        // Com relacionados: seção `## PARECER RELACIONADO` numerada à parte, DEPOIS dos recuperados.
        let blocos = vec!["BLOCO UM".to_string()];
        let rel = vec!["BLOCO REL".to_string()];
        assert_eq!(
            montar_rag_pareceres(&blocos, &rel),
            "\n## PARECER\n1\nBLOCO UM\n\n## PARECER RELACIONADO\n1\nBLOCO REL\n"
        );
    }
}
