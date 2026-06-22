//! In-process embeddings via `fastembed` (BGE-M3, ONNX) — replaces the Ollama HTTP client.
//!
//! The model is held behind a `Mutex` because `Bgem3Embedding::embed` takes `&mut self`. All
//! methods are **blocking** and CPU-bound; async callers wrap each call in
//! `tokio::task::spawn_blocking`. Phase 1 uses only the dense output, preserving the old
//! `Vec<Vec<f32>>` contract.
//!
//! Embedding strategy (see the migration plan): we embed a short, high-signal *key* (the user
//! question / FAQ `## pergunta` / service description), so `max_length` is sized to the key, not
//! to a whole document.

use std::path::PathBuf;
use std::sync::Mutex;

use fastembed::{Bgem3Embedding, Bgem3InitOptions, Bgem3Model};

use crate::errors::{Error, Result};

/// BGE-M3 dense width. A convenience for callers/asserts — the vector store itself is
/// dimensionless and stores whatever width the embedder emits.
pub const EMBED_DIM: usize = 1024;

pub struct Embedder {
    inner: Mutex<Bgem3Embedding>,
}

impl Embedder {
    /// Build once at startup. Blocking and slow: loads (and on first run downloads from Hugging
    /// Face into `cache_dir`) the BGE-M3 INT8 ONNX model.
    pub fn new(cache_dir: PathBuf, threads: usize) -> Result<Self> {
        let opts = Bgem3InitOptions::new(Bgem3Model::BGEM3Q)
            .with_max_length(512) // keys are short — size to the key, not the document
            .with_intra_threads(threads)
            .with_cache_dir(cache_dir);
        let model = Bgem3Embedding::try_new(opts)?; // anyhow::Error -> crate::Error via #[from]
        Ok(Self { inner: Mutex::new(model) })
    }

    /// Dense vectors for a batch of texts (Phase 1). Blocking — call via `spawn_blocking`.
    pub fn embed_dense(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut model = self.inner.lock().map_err(|_| Error::from("embedder mutex poisoned"))?;
        let out = model.embed(&texts, None)?;
        Ok(out.dense)
    }
}
