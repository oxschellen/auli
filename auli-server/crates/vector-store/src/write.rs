//! Write face of the store. Only `auli update` links this; the server cannot construct it.

use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{read_collection_file, write_collection_file, CollectionData, Error, Record, Result};

/// Writes `<name>.json` collection files under a base directory. Each operation reads the current
/// file, applies the change, and rewrites it — the clean-reload pattern (`reset` then `upsert`)
/// used by ingestion. Single-writer by design: there is exactly one `auli update` process.
pub struct Writer {
    base_path: PathBuf,
}

impl Writer {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self { base_path: base_path.into() }
    }

    /// On-disk path for a `<entity>-<kind>` collection name.
    pub fn path_for(&self, name: &str) -> PathBuf {
        self.base_path.join(format!("{name}.json"))
    }

    /// Drop every record in a collection (write an empty file). Used for clean full reloads so
    /// re-ingesting fewer blocks than before leaves no orphan `id-(N+1)..` records.
    pub fn reset<P: Serialize>(&self, name: &str) -> Result<()> {
        write_collection_file(self.path_for(name), &CollectionData::<P>::default())
    }

    /// Upsert `(id, embedding, payload)` triples (replace by id, else append) and persist.
    /// `ids` are the sequential `id-N` keys built by the caller, preserving today's scheme.
    /// Returns the total record count after the write.
    ///
    /// **Dimension is fixed on first insert.** The collection's width is whatever its first record
    /// established (or, for an empty collection, the first embedding in this batch); any embedding
    /// of a different width is rejected with [`Error::DimensionMismatch`]. This turns the old
    /// silent degrade (a wrong-width vector that `cosine_distance` would later score as max-distance)
    /// into a loud write-time error, so `cosine_distance`'s `2.0` fallback is left to cover only
    /// *legitimate* anti-correlation at query time.
    pub fn upsert<P>(&self, name: &str, ids: &[String], embeddings: Vec<Vec<f32>>, payloads: &[P]) -> Result<u64>
    where
        P: Serialize + DeserializeOwned + Clone,
    {
        // The three inputs are positional triples; a length mismatch would `zip` to the shortest and
        // drop records silently. Reject it before writing anything (the sole caller derives all three
        // from the same `table.items`, so this is a defensive contract, not a hot path).
        if ids.len() != embeddings.len() || ids.len() != payloads.len() {
            return Err(Error::ArityMismatch {
                ids: ids.len(),
                embeddings: embeddings.len(),
                payloads: payloads.len(),
            });
        }

        let path = self.path_for(name);
        let mut data: CollectionData<P> = read_collection_file(&path)?;

        // Establish the collection's dimension (existing records win; else this batch's first
        // embedding) and reject any divergent width before writing anything.
        if let Some(expected) = data
            .records
            .first()
            .map(|r| r.embedding.len())
            .or_else(|| embeddings.first().map(|e| e.len()))
            && let Some(bad) = embeddings.iter().find(|e| e.len() != expected)
        {
            return Err(Error::DimensionMismatch { expected, got: bad.len() });
        }

        for ((id, emb), payload) in ids.iter().zip(embeddings).zip(payloads.iter()) {
            let rec = Record { id: id.clone(), embedding: emb, payload: payload.clone() };
            match data.records.iter_mut().find(|r| r.id == rec.id) {
                Some(existing) => *existing = rec,
                None => data.records.push(rec),
            }
        }

        write_collection_file(&path, &data)?;
        Ok(data.records.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_then_upsert_writes_sequential_records() {
        let dir = std::env::temp_dir().join(format!("vs-writer-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let w = Writer::new(&dir);

        w.reset::<String>("rs-faqs").unwrap();
        let ids = vec!["id-1".to_string(), "id-2".to_string()];
        let embs = vec![vec![0.1, 0.2], vec![0.3, 0.4]];
        let payloads = vec!["alpha".to_string(), "beta".to_string()];
        let total = w.upsert("rs-faqs", &ids, embs, &payloads).unwrap();
        assert_eq!(total, 2);

        // Re-load and confirm the file round-trips with the `document` key.
        let data: CollectionData<String> = read_collection_file(w.path_for("rs-faqs")).unwrap();
        assert_eq!(data.records.len(), 2);
        assert_eq!(data.records[0].id, "id-1");
        assert_eq!(data.records[1].payload, "beta");

        // Upsert by existing id replaces in place (no duplicate, count stable).
        let total = w
            .upsert("rs-faqs", &["id-1".to_string()], vec![vec![9.0, 9.0]], &["ALPHA".to_string()])
            .unwrap();
        assert_eq!(total, 2);
        let data: CollectionData<String> = read_collection_file(w.path_for("rs-faqs")).unwrap();
        assert_eq!(data.records[0].payload, "ALPHA");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn upsert_rejects_mismatched_dimension() {
        let dir = std::env::temp_dir().join(format!("vs-dim-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let w = Writer::new(&dir);
        w.reset::<String>("c").unwrap();

        // First insert fixes width = 3.
        w.upsert("c", &["id-1".into()], vec![vec![1.0, 2.0, 3.0]], &["a".to_string()]).unwrap();

        // A later width-2 vector is rejected, and nothing is written.
        let err = w
            .upsert("c", &["id-2".into()], vec![vec![1.0, 2.0]], &["b".to_string()])
            .unwrap_err();
        assert!(matches!(err, Error::DimensionMismatch { expected: 3, got: 2 }));
        let data: CollectionData<String> = read_collection_file(w.path_for("c")).unwrap();
        assert_eq!(data.records.len(), 1, "rejected batch must not persist");

        // A mismatched width WITHIN the first batch (into an empty collection) is also caught.
        w.reset::<String>("d").unwrap();
        let err = w
            .upsert("d", &["id-1".into(), "id-2".into()], vec![vec![1.0, 2.0], vec![1.0, 2.0, 3.0]], &["a".to_string(), "b".to_string()])
            .unwrap_err();
        assert!(matches!(err, Error::DimensionMismatch { expected: 2, got: 3 }));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn upsert_rejects_arity_mismatch_without_writing() {
        let dir = std::env::temp_dir().join(format!("vs-arity-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let w = Writer::new(&dir);
        w.reset::<String>("c").unwrap();

        // Two ids but only one payload — must error, not truncate to one record.
        let err = w
            .upsert("c", &["id-1".into(), "id-2".into()], vec![vec![1.0], vec![2.0]], &["a".to_string()])
            .unwrap_err();
        assert!(matches!(err, Error::ArityMismatch { ids: 2, embeddings: 2, payloads: 1 }));
        let data: CollectionData<String> = read_collection_file(w.path_for("c")).unwrap();
        assert_eq!(data.records.len(), 0, "rejected batch must not persist");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
