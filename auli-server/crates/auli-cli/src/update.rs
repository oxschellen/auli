//! `auli update` — the vectorizer / pack builder (the only writer).
//!
//! For each content kind it reads the scraper's contract table `<source>/<entity>-<table>.json`
//! (an `auli_contract::Table<P>`), embeds each record's `text_to_embed()` and stores its
//! `stored_repr()` — via `auli-core::embed`, the SAME encoder the server uses on the query —
//! assigns sequential `id-1..id-N`, and `Writer::reset` + `upsert` into
//! `<out>/<entity>-<kind>.json`. Finally writes `<out>/<entity>.manifest.json` stamping the
//! embedding identity, so the server can validate it at boot.
//!
//! `servicos` is one vocabulary end-to-end: the source contract is `<entity>-servicos.json` and the
//! pack/kind is `<entity>-servicos` too. `pareceres`/`notas` have no
//! struct source yet (authored, not scraped) and are simply absent until modeled as a contract.
//!
//! Does NOT use the server `Config` (no LLM vars needed for ingestion) — only the embedder
//! settings, read directly from the environment with defaults.

use std::path::{Path, PathBuf};

use auli_contract::{Embeddable, Table};
use auli_core::embed::{Embedder, EMBED_DIM};
use auli_core::manifest::{self, CollectionEntry, Manifest};
use serde::de::DeserializeOwned;
use vector_store::Writer;

use crate::error::Result;

pub fn run_update(entity: String, source: PathBuf, out: PathBuf, version: Option<String>) -> Result<()> {
    dotenvy::dotenv().ok();
    let cache_dir = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "./models".to_string());
    let threads: usize = std::env::var("EMBED_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(16);

    println!("🧠 Carregando embedder (BGE-M3) de '{}'...", cache_dir);
    let embedder = Embedder::new(cache_dir.into(), threads)?;
    let writer = Writer::new(&out);
    let mut entries: Vec<CollectionEntry> = Vec::new();

    // faqs: contract Table<Faq> in <source>/<entity>-faqs.json -> pack <entity>-faqs.
    if let Some(entry) = ingest::<auli_contract::Faq>(
        &embedder, &writer, &entity, "faqs", &format!("{}-faqs.json", entity), &source, &out,
    )? {
        entries.push(entry);
    }
    // servicos: <entity>-servicos.json (contract) -> pack <entity>-servicos.
    if let Some(entry) = ingest::<auli_contract::Servico>(
        &embedder, &writer, &entity, "servicos", &format!("{}-servicos.json", entity), &source, &out,
    )? {
        entries.push(entry);
    }
    // pareceres/notas: sem fonte struct por ora (autorados) — ausentes até serem modelados.

    let manifest = Manifest {
        entity: entity.clone(),
        version: version.unwrap_or_else(|| "1".to_string()),
        built_at: chrono::Utc::now().to_rfc3339(),
        embed_model_id: manifest::EMBED_MODEL_ID.to_string(),
        embed_dim: EMBED_DIM,
        strategy_version: manifest::STRATEGY_VERSION,
        collections: entries,
    };
    let mpath = manifest::manifest_path(&out, &entity);
    manifest::write_manifest(&mpath, &manifest)?;
    println!("📝 Manifesto escrito em {:?}", mpath);

    Ok(())
}

/// Ingest one contract table into the entity's `<entity>-<kind>` vector collection.
///
/// `source_file` is the scraper's contract JSON (e.g. `rs-servicos.json`); `kind` is the engine
/// vector kind (e.g. `servicos`). Embeds `P::text_to_embed`, stores `P::stored_repr`. Returns the
/// manifest entry, or `None` if the source is absent (a kind with no struct source yet — skipped).
fn ingest<P>(
    embedder: &Embedder,
    writer: &Writer,
    entity: &str,
    kind: &str,
    source_file: &str,
    source_dir: &Path,
    out: &Path,
) -> Result<Option<CollectionEntry>>
where
    P: Embeddable + DeserializeOwned,
{
    let src = source_dir.join(source_file);
    if !src.exists() {
        println!("⏭️  {} ausente ({:?}) — pulando", kind, src);
        return Ok(None);
    }

    let bytes = std::fs::read(&src)?;
    let table: Table<P> = serde_json::from_slice(&bytes)?;
    let to_embed: Vec<String> = table.items.iter().map(|it| it.text_to_embed().to_string()).collect();
    let stored: Vec<String> = table.items.iter().map(|it| it.stored_repr()).collect();
    println!("🔢 {}: {} registros → vetorizando...", kind, stored.len());

    let name = format!("{}-{}", entity, kind);
    let embeddings = embedder.embed_dense(to_embed)?;
    // The manifest stamps `dim = EMBED_DIM` (a constant). If the model ever produces a different
    // real width without an `EMBED_MODEL_ID` bump, the manifest would lie and boot validation
    // (which checks the identity triple, not the file width) wouldn't catch it. Fail loudly here.
    if let Some(first) = embeddings.first()
        && first.len() != EMBED_DIM
    {
        return Err(crate::error::Error::Custom(format!(
            "embedder produziu dim {} ≠ EMBED_DIM {} para '{}' — faça bump de EMBED_MODEL_ID e re-gere os packs",
            first.len(),
            EMBED_DIM,
            name
        )));
    }
    let ids: Vec<String> = (1..=stored.len()).map(|i| format!("id-{}", i)).collect();

    writer.reset::<String>(&name)?; // clean reload: no orphan id-(N+1)..
    let total = writer.upsert(&name, &ids, embeddings, &stored)?;

    // Stamp this collection in the manifest (count + integrity hash of the written file).
    let file_name = format!("{}.json", name);
    let written = std::fs::read(out.join(&file_name))?;
    println!("✅ {} → {} registros", name, total);
    Ok(Some(CollectionEntry {
        kind: kind.to_string(),
        count: total as usize,
        dim: EMBED_DIM,
        file: file_name,
        bytes: written.len() as u64,
        hash: manifest::hash_hex(&written),
    }))
}
