//! `auli update` — the vectorizer / pack builder (the only writer).
//!
//! For each content kind: read `<source>/<file>`, `parse_blocks` → `prepare_documents` →
//! `embed_dense` (all via `auli-core`, the SAME code the server uses on the query), assign
//! sequential `id-1..id-N`, and `Writer::reset` + `upsert` into `<out>/<entity>-<kind>.json`.
//! Finally write `<out>/<entity>.manifest.json` stamping the embedding identity, so the server
//! can validate it at boot.
//!
//! Does NOT use the server `Config` (no LLM/JWT/DB needed for ingestion) — only the embedder
//! settings, read directly from the environment with defaults.

use std::path::PathBuf;

use auli_core::corpus;
use auli_core::embed::{Embedder, EMBED_DIM};
use auli_core::manifest::{self, CollectionEntry, Manifest};
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

    for collection in corpus::ALL {
        let src = source.join(collection.file);
        let name = format!("{}-{}", entity, collection.kind);

        if !src.exists() {
            println!("⏭️  {} ausente ({:?}) — pulando", collection.kind, src);
            continue;
        }

        let path_str = src.to_str().ok_or("caminho de origem inválido (não-UTF8)")?;
        let blocks = corpus::parse_blocks(path_str, collection.delimiter)?;
        let (stored, to_embed) = corpus::prepare_documents(&blocks, collection);
        println!("🔢 {}: {} blocos → vetorizando...", collection.kind, stored.len());

        let embeddings = embedder.embed_dense(to_embed)?;
        let ids: Vec<String> = (1..=stored.len()).map(|i| format!("id-{}", i)).collect();

        writer.reset::<String>(&name)?; // clean reload: no orphan id-(N+1)..
        let total = writer.upsert(&name, &ids, embeddings, &stored)?;

        // Stamp this collection in the manifest (count + integrity hash of the written file).
        let file_name = format!("{}.json", name);
        let written = std::fs::read(out.join(&file_name))?;
        entries.push(CollectionEntry {
            kind: collection.kind.to_string(),
            count: total as usize,
            dim: EMBED_DIM,
            file: file_name,
            bytes: written.len() as u64,
            hash: manifest::hash_hex(&written),
        });
        println!("✅ {} → {} registros", name, total);
    }

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
