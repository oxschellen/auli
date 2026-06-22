use std::sync::Arc;

use sqlx::postgres::PgPool;

use crate::clients::embedder::Embedder;
use crate::clients::vector_store::VectorStore;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    // In-process vector store, built once at startup and shared across handlers.
    pub vector: Arc<VectorStore>,
    // In-process embedder (fastembed/BGE-M3), built once at startup.
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
