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
use auli_contract::{DocumentoPack, Kind, bloco, conteudo_indisponivel};
use auli_core::corpus::{self, FAQS, SERVICES};
use auli_retrieval::{Engine, Hit};
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

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
    /// Uma coleção de jurisprudência, sozinha: pareceres ou acórdãos do TARF. O caminho é o mesmo
    /// (payload leve → corpo lido da árvore → bloco), e o que muda é a coleção, o rótulo do bloco e
    /// o prompt de sistema — tudo dado, nenhum código a duplicar.
    Jurisprudencia(Kind),
}

impl QueryType {
    /// O código de fio → `QueryType`. `None` ou valor inesperado cai em `ServicosFaqs`, então um
    /// `type` malformado degrada em vez de derrubar a requisição.
    ///
    /// Os códigos são **contrato com o frontend** e não podem ser reordenados: `2` é pareceres desde
    /// a primeira versão da tab do auditor, e `3` é o TARF.
    pub fn from_code(code: Option<u8>) -> Self {
        match code {
            Some(2) => QueryType::Jurisprudencia(Kind::Pareceres),
            Some(3) => QueryType::Jurisprudencia(Kind::Tarf),
            _ => QueryType::ServicosFaqs,
        }
    }

    /// Rótulo curto para o cabeçalho do log de auditoria. Contrato de grep
    /// (`grep "tipo: pareceres" logs/*.txt`) — `pareceres` segue sendo `pareceres`.
    pub fn label(self) -> &'static str {
        match self {
            QueryType::ServicosFaqs => "servicos+faqs",
            QueryType::Jurisprudencia(k) => k.as_str(),
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

/// **O invólucro único** de todo documento do contexto (D-MARC-1).
///
/// Antes havia quatro — `## servico`, `// Resultado:`, `## PARECER` e `## PARECER RELACIONADO` —,
/// um por coleção, com sintaxes que nem sequer combinavam entre si (uma delas era um comentário de
/// JavaScript). O que distingue as coleções é a **natureza da fonte**, que a LLM precisa saber para
/// citar direito; isso é DADO, não template. Então o template virou um só e a natureza virou o
/// rótulo:
///
/// ```text
/// ## documento 3: Parecer
/// ```
///
/// O template INTERNO do bloco (`## pergunta` / `## resposta` / `Link:`) não muda um byte — ele já
/// era unificado, e continua vindo pronto de quem chama.
fn envolver(rotulo: &str, i: usize, conteudo: &str) -> String {
    format!("\n## documento {i}: {rotulo}\n{conteudo}\n")
}

/// O rótulo do documento que veio por **expansão de grafo**, não por busca vetorial.
///
/// Fica aqui, e não no `Kind`, porque não é uma coleção: é um PAPEL que um parecer assume neste
/// contexto. O documento é o mesmo `Kind::Pareceres`; o que muda é como ele chegou — por co-citação
/// de dispositivos —, e a LLM precisa saber disso para citá-lo com o peso certo.
const ROTULO_PARECER_RELACIONADO: &str = "Parecer relacionado";

/// Contexto do tipo `ServicosFaqs`: serviços numerados + FAQs numeradas, nesta ordem.
///
/// A numeração segue **por coleção** (serviços 1..n, depois FAQs 1..m), e não corrida (D-MARC-3):
/// a seção ADERÊNCIA do log rotula por coleção+índice, e a correspondência 1:1 com o bloco tem que
/// se manter. Unifica-se o template, não a contagem.
pub(crate) fn montar_rag_servicos_faqs(svc_docs: &[String], faq_docs: &[String]) -> String {
    let rag_service = render(svc_docs, |i, doc| envolver(Kind::Servicos.rotulo(), i, doc));
    let rag_faq = render(faq_docs, |i, doc| envolver(Kind::Faqs.rotulo(), i, doc));
    format!("{}\n{}", rag_service, rag_faq)
}

/// Contexto do tipo `Jurisprudencia`: um bloco numerado por documento recuperado, e — quando a
/// expansão por grafo devolve algo — os pareceres que citam os mesmos dispositivos, rotulados como
/// relacionados. Com `relacionados` vazio (default/sem grafo, e SEMPRE no TARF — D-A4), só a
/// primeira seção sai.
fn montar_rag_jurisprudencia(kind: Kind, blocos: &[String], relacionados: &[String]) -> String {
    let principal = render(blocos, |i, bloco| envolver(kind.rotulo(), i, bloco));
    if relacionados.is_empty() {
        return principal;
    }
    let rel = render(relacionados, |i, bloco| {
        envolver(ROTULO_PARECER_RELACIONADO, i, bloco)
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

/// Remonta o bloco de UM documento a partir do payload do pack v2: lê o conteúdo da árvore `docs/`
/// e delega ao contrato para montar o bloco.
///
/// **Vale para as quatro coleções.** Antes da B4 só a jurisprudência passava por aqui — serviços e
/// FAQs traziam o bloco inteiro pré-renderizado dentro do pack, o que duplicava o texto integral de
/// cada documento (uma cópia na árvore, outra no vetor).
///
/// Degradação graciosa (D-B3): conteúdo ilegível vira [`conteudo_indisponivel`], que difere por
/// coleção — a jurisprudência serve o `resumo`, que é síntese legítima; serviços e FAQs, que não têm
/// resumo, servem um aviso. O `Link:` do bloco fica nos dois casos. Payload que não desserializa
/// (não deveria passar pelo boot, que valida a `STRATEGY_VERSION`) cai no caminho seguro de servir o
/// cru. Nunca derruba a query — o passo de rede do LLM domina, e a leitura síncrona dos k arquivos é
/// irrelevante na latência.
///
/// Lê via a função livre do motor (`auli_retrieval::ler_corpo`), e não pelo método do `Engine`, de
/// propósito: assim esta função continua testável com um diretório temporário, sem construir um
/// `Engine` (que carregaria o BGE-M3). O `Engine` só fornece a raiz — `docs_root()`.
pub(crate) fn bloco_documento(payload_json: &str, docs_root: &Path, entity_id: &str) -> String {
    let payload: DocumentoPack = match serde_json::from_str(payload_json) {
        Ok(p) => p,
        Err(e) => {
            error!("payload não desserializa ({e}) — pack incompatível passou pelo boot?");
            return payload_json.to_string();
        }
    };
    match auli_retrieval::ler_corpo(docs_root, entity_id, &payload.doc_path) {
        Ok(corpo) => bloco(&payload, &corpo),
        Err(e) => {
            error!(doc_path = %payload.doc_path, kind = %payload.kind,
                "conteúdo indisponível ({e}) — degradando");
            bloco(&payload, &conteudo_indisponivel(&payload))
        }
    }
}

/// Os blocos de contexto de uma lista de hits, na ordem. Ponto único das duas faces: o chat e a
/// ferramenta MCP `consultar_servicos_faqs` passam por aqui, e é isso que mantém a paridade byte a
/// byte entre elas sem ninguém precisar lembrar.
pub(crate) fn blocos(hits: &[Hit], docs_root: &Path, entity_id: &str) -> Vec<String> {
    hits.iter()
        .map(|h| bloco_documento(&h.payload, docs_root, entity_id))
        .collect()
}

pub async fn exec_all_question(
    engine: Arc<Engine>,
    anonimizador: Arc<Anonimizador>,
    question: String,
    entity: Option<String>,
    query_type: QueryType,
) -> Result<(String, Option<Uuid>)> {
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
            // Sem `log_id`: este atalho sai antes de qualquer gravação. O ícone do log não aparece
            // porque de fato não há registro — mesma regra dos outros retornos precoces.
            return Ok((e, None));
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
            // Pack v2: o payload é leve, então o bloco de CADA serviço e de CADA faq é montado
            // aqui, lendo a árvore. Antes eles vinham prontos de dentro do pack.
            montar_rag_servicos_faqs(
                &blocos(&svc_hits, engine.docs_root(), &cfg.id),
                &blocos(&faq_hits, engine.docs_root(), &cfg.id),
            )
        }
        QueryType::Jurisprudencia(kind) => {
            let colecao = corpus::from_kind(kind.as_str())
                .expect("todo Kind é um kind de corpus — ver o teste de cobertura");
            let hits = retrieve(
                engine.clone(),
                cfg.collection(colecao.kind),
                "jur",
                embedding,
                colecao.n_results,
                PAR_FLOOR,
                PAR_BAND,
            )
            .await?;

            // Coleção não vetorizada para esta entidade — avisa em vez de mandar o LLM responder com
            // contexto vazio (que é convite a alucinação).
            if hits.is_empty() {
                // Idem: sai antes do `log_question`, logo sem `log_id`.
                return Ok((
                    format!(
                        "A consulta de {} ainda não está disponível para esta entidade.",
                        kind.titulo()
                    ),
                    None,
                ));
            }

            // Expansão por grafo: pareceres que citam os MESMOS dispositivos dos recuperados — sinal
            // complementar ao do vetor (acha conexão jurídica que a similaridade textual erra).
            // **Exclusiva dos pareceres na v1** (D-A4): o `dispositivos-index.json` é derivado da
            // árvore de pareceres, e o `bloco_por_numero` varre o pack de pareceres — apontá-lo para
            // acórdãos devolveria, no melhor caso, nada; no pior, um parecer no meio do contexto de
            // acórdãos, sem que a numeração do bloco denunciasse a troca.
            let relacionados = if kind == Kind::Pareceres {
                let engine = engine.clone();
                let id = cfg.id.clone();
                let seeds: Vec<String> = hits
                    .iter()
                    .filter_map(|h| {
                        serde_json::from_str::<DocumentoPack>(&h.payload)
                            .ok()
                            .map(|p| p.titulo)
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
            } else {
                Vec::new()
            };

            // G3: cada doc recuperado é o payload LEVE (JSON, sem corpo). Remonta o bloco de sempre
            // lendo o corpo da árvore `docs/` da entidade (`<docs_root>/<id>/<doc_path>`). O
            // `doc_path` já traz a coleção, então o mesmo montador serve as duas.
            let docs: Vec<String> = blocos(&hits, engine.docs_root(), &cfg.id);
            info!(
                "Foram selecionados {} {} (+{} relacionados por grafo)",
                docs.len(),
                kind.plural(),
                relacionados.len()
            );

            // O rótulo da ADERÊNCIA é o do documento em minúsculas — nos pareceres isso dá
            // exatamente `parecer`, o literal que o log usa desde sempre (ver o teste que o pina).
            // Trocá-lo por `pareceres` mudaria um formato de log já em uso, de graça.
            itens_aderencia.extend(aderencia(
                &kind.rotulo().to_lowercase(),
                hits.iter().map(|h| Some(h.score)),
            ));
            // Os relacionados entram sem número: vieram por co-citação, não por vetor. Ficam na
            // lista mesmo assim para que a seção espelhe o bloco RAG documento a documento.
            itens_aderencia.extend(aderencia(
                "parecer relacionado",
                relacionados.iter().map(|_| None),
            ));
            montar_rag_jurisprudencia(kind, &docs, &relacionados)
        }
    };
    tempos.retrieve_ms = t.elapsed().as_millis() as u64;

    // System prompt = base prompt (per query type) + RAG context, closed with the original delimiter.
    let base_prompt = match query_type {
        QueryType::ServicosFaqs => &cfg.system_prompt,
        QueryType::Jurisprudencia(kind) => cfg.prompt_de(kind),
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

    // Gravação NÃO-FATAL (D-LOG-2). Até 09/08/2026 esta linha terminava em `?`: o erro subia, e o
    // handler converte qualquer `Err` em texto de resposta — com o disco cheio, o usuário recebia
    // "Permission denied (os error 13)" no lugar da resposta, depois de o LLM já ter rodado e sido
    // pago. Perder o registro é ruim; perder a resposta que já custou é pior. Agora o erro vai ao
    // tracing e a consulta segue sem `log_id` — que é exatamente o sinal de "não há log para abrir"
    // que o frontend usa para não desenhar o ícone.
    let log_id = log_question(&RegistroConsulta {
        entidade: &cfg.id,
        tipo: query_type.label(),
        original: &question,
        sanitizada: &pergunta_anon,
        answer: Some(&answer),
        aderencia: &montar_aderencia(&itens_aderencia),
        rag: &rag,
        tempos,
    })
    .inspect_err(|e| warn!("falha ao gravar o log da consulta: {e}"))
    .ok();

    Ok((answer, log_id))
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
    // A IDENTIDADE DO ESPAÇO VETORIAL em que estas distâncias foram medidas. Sem ela, uma mudança
    // de fórmula (bump de `STRATEGY_VERSION`) exigia LEMBRAR de arquivar o log — e esquecer não dá
    // erro, dá análise silenciosamente errada, misturando distâncias de dois espaços. É a mesma
    // classe de defeito que o `docs_hash` fecha no boot. Com o carimbo, separar eras é `grep`.
    //
    // O trio vem do `manifest::identity()`, a MESMA fonte que o manifesto usa para decidir se pack
    // e servidor combinam — três formulações de uma identidade só aparecem quando duas divergem.
    let id = auli_core::manifest::identity();
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
         EMBED · modelo: {} · dim: {} · strategy: {}\n\
         {regua}\n\n\
         {}\n{original}\n\n\
         {}\n{sanitizada}\n\n\
         {resposta}\
         {aderencia}\
         {}\n{rag}\n\
         {regua}",
        id.embed_model_id,
        id.embed_dim,
        id.strategy_version,
        secao("PERGUNTA (ORIGINAL)"),
        secao("PERGUNTA (ANONIMIZADA)"),
        secao("CONTEXTO RAG (documentos recuperados)"),
    )
}

/// Diretório de logs configurável; default `./logs` (relativo ao CWD). O `start_server.sh` aponta
/// para a raiz do repo (`$ROOT/logs`) para não depender de onde o binário é lançado.
///
/// Compartilhado com o handler de `GET /v1/log/{uuid}`, que precisa procurar no MESMO lugar onde
/// se grava — duas cópias desta linha divergiriam no dia em que a variável mudasse de nome.
pub(crate) fn log_dir() -> String {
    std::env::var("AULI_LOG_DIR").unwrap_or_else(|_| "./logs".to_string())
}

/// `pub(crate)` porque a face MCP grava pelo MESMO caminho — mesmo diretório, mesmas permissões
/// (0700/0600), mesmo formato. Duplicar o gravador criaria dois lugares para acertar a doutrina de
/// §7.0 do `auli_operations.md`; `answer: None` é a única diferença (o MCP não chama LLM).
///
/// **Devolve o UUID v7 do registro** (D-LOG-1), que é a identidade do arquivo e a *capability* da
/// rota `GET /v1/log/{uuid}`: quem o possui lê aquele log, e não há como enumerar os demais. É aqui
/// que ele nasce — e não no handler do chat — porque é aqui que se sabe se a gravação deu certo, e
/// porque assim as DUAS faces ganham nome único (ver a colisão descrita abaixo).
pub(crate) fn log_question(reg: &RegistroConsulta) -> std::io::Result<Uuid> {
    let log_dir = log_dir();
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
    // O prefixo de data continua na frente porque é para HUMANOS: `ls logs/` segue cronológico e
    // legível. O UUID v7 vai de sufixo e desempata o que o prefixo não distingue — dois registros
    // no mesmo segundo —, porque seus 48 bits mais significativos são o timestamp em ms.
    let id = Uuid::now_v7();
    let path = format!("{}/{}_{id}.txt", log_dir, agora.format("%Y-%m-%d_%H-%M-%S"));
    let stamp = agora.format("%Y-%m-%d %H:%M:%S").to_string();
    let content = format_log_record(&stamp, reg);
    let mut opts = OpenOptions::new();
    // `create_new`, não `create().append()`. Antes o nome era só o timestamp em SEGUNDOS e o
    // arquivo abria em append — duas consultas no mesmo segundo (o rate limit tem burst 2, IPs
    // distintos não competem entre si, e o MCP faz várias chamadas por conversa) gravavam no MESMO
    // arquivo, podendo intercalar sob `O_APPEND`. Com a rota `GET /v1/log/{uuid}` isso deixaria de
    // ser bagunça e viraria vazamento: o glob de um UUID devolveria a pergunta de outra pessoa,
    // com a PII dela. Aqui a unicidade vira INVARIANTE VERIFICADA — se um dia colidir, o open
    // falha, o erro é registrado e a consulta segue sem `log_id` (nunca dois registros num arquivo).
    opts.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600); // só vale na CRIAÇÃO — e com `create_new` toda abertura é criação
    }
    let mut file = opts.open(&path)?;
    debug!("Log da consulta gravado em {}", path);
    writeln!(file, "{}", content)?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::{
        QueryType, RegistroConsulta, TemposConsulta, Uuid, aderencia, bloco_documento, envolver,
        format_log_record, log_question, montar_aderencia, montar_rag_jurisprudencia,
        montar_rag_servicos_faqs,
    };
    use auli_contract::{DocumentoPack, Kind, mddoc};
    use std::path::Path;

    fn payload(doc_path: &str) -> String {
        payload_de(
            Kind::Pareceres,
            "PARECER Nº 1",
            "Resumo do parecer.",
            doc_path,
        )
    }

    fn payload_de(kind: Kind, titulo: &str, resumo: &str, doc_path: &str) -> String {
        serde_json::to_string(&DocumentoPack {
            kind,
            trilha: String::new(),
            titulo: titulo.into(),
            ementa: "ICMS – crédito".into(),
            resumo: resumo.into(),
            link: "http://x/1".into(),
            doc_path: doc_path.into(),
        })
        .unwrap()
    }

    #[test]
    fn bloco_documento_le_corpo_da_arvore_e_monta_o_bloco() {
        // A raiz agora é `<docs_root>/<entity_id>/<doc_path>` (o motor resolve o `<id>`), então a
        // árvore do teste ganha o nível da entidade.
        let dir = std::env::temp_dir().join(format!("auli-rag-g3-ok-{}", std::process::id()));
        let pdir = dir.join("sc").join("docs/pareceres");
        std::fs::create_dir_all(&pdir).unwrap();
        let header =
            mddoc::cabecalho_jurisprudencia("PARECER Nº 1", "ICMS – crédito", "http://x/1");
        std::fs::write(
            pdir.join("parecer-no-1.md"),
            mddoc::render_doc(&header, None, "É o corpo integral."),
        )
        .unwrap();

        let bloco = bloco_documento(&payload("docs/pareceres/parecer-no-1.md"), &dir, "sc");
        // Bloco de sempre, com o corpo lido da árvore.
        assert_eq!(
            bloco,
            "## pergunta\nPARECER Nº 1\nICMS – crédito\n\n## resposta\nÉ o corpo integral.\nLink: http://x/1"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bloco_documento_degrada_para_o_resumo_quando_o_arquivo_falta() {
        let dir = std::env::temp_dir().join(format!("auli-rag-g3-miss-{}", std::process::id()));
        // Nada materializado: o doc_path aponta para arquivo inexistente.
        let bloco = bloco_documento(&payload("docs/pareceres/parecer-no-1.md"), &dir, "sc");
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

    /// O caminho NOVO da B4: serviços e FAQs também passam pelo `bloco_documento`, e degradam
    /// diferente da jurisprudência — eles não têm resumo para servir no lugar do corpo.
    #[test]
    fn bloco_documento_de_servico_degrada_sem_inventar_resumo() {
        let dir = std::env::temp_dir().join(format!("auli-rag-b4-svc-{}", std::process::id()));
        let p = payload_de(Kind::Servicos, "Emitir guia", "", "docs/servicos/sumiu.md");
        let bloco = bloco_documento(&p, &dir, "rs");
        assert!(
            bloco.contains("[conteúdo indisponível — ver o link oficial]"),
            "bloco: {bloco}"
        );
        assert!(
            !bloco.contains("[corpo indisponível"),
            "esse é o aviso da jurisprudência, que TEM resumo: {bloco}"
        );
        // O link sobrevive à degradação — é para onde o leitor vai.
        assert!(bloco.ends_with("Link: http://x/1"), "bloco: {bloco}");
    }

    /// A árvore de serviços é lida do MESMO jeito que a de pareceres — o `doc_path` do payload é
    /// que manda. Sem este teste, um erro na montagem do caminho por coleção só apareceria em
    /// produção, como todo serviço degradado.
    #[test]
    fn bloco_documento_le_a_arvore_de_servicos() {
        let dir = std::env::temp_dir().join(format!("auli-rag-b4-svc-ok-{}", std::process::id()));
        let sdir = dir.join("rs").join("docs/servicos");
        std::fs::create_dir_all(&sdir).unwrap();
        let header = mddoc::DocHeader {
            titulo: "Emitir guia".into(),
            trilha: "Empresas | ICMS".into(),
            ementa: String::new(),
            link: "http://x/1".into(),
            orgao: Some("Receita Estadual".into()),
            resumo_info: None,
        };
        std::fs::write(
            sdir.join("emitir.md"),
            mddoc::render_doc(&header, None, "Passos para emitir."),
        )
        .unwrap();

        let p = payload_de(Kind::Servicos, "Emitir guia", "", "docs/servicos/emitir.md");
        // A trilha do bloco vem do PAYLOAD, não do `.md` — é o pack que manda no que o LLM lê.
        let mut pack: auli_contract::DocumentoPack = serde_json::from_str(&p).unwrap();
        pack.trilha = "Empresas | ICMS".into();
        pack.ementa = String::new();
        let bloco = bloco_documento(&serde_json::to_string(&pack).unwrap(), &dir, "rs");
        assert_eq!(
            bloco,
            "## pergunta\nEmpresas | ICMS\nEmitir guia\n\n## resposta\nPassos para emitir.\nLink: http://x/1"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bloco_documento_com_payload_invalido_nao_derruba() {
        // Pack incompatível que (hipoteticamente) passou pelo boot: não desserializa → serve cru.
        let bloco = bloco_documento("isto não é json", Path::new("/inexistente"), "sc");
        assert_eq!(bloco, "isto não é json");
    }

    // NOTA: `ler_corpo_falha_em_arquivo_inexistente` e a bateria de `select_by_proximity`
    // (empty_input, default_band, finite_band, floor_overrides_band, band_zero) migraram para o
    // `auli-retrieval` no G1, junto com o código que testam. Não foram enfraquecidos nem
    // duplicados aqui.

    #[test]
    fn query_type_label_is_stable() {
        // Contrato de grep do cabeçalho `tipo: ...` do log de auditoria. `pareceres` continua
        // `pareceres` depois de a variante virar `Jurisprudencia(Kind)` — era o risco da mudança.
        assert_eq!(QueryType::ServicosFaqs.label(), "servicos+faqs");
        assert_eq!(
            QueryType::Jurisprudencia(Kind::Pareceres).label(),
            "pareceres"
        );
        assert_eq!(QueryType::Jurisprudencia(Kind::Tarf).label(), "tarf");
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

    /// O log carrega a identidade do espaço vetorial que produziu aquelas distâncias, para que
    /// separar eras seja `grep` e não memória de quem arquivou a pasta na hora certa.
    #[test]
    fn o_log_carimba_a_identidade_do_espaco_vetorial() {
        let rec = format_log_record("2026-08-11 10:00:00", &registro_minimo());
        let id = auli_core::manifest::identity();

        assert!(
            rec.contains(&format!(
                "EMBED · modelo: {} · dim: {} · strategy: {}",
                id.embed_model_id, id.embed_dim, id.strategy_version
            )),
            "carimbo ausente ou fora do formato: {rec}"
        );
        // Contrato de grep, como o da linha TEMPOS: `grep -l "strategy: N" logs/*.txt` é o filtro
        // que substitui mover pasta.
        assert!(rec.contains(&format!("strategy: {}", id.strategy_version)));

        // O carimbo entra no CABEÇALHO, antes da régua — não no meio das seções.
        let i_embed = rec.find("EMBED ·").expect("linha EMBED");
        let i_regua = rec[i_embed..].find("====").map(|d| i_embed + d);
        let i_secao = rec.find("-----").expect("primeira seção");
        assert!(
            i_regua.is_some_and(|r| r < i_secao),
            "EMBED deve preceder a régua e as seções"
        );
    }

    /// Um registro mínimo, para os testes que olham só o cabeçalho.
    fn registro_minimo() -> RegistroConsulta<'static> {
        RegistroConsulta {
            entidade: "rs",
            tipo: "pareceres",
            original: "p",
            sanitizada: "p",
            answer: Some("r"),
            aderencia: "",
            rag: "",
            tempos: TemposConsulta {
                embed_ms: 1,
                retrieve_ms: 1,
                llm_ms: 1,
                total_ms: 3,
            },
        }
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
                rag: "## documento 1: Parecer\n...",
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
                rag: "\n## documento 1: Serviço\n...",
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
        // Os códigos são contrato com o frontend: `2` é pareceres desde a primeira versão da tab do
        // auditor e não pode ser reordenado; `3` é o TARF.
        assert_eq!(
            QueryType::from_code(Some(2)),
            QueryType::Jurisprudencia(Kind::Pareceres)
        );
        assert_eq!(
            QueryType::from_code(Some(3)),
            QueryType::Jurisprudencia(Kind::Tarf)
        );
        assert_eq!(QueryType::from_code(Some(1)), QueryType::ServicosFaqs);
        assert_eq!(QueryType::from_code(None), QueryType::ServicosFaqs);
        // Unknown code degrades to the default rather than erroring.
        assert_eq!(QueryType::from_code(Some(9)), QueryType::ServicosFaqs);
    }

    /// A trava que impede o `expect` do caminho de jurisprudência de virar pânico em produção: todo
    /// `Kind` tem que resolver em `corpus::from_kind`.
    #[test]
    fn todo_kind_resolve_em_corpus() {
        for k in Kind::TODOS {
            let c = auli_core::corpus::from_kind(k.as_str())
                .unwrap_or_else(|e| panic!("{k} não resolve em corpus: {e}"));
            assert_eq!(c.kind, k.as_str());
        }
    }

    // ---- Trava de paridade do formato do contexto RAG (item 8 do G2) ----
    //
    // Estes testes pinam BYTE A BYTE a string que vai ao prompt do LLM e ao log de auditoria.
    // São a razão de `montar_rag_*` existir: o G2 reescreve o caminho vivo do chat e, sem eles, a
    // única verificação de paridade seria o diff manual de log. Se um destes asserts falhar num
    // refactor futuro, o contexto do RAG mudou — o que muda a resposta do modelo.
    //
    // Os literais mudaram em 2026-08, deliberadamente (TAREFA-MARCADORES, PR #129): os quatro invólucros
    // viraram `## documento {i}: {rótulo}`. As travas impedem mudança ACIDENTAL, não decidida — por
    // isso foram reescritas no mesmo commit do código, à mão, e não regeradas pela função sob teste.

    #[test]
    fn envolver_pina_o_involucro_unico() {
        // A unidade do formato: quebra de linha, marcador, índice, dois-pontos, rótulo, conteúdo,
        // quebra final. Tudo o mais é composição disto.
        assert_eq!(
            envolver("Serviço", 1, "CONTEUDO"),
            "\n## documento 1: Serviço\nCONTEUDO\n"
        );
        assert_eq!(
            envolver("Parecer relacionado", 12, "X"),
            "\n## documento 12: Parecer relacionado\nX\n"
        );
    }

    #[test]
    fn montar_rag_servicos_faqs_pina_o_formato() {
        let svc = vec!["SERVICO A".to_string(), "SERVICO B".to_string()];
        let faq = vec!["FAQ X".to_string()];
        assert_eq!(
            montar_rag_servicos_faqs(&svc, &faq),
            "\n## documento 1: Serviço\nSERVICO A\n\n## documento 2: Serviço\nSERVICO B\n\n\n## documento 1: FAQ\nFAQ X\n"
        );
    }

    #[test]
    fn montar_rag_servicos_faqs_com_listas_vazias() {
        // Caso real: entidade sem faqs. O separador `\n` entre os dois blocos permanece — é o que
        // o formato sempre produziu, e mudá-lo mudaria o prompt.
        assert_eq!(montar_rag_servicos_faqs(&[], &[]), "\n");
        assert_eq!(
            montar_rag_servicos_faqs(&["SO SERVICO".to_string()], &[]),
            "\n## documento 1: Serviço\nSO SERVICO\n\n"
        );
    }

    #[test]
    fn a_numeracao_e_por_colecao_e_nao_corrida() {
        // D-MARC-3: as FAQs recomeçam em 1 depois dos serviços. É o que mantém a correspondência
        // 1:1 com a seção ADERÊNCIA do log, que rotula por coleção+índice.
        let rag = montar_rag_servicos_faqs(&["S1".into(), "S2".into()], &["F1".into()]);
        assert!(rag.contains("## documento 2: Serviço\nS2"));
        assert!(
            rag.contains("## documento 1: FAQ\nF1"),
            "faq recomeça em 1: {rag}"
        );
    }

    #[test]
    fn montar_rag_pareceres_pina_o_formato() {
        let blocos = vec!["BLOCO UM".to_string(), "BLOCO DOIS".to_string()];
        assert_eq!(
            montar_rag_jurisprudencia(Kind::Pareceres, &blocos, &[]),
            "\n## documento 1: Parecer\nBLOCO UM\n\n## documento 2: Parecer\nBLOCO DOIS\n"
        );
        assert_eq!(montar_rag_jurisprudencia(Kind::Pareceres, &[], &[]), "");
    }

    /// O bloco do TARF: MESMO template, só o rótulo muda. É o que a D-MARC-1 promete — o que
    /// distingue as coleções é dado, não estrutura.
    #[test]
    fn montar_rag_tarf_pina_o_formato() {
        let blocos = vec!["BLOCO UM".to_string()];
        assert_eq!(
            montar_rag_jurisprudencia(Kind::Tarf, &blocos, &[]),
            "\n## documento 1: Acórdão TARF\nBLOCO UM\n"
        );
    }

    #[test]
    fn montar_rag_pareceres_anexa_secao_relacionados() {
        // Com relacionados: rotulados à parte e numerados do 1, DEPOIS dos recuperados.
        let blocos = vec!["BLOCO UM".to_string()];
        let rel = vec!["BLOCO REL".to_string()];
        assert_eq!(
            montar_rag_jurisprudencia(Kind::Pareceres, &blocos, &rel),
            "\n## documento 1: Parecer\nBLOCO UM\n\n## documento 1: Parecer relacionado\nBLOCO REL\n"
        );
    }

    // ---- Gravação: identidade e unicidade do arquivo (D-LOG-1) ----

    /// `AULI_LOG_DIR` é global do processo e os testes rodam em paralelo: uma escrita só, garantida
    /// pelo `LazyLock`. Nenhum outro teste desta crate chama o `log_question`.
    static DIR_DE_TESTE: std::sync::LazyLock<std::path::PathBuf> = std::sync::LazyLock::new(|| {
        let dir = std::env::temp_dir().join(format!("auli-rag-log-{}", Uuid::now_v7()));
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("AULI_LOG_DIR", &dir) };
        dir
    });

    fn registro_de_teste() -> RegistroConsulta<'static> {
        RegistroConsulta {
            entidade: "rs",
            tipo: "pareceres",
            original: "pergunta",
            sanitizada: "pergunta",
            answer: Some("resposta"),
            aderencia: "",
            rag: "contexto",
            tempos: TemposConsulta::default(),
        }
    }

    /// O UUID devolvido é o que nomeia o arquivo — é o contrato inteiro do `log_id`: o que volta ao
    /// cliente tem de abrir aquele registro, e não outro.
    #[test]
    fn log_question_devolve_o_uuid_que_nomeia_o_arquivo() {
        let dir = &*DIR_DE_TESTE;
        let id = log_question(&registro_de_teste()).unwrap();

        let achado = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|n| n.ends_with(&format!("_{id}.txt")));

        assert!(achado.is_some(), "nenhum arquivo termina em _{id}.txt");
    }

    /// **A regressão que a D-LOG-1 existe para impedir.** Antes o nome era só o timestamp em
    /// SEGUNDOS e o arquivo abria em `append`: duas consultas no mesmo segundo viravam UM arquivo
    /// com dois registros. Com a rota `GET /v1/log/{uuid}` isso serviria a pergunta — e a PII — de
    /// outra pessoa a quem pedisse o próprio log.
    ///
    /// Duas gravações em sequência imediata caem no mesmo segundo (e quase sempre no mesmo
    /// milissegundo), que é justamente o caso que colidia.
    #[test]
    fn duas_gravacoes_seguidas_nao_se_misturam_num_arquivo() {
        let dir = &*DIR_DE_TESTE;
        let a = log_question(&registro_de_teste()).unwrap();
        let b = log_question(&registro_de_teste()).unwrap();

        assert_ne!(a, b, "dois registros não podem compartilhar identidade");

        for id in [a, b] {
            let arquivo = std::fs::read_dir(dir)
                .unwrap()
                .flatten()
                .map(|e| e.path())
                .find(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.ends_with(&format!("_{id}.txt")))
                })
                .unwrap_or_else(|| panic!("arquivo de {id} não existe"));

            let conteudo = std::fs::read_to_string(&arquivo).unwrap();
            assert_eq!(
                conteudo.matches("PERGUNTA (ORIGINAL)").count(),
                1,
                "{arquivo:?} tem mais de um registro — a colisão voltou"
            );
        }
    }
}
