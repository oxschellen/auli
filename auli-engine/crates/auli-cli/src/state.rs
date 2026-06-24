use std::sync::Arc;

use auli_core::embed::Embedder;

use crate::packs::Collections;

/// Shared server state. The vector collections are **read-only** (`ReadStore`), eager-loaded at
/// boot — the server holds no writer and cannot mutate them. The only thing it "produces" is the
/// ephemeral query vector, in memory.
#[derive(Clone)]
pub struct AppState {
    pub collections: Arc<Collections>,
    pub embedder: Arc<Embedder>,
}
