//! Agnostic, pure-Rust flat cosine vector store — the "anti-zvec".
//!
//! Each `<entity>-<kind>` collection is a flat list of `(id, embedding, payload)` records,
//! persisted as one JSON file. Similarity search is an exact brute-force cosine scan, which is
//! more than fast enough for the small per-collection corpora here (thousands of documents). No
//! external service, no C++ toolchain, no fixed embedding dimension — vectors are compared at
//! whatever width the caller emits.
//!
//! The store knows **only** `(id, vector, payload P)`. It has zero coupling with embeddings,
//! tributação, or the auli domain — the embedding *identity*/manifest lives one layer up, in
//! `auli-core`, never here.
//!
//! **Read and write are separate types by design** (the load-bearing correctness property):
//! - [`ReadStore`] opens a collection read-only and exposes `query_scored`/`list`.
//! - [`Writer`] does `reset`/`upsert` and persists.
//!
//! A consumer that only reads (the `auli server`) links [`ReadStore`] and **cannot construct a
//! [`Writer`]** — it is incapable of writing by construction, not by discipline.

mod error;
mod read;
mod write;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub use error::{Error, Result};
pub use read::ReadStore;
pub use write::Writer;

/// One stored record: a sequential id, its embedding, and an opaque payload.
///
/// `P` is whatever the caller stores alongside the vector (today: the document text). The store
/// never inspects it. Renamed from the old `document` field to `payload` to make the genericity
/// explicit, while the on-disk JSON key stays `document` for format compatibility (see the
/// `#[serde(rename)]` below).
#[derive(Clone, Serialize, Deserialize)]
pub struct Record<P> {
    pub id: String,
    pub embedding: Vec<f32>,
    #[serde(rename = "document")]
    pub payload: P,
}

/// A whole collection: just its records. Persisted verbatim as `{ "records": [...] }`.
#[derive(Serialize, Deserialize)]
pub struct CollectionData<P> {
    pub records: Vec<Record<P>>,
}

// Manual `Default` so callers don't need `P: Default` to make an empty collection.
impl<P> Default for CollectionData<P> {
    fn default() -> Self {
        Self { records: Vec::new() }
    }
}

/// Read a collection file. A missing file is **not** an error — it yields an empty collection,
/// preserving the old `load_from_disk` semantics.
pub fn read_collection_file<P: DeserializeOwned>(path: impl AsRef<Path>) -> Result<CollectionData<P>> {
    match fs::read(path.as_ref()) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(CollectionData::default()),
        Err(e) => Err(e.into()),
    }
}

/// Write a collection file, creating parent directories as needed. The whole file is rewritten on
/// every call — O(n), fine for the small corpora here (see the README tradeoff note).
pub fn write_collection_file<P: Serialize>(path: impl AsRef<Path>, data: &CollectionData<P>) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec(data)?)?;
    Ok(())
}

/// Cosine distance in `[0, 2]`: `1 - cos(a, b)`. Lower means closer. Vectors of mismatched
/// width or a zero vector are treated as maximally distant (`2.0`, the metric's true maximum —
/// `1 - cos` with `cos = -1`) so they sink below even genuinely anti-correlated documents.
/// Returning the midpoint `1.0` here would rank a corrupted vector *ahead* of such documents.
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
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

/// Score every record against `embedding` and return `(payload, distance)` sorted best-first
/// (ascending distance), truncated to `max_results`. Shared by [`ReadStore`] and the Phase-1
/// server registry so the two cannot diverge.
pub(crate) fn scan<P: Clone>(records: &[Record<P>], embedding: &[f32], max_results: usize) -> Vec<(P, f32)> {
    let mut scored: Vec<(P, f32)> = records
        .iter()
        .map(|r| (r.payload.clone(), cosine_distance(embedding, &r.embedding)))
        .collect();
    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(max_results);
    scored
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
        approx(cosine_distance(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]), 0.0);
    }

    #[test]
    fn orthogonal_is_one() {
        approx(cosine_distance(&[1.0, 0.0], &[0.0, 1.0]), 1.0);
    }

    #[test]
    fn opposite_is_two() {
        approx(cosine_distance(&[1.0, 0.0], &[-1.0, 0.0]), 2.0);
    }

    #[test]
    fn mismatched_width_sinks_to_max_not_midpoint() {
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

    #[test]
    fn payload_is_generic_and_roundtrips_as_document_key() {
        // On-disk key stays `document` for format compatibility even though the field is `payload`.
        let data = CollectionData {
            records: vec![Record { id: "id-1".into(), embedding: vec![0.1, 0.2], payload: "hello".to_string() }],
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"document\":\"hello\""), "got {json}");
        let back: CollectionData<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.records[0].payload, "hello");
    }

    #[test]
    fn scan_sorts_best_first_and_truncates() {
        let recs = vec![
            Record { id: "id-1".into(), embedding: vec![1.0, 0.0], payload: "a".to_string() },
            Record { id: "id-2".into(), embedding: vec![0.0, 1.0], payload: "b".to_string() },
            Record { id: "id-3".into(), embedding: vec![-1.0, 0.0], payload: "c".to_string() },
        ];
        let out = scan(&recs, &[1.0, 0.0], 2);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, "a"); // identical -> distance 0
        assert_eq!(out[1].0, "b"); // orthogonal -> distance 1
    }
}
