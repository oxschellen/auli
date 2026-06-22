//! Boot-time pack loading for the server (read-only).
//!
//! For each registered entity: validate the pack manifest against this build's embedding identity
//! (refuse to serve on mismatch), then **eager-load** every `<entity>-<kind>.json` into an
//! immutable [`ReadStore`]. Eager + immutable buys three things: the manifest check happens at boot
//! (the server refuses to start on incompatible data instead of discovering it mid-request), no
//! cold-start latency on the first question, and — being read-only — no lock on the query path.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use auli_core::corpus;
use auli_core::manifest;
use vector_store::ReadStore;

use crate::entities;
use crate::error::Result;

/// All loaded collections, keyed by `<entity>-<kind>`.
pub type Collections = HashMap<String, Arc<ReadStore<String>>>;

/// Eager-load and validate every entity's packs. Returns the in-memory, immutable collection map.
///
/// Layout: packs live **per entity** under `<packs_root>/<id>/packs/` — `<id>-<kind>.json` plus
/// `<id>.manifest.json` (see `data/` integration plan). A missing collection file loads as an empty
/// store (`read_collection_file` tolerates `NotFound`), so a partial entity (e.g. `sc` with only
/// `services`) boots cleanly.
pub fn load_all(packs_root: impl AsRef<Path>) -> Result<Collections> {
    let packs_root = packs_root.as_ref();
    let expected = manifest::identity();
    let mut map: Collections = HashMap::new();

    for id in entities::ENTITIES.keys() {
        let packs_dir = packs_root.join(id).join("packs");
        let manifest_path = manifest::manifest_path(&packs_dir, id);
        if manifest_path.exists() {
            // Hard fail on model/dim/strategy mismatch — never serve from a foreign vector space.
            manifest::validate_manifest(&manifest_path, &expected)?;
            println!("🔎 Manifesto de '{}' validado contra a identidade local.", id);
        } else {
            eprintln!(
                "⚠️  Manifesto ausente para '{}' ({:?}). Carregando pacotes sem validação — gere com `auli update`.",
                id, manifest_path
            );
        }

        for collection in corpus::ALL {
            let name = format!("{}-{}", id, collection.kind);
            let path = packs_dir.join(format!("{}.json", name));
            let store = ReadStore::<String>::load(&path)?;
            println!("📦 {} — {} registros", name, store.len());
            map.insert(name, Arc::new(store));
        }
    }

    Ok(map)
}
