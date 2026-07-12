// RAG orchestration: embed the question, retrieve from the entity's servicos + faqs
// collections (scored), narrow each set by proximity, assemble the prompt, call the LLM,
// and log the exchange. The server embeds ONLY the question here; documents were embedded
// ahead of time by `auli update`. Retrieval reads immutable `ReadStore`s — no writes, no locks.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Arc;

use auli_core::corpus::{FAQS, PARECERES, SERVICES};
use auli_core::embed::Embedder;
use tracing::{debug, info, trace, warn};
use vector_store::ReadStore;

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

pub async fn exec_all_question(
    collections: Arc<Collections>,
    embedder: Arc<Embedder>,
    question: String,
    entity: Option<String>,
    query_type: QueryType,
) -> Result<String> {
    debug!("Executando consulta: {}", question);

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
            render(&par_docs, |i, doc| format!("\n## parecer\n{i}\n{doc}\n"))
        }
    };

    // System prompt = entity prompt + RAG context, closed with the original delimiter.
    let system_prompt = format!("{}{}'''", cfg.system_prompt, rag);
    trace!("System instructions with RAG: {}", system_prompt);

    let answer = llm::chat(&system_prompt, &question).await?;

    info!("Resposta: {}", answer);

    log_question(format!("Pergunta: {}\n{}\nResposta:\n{}", question, rag, answer))?;

    Ok(answer)
}

fn log_question(content: String) -> std::io::Result<()> {
    // Diretório de logs configurável; default `./logs` (relativo ao CWD). O start_server.sh aponta
    // para a raiz do repo (`$ROOT/logs`) para não depender de onde o binário é lançado.
    let log_dir = std::env::var("AULI_LOG_DIR").unwrap_or_else(|_| "./logs".to_string());
    fs::create_dir_all(&log_dir)?;
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let path = format!("{}/{}.txt", log_dir, timestamp);
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    debug!("Log da consulta gravado em {}", path);
    writeln!(file, "{}", content)
}

#[cfg(test)]
mod tests {
    use super::{select_by_proximity, QueryType};

    fn scored(pairs: &[(&str, f32)]) -> Vec<(String, f32)> {
        pairs.iter().map(|(d, s)| (d.to_string(), *s)).collect()
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
