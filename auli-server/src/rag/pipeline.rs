// RAG orchestration: embed the question, retrieve from the entity's services + faqs
// collections (scored), narrow each set by proximity, assemble the prompt, call the LLM,
// and log the exchange.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Arc;

use crate::clients::embedder::Embedder;
use crate::clients::llm;
use crate::clients::vector_store::VectorStore;
use crate::domain::collections::{FAQS, SERVICES};
use crate::domain::entities::get_entity;
use crate::errors::Result;
use crate::util::run_blocking;
use tracing::{debug, info, trace, warn};

// Per-kind adaptive selection. `score` is a cosine DISTANCE — lower is closer. `floor` is the
// always-keep count; `band` is the max distance ABOVE the best match still admitted.
//
// Defaults preserve parity with the old fixed-take behavior: `band = ∞` keeps every retrieved
// doc up to the ceiling (`Collection::n_results`). To enable adaptive narrowing, run real
// questions, read the per-kind score arrays printed below, and lower each band to just above
// where genuine matches separate from filler. Services (description embedding) and faqs
// (full-text embedding) distribute differently — tune the two pairs independently.
const SVC_FLOOR: usize = 0;
const SVC_BAND: f32 = f32::INFINITY;
const FAQ_FLOOR: usize = 0;
const FAQ_BAND: f32 = f32::INFINITY;

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

/// Retrieve + narrow one collection: query the store for up to `n_results` scored docs (on a
/// blocking worker thread), then keep those within `band` of the best (always the top `floor`).
/// One self-contained async unit per kind, so services and faqs can run concurrently.
async fn retrieve(
    vector: Arc<VectorStore>,
    label: &'static str,
    collection: String,
    embedding: Vec<f32>,
    n_results: usize,
    floor: usize,
    band: f32,
) -> Result<Vec<String>> {
    let scored = run_blocking(move || vector.query_scored(&collection, embedding, n_results)).await?;
    // Score array — calibrate the per-kind band against real questions.
    debug!("{label} scores: {:?}", scored.iter().map(|(_, s)| *s).collect::<Vec<_>>());
    Ok(select_by_proximity(scored, floor, band))
}

/// Render retrieved docs into the RAG context block, one entry per doc (1-based index).
fn render(docs: &[String], fmt: impl Fn(usize, &str) -> String) -> String {
    docs.iter().enumerate().map(|(i, doc)| fmt(i + 1, doc)).collect()
}

pub async fn exec_all_question(
    vector: Arc<VectorStore>,
    embedder: Arc<Embedder>,
    question: String,
    entity: Option<String>,
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

    // Embed the question once (off the async worker thread), reuse for both retrievals. The
    // question is itself a short "key", so it embeds directly.
    let embedding = {
        let e = embedder.clone();
        let q = vec![question.clone()];
        run_blocking(move || e.embed_dense(q))
            .await?
            .into_iter()
            .next()
            .ok_or("Não foi possível gerar embedding para a pergunta.")?
    };

    // Retrieve services + faqs as two independent, concurrent async calls. Each runs its
    // (blocking) store query on a blocking worker thread and narrows by proximity; `try_join!`
    // drives both at once and short-circuits on the first error.
    let svc_fut = retrieve(
        vector.clone(),
        "svc",
        cfg.collection(SERVICES.kind),
        embedding.clone(),
        SERVICES.n_results,
        SVC_FLOOR,
        SVC_BAND,
    );
    let faq_fut = retrieve(
        vector.clone(),
        "faq",
        cfg.collection(FAQS.kind),
        embedding,
        FAQS.n_results,
        FAQ_FLOOR,
        FAQ_BAND,
    );
    let (svc_docs, faq_docs) = tokio::try_join!(svc_fut, faq_fut)?;
    info!("Foram selecionados {} serviços e {} faqs", svc_docs.len(), faq_docs.len());

    // Assemble RAG context (formatting preserved from the original pipeline).
    let rag_service = render(&svc_docs, |i, doc| format!("\n## servico\n{i}\n{doc}\n"));
    let rag_faq = render(&faq_docs, |i, doc| format!("\n// Resultado: {i}\n{doc}\n"));
    let rag = format!("{}\n{}", rag_service, rag_faq);

    // System prompt = entity prompt + RAG context, closed with the original delimiter.
    let system_prompt = format!("{}{}'''", cfg.system_prompt, rag);
    trace!("System instructions with RAG: {}", system_prompt);

    let answer = llm::chat(&system_prompt, &question).await?;

    info!("Resposta: {}", answer);

    log_question(format!("Pergunta: {}\n{}\nResposta:\n{}", question, rag, answer))?;

    Ok(answer)
}

fn log_question(content: String) -> std::io::Result<()> {
    fs::create_dir_all("./logs")?;
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let path = format!("./logs/{}.txt", timestamp);
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    debug!("Log da consulta gravado em {}", path);
    writeln!(file, "{}", content)
}

#[cfg(test)]
mod tests {
    use super::select_by_proximity;

    fn scored(pairs: &[(&str, f32)]) -> Vec<(String, f32)> {
        pairs.iter().map(|(d, s)| (d.to_string(), *s)).collect()
    }

    #[test]
    fn empty_input_yields_nothing() {
        assert!(select_by_proximity(vec![], 0, f32::INFINITY).is_empty());
    }

    #[test]
    fn default_band_keeps_everything() {
        // floor=0, band=∞ is the live default: parity with the old fixed take — nothing dropped.
        let docs = scored(&[("a", 0.0), ("b", 0.3), ("c", 1.7)]);
        assert_eq!(select_by_proximity(docs, 0, f32::INFINITY), vec!["a", "b", "c"]);
    }

    #[test]
    fn finite_band_narrows_to_proximity_of_best() {
        // band is measured from the global best (0.10), not the floor boundary.
        let docs = scored(&[("a", 0.10), ("b", 0.12), ("c", 0.90)]);
        assert_eq!(select_by_proximity(docs, 0, 0.05), vec!["a", "b"]);
    }

    #[test]
    fn floor_overrides_band() {
        // The top `floor` docs are kept unconditionally even when outside the band.
        let docs = scored(&[("a", 0.10), ("b", 0.12), ("c", 0.90)]);
        assert_eq!(select_by_proximity(docs.clone(), 3, 0.05), vec!["a", "b", "c"]);
        // ...and the floor never invents docs that aren't there.
        assert_eq!(select_by_proximity(docs, 10, 0.05).len(), 3);
    }

    #[test]
    fn band_zero_keeps_only_ties_with_best() {
        let docs = scored(&[("a", 0.20), ("b", 0.20), ("c", 0.21)]);
        assert_eq!(select_by_proximity(docs, 0, 0.0), vec!["a", "b"]);
    }
}
