//! In-process, pure-Rust vector store — replaces the ChromaDB-over-HTTP backend.
//!
//! Each `<entity>-<kind>` collection is a flat list of `(id, embedding, document)` records
//! held in memory and persisted to `<base_path>/<name>.json`. Similarity search is an exact
//! brute-force cosine scan, which is more than fast enough for the small per-collection
//! corpora here (tens–hundreds of documents). No external service, no C++ toolchain, and no
//! fixed embedding dimension — vectors are compared at whatever width the embedder emits.
//!
//! The upstream contract matches the old `clients::chroma` surface so nothing above the store
//! had to change shape: documents in as blocks, document texts (now with scores) out.
//!
//! All methods are **synchronous** (they block on file I/O and CPU); async callers wrap each
//! call in `tokio::task::spawn_blocking`. The collection registry behind an `RwLock` is held
//! only briefly for lookup/insert, never across the scan itself.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::errors::Result;

#[derive(Clone, Serialize, Deserialize)]
struct Record {
    id: String,
    embedding: Vec<f32>,
    document: String,
}

#[derive(Default, Serialize, Deserialize)]
struct CollectionData {
    records: Vec<Record>,
}

pub struct VectorStore {
    base_path: String,
    collections: RwLock<HashMap<String, Arc<RwLock<CollectionData>>>>,
}

impl VectorStore {
    pub fn new(base_path: impl Into<String>) -> Self {
        Self {
            base_path: base_path.into(),
            collections: RwLock::new(HashMap::new()),
        }
    }

    /// On-disk path for a `<entity>-<kind>` collection name.
    fn path_for(&self, name: &str) -> PathBuf {
        PathBuf::from(&self.base_path).join(format!("{name}.json"))
    }

    /// Get the in-memory collection, loading it from disk (or creating an empty one) on first
    /// use. Subsequent calls hit the registry fast path. Blocking.
    fn get_or_open(&self, name: &str) -> Result<Arc<RwLock<CollectionData>>> {
        if let Some(c) = self.collections.read().unwrap().get(name).cloned() {
            return Ok(c);
        }
        let data = self.load_from_disk(name)?;
        let arc = Arc::new(RwLock::new(data));
        self.collections.write().unwrap().insert(name.to_string(), arc.clone());
        Ok(arc)
    }

    fn load_from_disk(&self, name: &str) -> Result<CollectionData> {
        match fs::read(self.path_for(name)) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(CollectionData::default()),
            Err(e) => Err(e.into()),
        }
    }

    fn persist(&self, name: &str, data: &CollectionData) -> Result<()> {
        fs::create_dir_all(&self.base_path)?;
        fs::write(self.path_for(name), serde_json::to_vec(data)?)?;
        Ok(())
    }

    /// Drop every record in a collection (in memory and on disk). Used for clean full reloads
    /// so re-ingesting fewer blocks than before leaves no orphan `id-(N+1)..` records.
    pub fn reset(&self, name: &str) -> Result<()> {
        let arc = self.get_or_open(name)?;
        arc.write().unwrap().records.clear();
        self.persist(name, &CollectionData::default())
    }

    /// Upsert `(id, embedding, text)` triples (replace by id, else append) and persist.
    /// `ids` are the sequential `id-N` keys built by the caller, preserving today's scheme.
    /// Returns the total record count after the write. Blocking.
    pub fn upsert(&self, name: &str, ids: &[String], embeddings: Vec<Vec<f32>>, texts: &[String]) -> Result<u64> {
        let arc = self.get_or_open(name)?;
        let mut data = arc.write().unwrap();

        for ((id, emb), text) in ids.iter().zip(embeddings.into_iter()).zip(texts.iter()) {
            let rec = Record {
                id: id.clone(),
                embedding: emb,
                document: text.clone(),
            };
            match data.records.iter_mut().find(|r| r.id == rec.id) {
                Some(existing) => *existing = rec,
                None => data.records.push(rec),
            }
        }

        self.persist(name, &data)?;
        Ok(data.records.len() as u64)
    }

    /// Vector similarity search returning `(text, score)` pairs sorted best-first.
    /// `score` is a cosine DISTANCE — lower is closer (0.0 == identical direction). The
    /// `max_results` ceiling caps the return; the pipeline narrows further by proximity.
    /// Blocking.
    pub fn query_scored(&self, name: &str, embedding: Vec<f32>, max_results: usize) -> Result<Vec<(String, f32)>> {
        let arc = self.get_or_open(name)?;
        let data = arc.read().unwrap();

        let mut scored: Vec<(String, f32)> = data
            .records
            .iter()
            .map(|r| (r.document.clone(), cosine_distance(&embedding, &r.embedding)))
            .collect();

        scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(max_results);
        Ok(scored)
    }

    /// List every stored document text in a collection (admin endpoint). Blocking.
    pub fn list(&self, name: &str) -> Result<Vec<String>> {
        let arc = self.get_or_open(name)?;
        let data = arc.read().unwrap();
        Ok(data.records.iter().map(|r| r.document.clone()).collect())
    }
}

/// Cosine distance in `[0, 2]`: `1 - cos(a, b)`. Lower means closer. Vectors of mismatched
/// width or a zero vector are treated as maximally distant (`2.0`, the metric's true maximum —
/// `1 - cos` with `cos = -1`) so they sink below even genuinely anti-correlated documents.
/// BGE-M3 vectors have negative components, so real anti-correlation (distance `> 1.0`) occurs;
/// returning the midpoint `1.0` here would rank a corrupted vector *ahead* of such documents.
fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || a.len() != b.len() {
        return 2.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 2.0;
    }
    1.0 - dot / (na.sqrt() * nb.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-6, "expected ~{b}, got {a}");
    }

    #[test]
    fn identical_direction_is_zero() {
        approx(cosine_distance(&[1.0, 0.0, 0.0], &[1.0, 0.0, 0.0]), 0.0);
        // Magnitude doesn't matter — only direction.
        approx(cosine_distance(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]), 0.0);
    }

    #[test]
    fn orthogonal_is_one() {
        approx(cosine_distance(&[1.0, 0.0], &[0.0, 1.0]), 1.0);
    }

    #[test]
    fn opposite_is_two() {
        // The real maximum of the metric: anti-correlation, which BGE-M3 vectors can hit.
        approx(cosine_distance(&[1.0, 0.0], &[-1.0, 0.0]), 2.0);
    }

    #[test]
    fn mismatched_width_sinks_to_max_not_midpoint() {
        // Must be 2.0 (bottom), never 1.0 (orthogonal midpoint), so a corrupted/model-swapped
        // vector ranks BELOW a genuinely anti-correlated document rather than ahead of it.
        let corrupt = cosine_distance(&[1.0, 0.0, 0.0], &[0.0, 0.0]);
        let opposite = cosine_distance(&[1.0, 0.0], &[-1.0, 0.0]);
        approx(corrupt, 2.0);
        assert!(corrupt >= opposite, "corrupt {corrupt} must not outrank anti-correlated {opposite}");
    }

    #[test]
    fn zero_vector_is_max_distance() {
        approx(cosine_distance(&[0.0, 0.0], &[0.0, 0.0]), 2.0);
        approx(cosine_distance(&[1.0, 2.0], &[0.0, 0.0]), 2.0);
    }

    #[test]
    fn empty_input_is_max_distance() {
        approx(cosine_distance(&[], &[]), 2.0);
    }
}
