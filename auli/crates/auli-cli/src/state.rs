use std::sync::Arc;

use sqlx::postgres::PgPool;

use auli_core::embed::Embedder;

use crate::packs::Collections;

/// Shared server state. The vector collections are **read-only** (`ReadStore`), eager-loaded at
/// boot — the server holds no writer and cannot mutate them. The only thing it "produces" is the
/// ephemeral query vector, in memory.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub collections: Arc<Collections>,
    pub embedder: Arc<Embedder>,
    #[allow(dead_code)]
    pub secret: String,
    #[allow(dead_code)]
    pub access_min: i64,
    #[allow(dead_code)]
    pub refresh_days: i64,
    #[allow(dead_code)]
    pub verify_h: i64,
    #[allow(dead_code)]
    pub reset_h: i64,
}
