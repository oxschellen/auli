//! End-to-end serving-path smoke test against REAL packs built by `auli update`.
//!
//! Gated on `AULI_PACKS_DIR` (and uses `EMBED_CACHE_DIR` for the model), so a normal `cargo test`
//! skips it — it needs the generated `<entity>-<kind>.json` packs and the BGE-M3 model on disk.
//! Run with:
//!   AULI_PACKS_DIR=../../packs EMBED_CACHE_DIR=../../models cargo test -p auli-cli --release \
//!     --test packs_smoke -- --nocapture --ignored
//!
//! It exercises exactly the server's read path: validate the manifest against the local identity,
//! load a collection read-only, embed a real question, and confirm retrieval returns sane,
//! best-first cosine distances.

use auli_core::embed::Embedder;
use auli_core::manifest::{self, identity};
use vector_store::ReadStore;

fn packs_dir() -> Option<std::path::PathBuf> {
    std::env::var("AULI_PACKS_DIR").ok().map(Into::into)
}

#[test]
#[ignore = "needs generated packs + model; run explicitly with AULI_PACKS_DIR set"]
fn manifest_validates_and_query_retrieves() {
    let Some(packs) = packs_dir() else {
        eprintln!("AULI_PACKS_DIR unset — skipping");
        return;
    };

    // 1. Manifest validates against the build's embedding identity (the server's boot gate).
    let mpath = manifest::manifest_path(&packs, "rs");
    let m = manifest::validate_manifest(&mpath, &identity()).expect("manifest should validate");
    assert_eq!(m.embed_dim, 1024);
    assert!(m.collections.iter().any(|c| c.kind == "faqs" && c.count > 0));

    // 2. Load the faqs collection read-only.
    let store = ReadStore::<String>::load(packs.join("rs-faqs.json")).expect("load rs-faqs");
    assert!(store.len() > 1000, "expected the full faq corpus, got {}", store.len());

    // 3. Embed a real question with the SAME encoder the packs were built with.
    let cache = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "./models".into());
    let embedder = Embedder::new(cache.into(), 16).expect("load embedder");
    let q = embedder
        .embed_dense(vec!["Como obtenho uma certidão negativa de débitos?".to_string()])
        .expect("embed question")
        .pop()
        .expect("one vector");
    assert_eq!(q.len(), 1024);

    // 4. Retrieve. Results come back best-first; the top distance must be a real match, not the
    //    2.0 max-distance fallback (which would signal dimension/space mismatch).
    let hits = store.query_scored(&q, 20);
    assert_eq!(hits.len(), 20);
    let distances: Vec<f32> = hits.iter().map(|(_, d)| *d).collect();
    assert!(distances.windows(2).all(|w| w[0] <= w[1]), "must be sorted best-first");
    assert!(distances[0] < 1.0, "top hit should be a genuine match, got dist {}", distances[0]);
    assert!(distances[0] >= 0.0, "cosine distance is non-negative");

    eprintln!("top-5 distances: {:?}", &distances[..5]);
    eprintln!("top hit document starts: {:?}", &hits[0].0.chars().take(80).collect::<String>());
}
