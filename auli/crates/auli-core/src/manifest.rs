//! Embedding identity + pack manifest.
//!
//! A manifest pins the *identity* of the embedding space a set of packs was built with: which
//! model, which dimension, and which `STRATEGY_VERSION` (what `corpus::prepare_documents`/`parse_*`
//! actually embedded). `auli update` writes it; `auli server` validates the local identity against
//! it at boot and **refuses to serve** on a mismatch — turning "forgot to re-ingest after changing
//! the strategy" from silent bad retrieval into a loud boot error.
//!
//! This lives in `auli-core`, never in `vector-store`: an agnostic store cannot know what an
//! "embedding model" is. Identity is a domain concept.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::embed::EMBED_DIM;
use crate::error::{Error, Result};

/// Model identifier baked into every manifest. Change this whenever the model or quantization
/// changes (it implies a full re-ingest).
pub const EMBED_MODEL_ID: &str = "bge-m3-q-int8";

/// Bumped whenever `corpus::prepare_documents` / `parse_*` change what gets embedded. A pack built
/// under an old strategy is incompatible with a server running a new one even if the model matches.
pub const STRATEGY_VERSION: u32 = 1;

/// The triple that must match between the packs and the running server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedIdentity {
    pub embed_model_id: String,
    pub embed_dim: usize,
    pub strategy_version: u32,
}

/// The local identity of this build. Compared against a pack's manifest at server boot.
pub fn identity() -> EmbedIdentity {
    EmbedIdentity {
        embed_model_id: EMBED_MODEL_ID.to_string(),
        embed_dim: EMBED_DIM,
        strategy_version: STRATEGY_VERSION,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionEntry {
    pub kind: String,
    pub count: usize,
    pub dim: usize,
    pub file: String,
    pub bytes: u64,
    /// FNV-1a 64 of the collection file bytes — catches a half-copied/corrupted pack.
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub entity: String,
    pub version: String,
    pub built_at: String,
    pub embed_model_id: String,
    pub embed_dim: usize,
    pub strategy_version: u32,
    pub collections: Vec<CollectionEntry>,
}

impl Manifest {
    pub fn embed_identity(&self) -> EmbedIdentity {
        EmbedIdentity {
            embed_model_id: self.embed_model_id.clone(),
            embed_dim: self.embed_dim,
            strategy_version: self.strategy_version,
        }
    }
}

/// FNV-1a 64-bit hash of a byte slice. Cheap, dependency-free, plenty for "is this the file I
/// wrote?" integrity — not a cryptographic guarantee.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Hex string form used in the manifest.
pub fn hash_hex(bytes: &[u8]) -> String {
    format!("{:016x}", fnv1a64(bytes))
}

/// Manifest file name for an entity: `<entity>.manifest.json`.
pub fn manifest_path(packs_dir: impl AsRef<Path>, entity: &str) -> std::path::PathBuf {
    packs_dir.as_ref().join(format!("{entity}.manifest.json"))
}

pub fn write_manifest(path: impl AsRef<Path>, manifest: &Manifest) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(manifest)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

pub fn read_manifest(path: impl AsRef<Path>) -> Result<Manifest> {
    let bytes = std::fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Read the manifest at `path` and check its embedding identity equals `expected`. A divergence in
/// model, dimension, or strategy version is a hard error: the server should refuse to serve rather
/// than answer from a vector space that doesn't match its query encoder.
pub fn validate_manifest(path: impl AsRef<Path>, expected: &EmbedIdentity) -> Result<Manifest> {
    let manifest = read_manifest(path)?;
    let got = manifest.embed_identity();
    if &got != expected {
        return Err(Error::from(format!(
            "Manifest incompatível: pacote tem (modelo={}, dim={}, strategy={}), servidor espera (modelo={}, dim={}, strategy={}). Re-gere os pacotes com `auli update`.",
            got.embed_model_id, got.embed_dim, got.strategy_version,
            expected.embed_model_id, expected.embed_dim, expected.strategy_version,
        )));
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_round_trips_and_matches_self() {
        let id = identity();
        assert_eq!(id.embed_model_id, EMBED_MODEL_ID);
        assert_eq!(id.embed_dim, EMBED_DIM);
        assert_eq!(id, id.clone());
    }

    #[test]
    fn fnv1a_known_vector() {
        // FNV-1a 64 of "" is the offset basis; of "a" is a well-known constant.
        assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);
        assert_eq!(fnv1a64(b"a"), 0xaf63dc4c8601ec8c);
    }

    #[test]
    fn validate_rejects_strategy_mismatch() {
        let dir = std::env::temp_dir().join(format!("auli-manifest-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = manifest_path(&dir, "rs");
        let mut m = Manifest {
            entity: "rs".into(),
            version: "1".into(),
            built_at: "now".into(),
            embed_model_id: EMBED_MODEL_ID.into(),
            embed_dim: EMBED_DIM,
            strategy_version: STRATEGY_VERSION + 1, // wrong
            collections: vec![],
        };
        write_manifest(&path, &m).unwrap();
        assert!(validate_manifest(&path, &identity()).is_err());

        m.strategy_version = STRATEGY_VERSION; // fixed
        write_manifest(&path, &m).unwrap();
        assert!(validate_manifest(&path, &identity()).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
