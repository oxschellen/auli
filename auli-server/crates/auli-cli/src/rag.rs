// RAG orchestration: embed the question, retrieve from the entity's servicos + faqs
// collections (scored), narrow each set by proximity, assemble the prompt, call the LLM,
// and log the exchange. The server embeds ONLY the question here; documents were embedded
// ahead of time by `auli update`. Retrieval reads immutable `ReadStore`s — no writes, no locks.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use auli_anon::{Anonimizador, TEXTO_FALLBACK_ERRO};
use auli_contract::{ConsultaPackPayload, mddoc, render_consulta_block};
use auli_core::corpus::{FAQS, PARECERES, SERVICES};
use auli_core::embed::Embedder;
use tracing::{debug, error, info, trace, warn};
use vector_store::ReadStore;

use crate::config::config;
use crate::entities::get_entity;
use crate::error::Result;
use crate::llm;
use crate::packs::Collections;
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

/// Keep the top `floor` docs always; beyond that, keep docs within `band` of the best score.
/// Stops early once a doc falls outside the band, since everything after it is farther still.
///
/// CONTRACT: `scored` MUST be sorted by ascending score (best/closest first). The early `break`
/// relies on that monotonicity — on unsorted input it silently drops in-band docs. `query_scored`
/// guarantees the order; the `debug_assert!` catches any future caller that doesn't.
fn select_by_proximity(scored: Vec<(String, f32)>, floor: usize, band: f32) -> Vec<String> {
    debug_assert!(
        scored.windows(2).all(|w| w[0].1 <= w[1].1),
        "select_by_proximity requires input sorted by ascending score (best-first)"
    );
    let Some(&(_, best)) = scored.first() else {
        return vec![];
    };
    let mut out = Vec::new();
    for (i, (doc, score)) in scored.into_iter().enumerate() {
        if i < floor || (score - best) <= band {
            out.push(doc);
        } else {
            break;
        }
    }
    out
}

/// Retrieve + narrow one collection. The collection may be absent (an entity that doesn't carry
/// this kind) — then we contribute nothing. The (CPU-bound) scan runs on a blocking worker thread.
async fn retrieve(
    store: Option<Arc<ReadStore<String>>>,
    label: &'static str,
    embedding: Vec<f32>,
    n_results: usize,
    floor: usize,
    band: f32,
) -> Result<Vec<String>> {
    let Some(store) = store else {
        warn!("coleção '{label}' ausente para esta entidade — ignorando");
        return Ok(vec![]);
    };
    let scored = run_blocking(move || Ok(store.query_scored(&embedding, n_results))).await?;
    // Score array — calibrate the per-kind band against real questions.
    debug!("{label} scores: {:?}", scored.iter().map(|(_, s)| *s).collect::<Vec<_>>());
    Ok(select_by_proximity(scored, floor, band))
}

/// Render retrieved docs into the RAG context block, one entry per doc (1-based index).
fn render(docs: &[String], fmt: impl Fn(usize, &str) -> String) -> String {
    docs.iter().enumerate().map(|(i, doc)| fmt(i + 1, doc)).collect()
}

/// Remonta o bloco de um parecer a partir do payload leve gravado no pack (G3): lê o corpo da árvore
/// `docs/` e delega ao contrato para montar o MESMO bloco de sempre.
///
/// Degradação graciosa (D1 do plano): se o corpo não puder ser lido/parseado, `error!` no log (com o
/// `doc_path`) e o bloco sai com o `resumo` no lugar do corpo, precedido de `[corpo indisponível — ver
/// link]`. Payload que não desserializa (não deveria passar pelo boot) cai no mesmo caminho seguro.
/// Nunca derruba a query — o passo de rede do LLM domina, então a leitura síncrona dos k arquivos é
/// irrelevante na latência.
fn bloco_parecer(payload_json: &str, entity_dir: &Path) -> String {
    let payload: ConsultaPackPayload = match serde_json::from_str(payload_json) {
        Ok(p) => p,
        Err(e) => {
            error!("payload de parecer não desserializa ({e}) — pack incompatível passou pelo boot?");
            return payload_json.to_string();
        }
    };
    match ler_corpo(&entity_dir.join(&payload.doc_path)) {
        Ok(corpo) => render_consulta_block(&payload, &corpo),
        Err(e) => {
            error!(doc_path = %payload.doc_path, "corpo indisponível ({e}) — degradando para o resumo");
            let corpo = format!("[corpo indisponível — ver link]\n{}", payload.resumo);
            render_consulta_block(&payload, &corpo)
        }
    }
}

/// Lê o `.md` da árvore e extrai a seção `## corpo` via o parser do contrato. Erro como `String`
/// (só serve ao `error!` da degradação — `auli-cli` não depende de `anyhow`).
fn ler_corpo(caminho: &Path) -> std::result::Result<String, String> {
    let texto = std::fs::read_to_string(caminho).map_err(|e| e.to_string())?;
    let (_header, _sinopse, corpo) = mddoc::parse_doc(&texto).map_err(|e| e.to_string())?;
    Ok(corpo)
}

pub async fn exec_all_question(
    collections: Arc<Collections>,
    embedder: Arc<Embedder>,
    anonimizador: Arc<Anonimizador>,
    docs_root: Arc<Path>,
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
        let e = embedder.clone();
        let q = vec![question.clone()];
        run_blocking(move || e.embed_dense(q).map_err(Into::into))
            .await?
            .into_iter()
            .next()
            .ok_or("Não foi possível gerar embedding para a pergunta.")?
    };

    // Assemble the RAG context for the requested query type. The system-prompt/LLM/log tail below is
    // shared across types (the entity prompt is reused for every type).
    let rag = match query_type {
        QueryType::ServicosFaqs => {
            // Look up this entity's servicos + faqs stores and retrieve concurrently.
            let svc_store = collections.get(&cfg.collection(SERVICES.kind)).cloned();
            let faq_store = collections.get(&cfg.collection(FAQS.kind)).cloned();

            let svc_fut = retrieve(svc_store, "svc", embedding.clone(), SERVICES.n_results, SVC_FLOOR, SVC_BAND);
            let faq_fut = retrieve(faq_store, "faq", embedding, FAQS.n_results, FAQ_FLOOR, FAQ_BAND);
            let (svc_docs, faq_docs) = tokio::try_join!(svc_fut, faq_fut)?;
            info!("Foram selecionados {} serviços e {} faqs", svc_docs.len(), faq_docs.len());

            // Assemble RAG context (formatting preserved from the original pipeline).
            let rag_service = render(&svc_docs, |i, doc| format!("\n## servico\n{i}\n{doc}\n"));
            let rag_faq = render(&faq_docs, |i, doc| format!("\n// Resultado: {i}\n{doc}\n"));
            format!("{}\n{}", rag_service, rag_faq)
        }
        QueryType::Pareceres => {
            let par_store = collections.get(&cfg.collection(PARECERES.kind)).cloned();
            let par_docs =
                retrieve(par_store, "par", embedding, PARECERES.n_results, PAR_FLOOR, PAR_BAND).await?;
            info!("Foram selecionados {} pareceres", par_docs.len());

            // No pareceres vectorized for this entity yet — answer with a friendly notice instead of
            // prompting the LLM on empty context (which would invite a hallucinated answer).
            if par_docs.is_empty() {
                return Ok("A consulta de Pareceres ainda não está disponível para esta entidade.".to_string());
            }
            // G3: cada doc recuperado é o payload LEVE (JSON, sem corpo). Remonta o bloco de sempre
            // lendo o corpo da árvore `docs/` da entidade (`<docs_root>/<id>/<doc_path>`).
            let entity_dir = docs_root.join(&cfg.id);
            let blocos: Vec<String> =
                par_docs.iter().map(|payload_json| bloco_parecer(payload_json, &entity_dir)).collect();
            render(&blocos, |i, bloco| format!("\n## PARECER\n{i}\n{bloco}\n"))
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
    use super::{bloco_parecer, format_log_record, ler_corpo, select_by_proximity, QueryType};
    use auli_contract::{mddoc, ConsultaPackPayload};
    use std::path::Path;

    fn scored(pairs: &[(&str, f32)]) -> Vec<(String, f32)> {
        pairs.iter().map(|(d, s)| (d.to_string(), *s)).collect()
    }

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
        let dir = std::env::temp_dir().join(format!("auli-rag-g3-ok-{}", std::process::id()));
        let pdir = dir.join("docs/pareceres");
        std::fs::create_dir_all(&pdir).unwrap();
        let header = mddoc::DocHeader {
            numero: "PARECER Nº 1".into(),
            assunto: "ICMS – crédito".into(),
            link: "http://x/1".into(),
            sinopse_info: None,
        };
        std::fs::write(pdir.join("parecer-no-1.md"), mddoc::render_doc(&header, None, "É o corpo integral.")).unwrap();

        let bloco = bloco_parecer(&payload("docs/pareceres/parecer-no-1.md"), &dir);
        // Bloco de sempre, com o corpo lido da árvore.
        assert_eq!(bloco, "## pergunta\nPARECER Nº 1\nICMS – crédito\n\n## resposta\nÉ o corpo integral.\nLink: http://x/1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bloco_parecer_degrada_para_o_resumo_quando_o_arquivo_falta() {
        let dir = std::env::temp_dir().join(format!("auli-rag-g3-miss-{}", std::process::id()));
        // Nada materializado: o doc_path aponta para arquivo inexistente.
        let bloco = bloco_parecer(&payload("docs/pareceres/parecer-no-1.md"), &dir);
        assert!(bloco.contains("[corpo indisponível — ver link]"), "bloco: {bloco}");
        assert!(bloco.contains("Resumo do parecer."), "usa o resumo no lugar do corpo");
        assert!(bloco.contains("Link: http://x/1"), "preserva o link");
        // Nunca propaga erro — a query segue.
    }

    #[test]
    fn bloco_parecer_com_payload_invalido_nao_derruba() {
        // Pack incompatível que (hipoteticamente) passou pelo boot: não desserializa → serve cru.
        let bloco = bloco_parecer("isto não é json", Path::new("/inexistente"));
        assert_eq!(bloco, "isto não é json");
    }

    #[test]
    fn ler_corpo_falha_em_arquivo_inexistente() {
        assert!(ler_corpo(Path::new("/nao/existe/x.md")).is_err());
    }

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

    #[test]
    fn empty_input_yields_nothing() {
        assert!(select_by_proximity(vec![], 0, f32::INFINITY).is_empty());
    }

    #[test]
    fn default_band_keeps_everything() {
        let docs = scored(&[("a", 0.0), ("b", 0.3), ("c", 1.7)]);
        assert_eq!(select_by_proximity(docs, 0, f32::INFINITY), vec!["a", "b", "c"]);
    }

    #[test]
    fn finite_band_narrows_to_proximity_of_best() {
        let docs = scored(&[("a", 0.10), ("b", 0.12), ("c", 0.90)]);
        assert_eq!(select_by_proximity(docs, 0, 0.05), vec!["a", "b"]);
    }

    #[test]
    fn floor_overrides_band() {
        let docs = scored(&[("a", 0.10), ("b", 0.12), ("c", 0.90)]);
        assert_eq!(select_by_proximity(docs.clone(), 3, 0.05), vec!["a", "b", "c"]);
        assert_eq!(select_by_proximity(docs, 10, 0.05).len(), 3);
    }

    #[test]
    fn band_zero_keeps_only_ties_with_best() {
        let docs = scored(&[("a", 0.20), ("b", 0.20), ("c", 0.21)]);
        assert_eq!(select_by_proximity(docs, 0, 0.0), vec!["a", "b"]);
    }
}
