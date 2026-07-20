// RAG orchestration: embed the question, retrieve from the entity's servicos + faqs
// collections (scored), narrow each set by proximity, assemble the prompt, call the LLM,
// and log the exchange. The server embeds ONLY the question here; documents were embedded
// ahead of time by `auli update`. Retrieval reads immutable `ReadStore`s — no writes, no locks.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use auli_anon::{Anonimizador, TEXTO_FALLBACK_ERRO};
use auli_contract::{ConsultaPackPayload, render_consulta_block};
use auli_core::corpus::{FAQS, PARECERES, SERVICES};
use auli_retrieval::Engine;
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
const SVC_FLOOR: usize = 0;
const SVC_BAND: f32 = f32::INFINITY;
const FAQ_FLOOR: usize = 0;
const FAQ_BAND: f32 = f32::INFINITY;
const PAR_FLOOR: usize = 0;
const PAR_BAND: f32 = f32::INFINITY;

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
async fn retrieve(
    engine: Arc<Engine>,
    collection: String,
    label: &'static str,
    embedding: Vec<f32>,
    n_results: usize,
    floor: usize,
    band: f32,
) -> Result<Vec<String>> {
    run_blocking(move || {
        match engine.search_embedded(&collection, &embedding, n_results, floor, band) {
            Ok(hits) => Ok(hits.into_iter().map(|h| h.payload).collect()),
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
    docs.iter().enumerate().map(|(i, doc)| fmt(i + 1, doc)).collect()
}

// Montagem do contexto RAG — funções PURAS (sem I/O, sem Engine), extraídas do
// `exec_all_question` no G2 para que a paridade do formato tenha trava automatizada. Recebem
// documentos JÁ PRONTOS para renderizar: para pareceres, os blocos já montados por
// `bloco_parecer` (que é quem lê a árvore `docs/`). Ver o item 8 do G2 na TAREFA-MCP.

/// Contexto do tipo `ServicosFaqs`: serviços numerados + FAQs numeradas, nesta ordem.
fn montar_rag_servicos_faqs(svc_docs: &[String], faq_docs: &[String]) -> String {
    let rag_service = render(svc_docs, |i, doc| format!("\n## servico\n{i}\n{doc}\n"));
    let rag_faq = render(faq_docs, |i, doc| format!("\n// Resultado: {i}\n{doc}\n"));
    format!("{}\n{}", rag_service, rag_faq)
}

/// Contexto do tipo `Pareceres`: um bloco numerado por parecer.
fn montar_rag_pareceres(blocos: &[String]) -> String {
    render(blocos, |i, bloco| format!("\n## PARECER\n{i}\n{bloco}\n"))
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
            error!("payload de parecer não desserializa ({e}) — pack incompatível passou pelo boot?");
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
    let embedding = {
        let e = engine.clone();
        let q = question.clone();
        run_blocking(move || e.embed(&q).map_err(|err| err.to_string().into())).await?
    };

    // Assemble the RAG context for the requested query type. The system-prompt/LLM/log tail below is
    // shared across types (the entity prompt is reused for every type).
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
            let (svc_docs, faq_docs) = tokio::try_join!(svc_fut, faq_fut)?;
            info!("Foram selecionados {} serviços e {} faqs", svc_docs.len(), faq_docs.len());

            montar_rag_servicos_faqs(&svc_docs, &faq_docs)
        }
        QueryType::Pareceres => {
            let par_docs = retrieve(
                engine.clone(),
                cfg.collection(PARECERES.kind),
                "par",
                embedding,
                PARECERES.n_results,
                PAR_FLOOR,
                PAR_BAND,
            )
            .await?;
            info!("Foram selecionados {} pareceres", par_docs.len());

            // No pareceres vectorized for this entity yet — answer with a friendly notice instead of
            // prompting the LLM on empty context (which would invite a hallucinated answer).
            if par_docs.is_empty() {
                return Ok("A consulta de Pareceres ainda não está disponível para esta entidade.".to_string());
            }
            // G3: cada doc recuperado é o payload LEVE (JSON, sem corpo). Remonta o bloco de sempre
            // lendo o corpo da árvore `docs/` da entidade (`<docs_root>/<id>/<doc_path>`).
            let blocos: Vec<String> = par_docs
                .iter()
                .map(|payload_json| bloco_parecer(payload_json, engine.docs_root(), &cfg.id))
                .collect();
            montar_rag_pareceres(&blocos)
        }
    };

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
    let entrada_llm = if anonimizar { &pergunta_anon } else { &question };
    let resposta_llm = llm::chat(&system_prompt, entrada_llm).await?;

    // Restaura os placeholders (`[CNPJ_1]` → valor original) antes de devolver — o usuário vê o valor
    // real, mas o LLM só viu o placeholder. Sem mapping (anonimização falhou) ou flag off: sem troca.
    let answer = match (anonimizar, &mapping) {
        (true, Some(m)) => anonimizador.restaurar(&resposta_llm, m),
        _ => resposta_llm,
    };

    // Resposta em `debug!` (não `info!`): evita despejar a resposta crua no stdout por padrão.
    debug!("Resposta: {}", answer);

    log_question(&cfg.id, query_type.label(), &question, &pergunta_anon, &answer, &rag)?;

    Ok(answer)
}

/// Monta o registro estruturado do log de auditoria: cabeçalho + seções rotuladas
/// (pergunta original, pergunta anonimizada, resposta e, por fim, o contexto RAG).
/// Pura (sem I/O) para ser testável.
fn format_log_record(
    stamp: &str,
    entidade: &str,
    tipo: &str,
    original: &str,
    sanitizada: &str,
    answer: &str,
    rag: &str,
) -> String {
    let regua = "=".repeat(64);
    let secao = |titulo: &str| -> String {
        let base = format!("----- {titulo} ");
        let faltam = 64usize.saturating_sub(base.chars().count());
        format!("{base}{}", "-".repeat(faltam))
    };
    format!(
        "{regua}\n\
         CONSULTA · {stamp} · entidade: {entidade} · tipo: {tipo}\n\
         {regua}\n\n\
         {}\n{original}\n\n\
         {}\n{sanitizada}\n\n\
         {}\n{answer}\n\n\
         {}\n{rag}\n\
         {regua}",
        secao("PERGUNTA (ORIGINAL)"),
        secao("PERGUNTA (ANONIMIZADA)"),
        secao("RESPOSTA"),
        secao("CONTEXTO RAG (documentos recuperados)"),
    )
}

fn log_question(
    entidade: &str,
    tipo: &str,
    original: &str,
    sanitizada: &str,
    answer: &str,
    rag: &str,
) -> std::io::Result<()> {
    // Diretório de logs configurável; default `./logs` (relativo ao CWD). O start_server.sh aponta
    // para a raiz do repo (`$ROOT/logs`) para não depender de onde o binário é lançado.
    let log_dir = std::env::var("AULI_LOG_DIR").unwrap_or_else(|_| "./logs".to_string());
    fs::create_dir_all(&log_dir)?;
    let agora = chrono::Local::now();
    let path = format!("{}/{}.txt", log_dir, agora.format("%Y-%m-%d_%H-%M-%S"));
    let stamp = agora.format("%Y-%m-%d %H:%M:%S").to_string();
    let content = format_log_record(&stamp, entidade, tipo, original, sanitizada, answer, rag);
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    debug!("Log da consulta gravado em {}", path);
    writeln!(file, "{}", content)
}

#[cfg(test)]
mod tests {
    use super::{
        bloco_parecer, format_log_record, montar_rag_pareceres, montar_rag_servicos_faqs, QueryType,
    };
    use auli_contract::{mddoc, ConsultaPackPayload};
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
        std::fs::write(pdir.join("parecer-no-1.md"), mddoc::render_doc(&header, None, "É o corpo integral.")).unwrap();

        let bloco = bloco_parecer(&payload("docs/pareceres/parecer-no-1.md"), &dir, "sc");
        // Bloco de sempre, com o corpo lido da árvore.
        assert_eq!(bloco, "## pergunta\nPARECER Nº 1\nICMS – crédito\n\n## resposta\nÉ o corpo integral.\nLink: http://x/1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bloco_parecer_degrada_para_o_resumo_quando_o_arquivo_falta() {
        let dir = std::env::temp_dir().join(format!("auli-rag-g3-miss-{}", std::process::id()));
        // Nada materializado: o doc_path aponta para arquivo inexistente.
        let bloco = bloco_parecer(&payload("docs/pareceres/parecer-no-1.md"), &dir, "sc");
        assert!(bloco.contains("[corpo indisponível — ver link]"), "bloco: {bloco}");
        assert!(bloco.contains("Resumo do parecer."), "usa o resumo no lugar do corpo");
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
    fn log_record_has_header_and_sections_in_order() {
        let rec = format_log_record(
            "2026-07-16 14:23:05",
            "rs",
            "pareceres",
            "CNPJ 11.222.333/0001-81 pode aderir?",
            "CNPJ [CNPJ_1] pode aderir?",
            "Sim, o CNPJ 11.222.333/0001-81 atende.",
            "## PARECER\n0\n...",
        );

        // Cabeçalho com metadados (data, entidade, tipo) — sem IP.
        assert!(rec.contains("CONSULTA · 2026-07-16 14:23:05 · entidade: rs · tipo: pareceres"));

        // As quatro seções, na ordem: original → anonimizada → resposta → contexto RAG.
        let i_orig = rec.find("PERGUNTA (ORIGINAL)").expect("seção original");
        let i_anon = rec.find("PERGUNTA (ANONIMIZADA)").expect("seção anonimizada");
        let i_resp = rec.find("RESPOSTA").expect("seção resposta");
        let i_rag = rec.find("CONTEXTO RAG").expect("seção rag");
        assert!(i_orig < i_anon && i_anon < i_resp && i_resp < i_rag);

        // A original mantém o PII; a anonimizada tem o placeholder; a resposta fica como veio.
        assert!(rec.contains("CNPJ 11.222.333/0001-81 pode aderir?"));
        assert!(rec.contains("CNPJ [CNPJ_1] pode aderir?"));
        assert!(rec.contains("Sim, o CNPJ 11.222.333/0001-81 atende."));
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
        let blocos = vec!["BLOCO UM".to_string(), "BLOCO DOIS".to_string()];
        assert_eq!(
            montar_rag_pareceres(&blocos),
            "\n## PARECER\n1\nBLOCO UM\n\n## PARECER\n2\nBLOCO DOIS\n"
        );
        assert_eq!(montar_rag_pareceres(&[]), "");
    }
}
