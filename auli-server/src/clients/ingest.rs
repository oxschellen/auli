//! Ingest orchestration for any content kind.
//!
//! Parses are already done by the caller; this glues the remaining steps: turn blocks into
//! `(stored document, embedding key)` pairs, assign the sequential `id-N` keys, embed the keys
//! in-process (fastembed, off the async worker thread), then write into the vector store.
//! Replaces the old `clients::chroma::load_collection`, keeping the same human-readable log
//! string return that `build_response` expects.

use std::sync::Arc;

use crate::clients::embedder::Embedder;
use crate::clients::vector_store::VectorStore;
use crate::domain::collections::{prepare_documents, Collection};
use crate::domain::entities::EntityConfig;
use crate::errors::Result;
use crate::util::run_blocking;

pub async fn load_collection(
    vector: Arc<VectorStore>,
    embedder: Arc<Embedder>,
    entity: &EntityConfig,
    collection: &Collection,
    blocks: Vec<String>,
) -> Result<String> {
    let collection_name = entity.collection(collection.kind);
    println!(
        "Carregando '{}' ({} blocos) para a coleção {}",
        collection.kind,
        blocks.len(),
        collection_name
    );

    // 1 - Prepare stored documents and the texts to embed.
    let (stored_docs, texts_to_embed) = prepare_documents(&blocks, collection);

    // 2 - Assign sequential ids and build a human-readable log.
    let mut output_str = String::new();
    let mut ids: Vec<String> = Vec::with_capacity(stored_docs.len());
    for (i, doc) in stored_docs.iter().enumerate() {
        let id = i + 1;
        output_str += "--------------Load Embeddings--------------\n";
        output_str += &format!("Registro: {}\n{}\n", id, doc);
        ids.push(format!("id-{}", id));
    }
    println!("Total de registros inputados: {}", stored_docs.len());

    // 3 - Vectorize the embedding keys (in-process, off the async worker thread).
    let e = embedder.clone();
    let embeddings = run_blocking(move || e.embed_dense(texts_to_embed)).await?;
    println!("Numero de registros vetorizados: {}", embeddings.len());

    // 4 - Clean reload + upsert (blocking; runs on a blocking worker thread).
    let name = collection_name.clone();
    let total = run_blocking(move || {
        vector.reset(&name)?;
        vector.upsert(&name, &ids, embeddings, &stored_docs)
    })
    .await?;

    println!("Total de registros na coleção {}: {}", collection_name, total);
    Ok(output_str)
}
