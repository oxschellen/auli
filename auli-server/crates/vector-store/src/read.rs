//! Read-only face of the store. The `auli server` links this and never the [`crate::Writer`].

use std::path::Path;

use serde::de::DeserializeOwned;

use crate::{read_collection_file, scan, Record, Result};

/// A single collection opened read-only. Holds its records as an **immutable** vector — once
/// loaded it is never mutated, so queries need no lock and an `Arc<ReadStore<P>>` can be shared
/// across handler threads freely (the eager-load model in the server).
pub struct ReadStore<P> {
    records: Vec<Record<P>>,
}

impl<P> ReadStore<P> {
    /// Wrap already-loaded records (e.g. from a custom loader or a test).
    pub fn from_records(records: Vec<Record<P>>) -> Self {
        Self { records }
    }

    /// Load a collection file read-only. A missing file yields an empty store.
    pub fn load(path: impl AsRef<Path>) -> Result<Self>
    where
        P: DeserializeOwned,
    {
        Ok(Self { records: read_collection_file::<P>(path)?.records })
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

impl<P: Clone> ReadStore<P> {
    /// Vector similarity search returning `(payload, score)` pairs sorted best-first. `score` is a
    /// cosine DISTANCE — lower is closer (0.0 == identical direction). `max_results` caps the return.
    pub fn query_scored(&self, embedding: &[f32], max_results: usize) -> Vec<(P, f32)> {
        scan(&self.records, embedding, max_results)
    }

    /// Every stored payload, in insertion order (admin/list endpoint).
    pub fn list(&self) -> Vec<P> {
        self.records.iter().map(|r| r.payload.clone()).collect()
    }
}
