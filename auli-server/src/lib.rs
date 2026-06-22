use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

pub mod api;
pub mod auth;
pub mod clients;
pub mod config;
pub mod domain;
pub mod errors;
pub mod rag;
pub mod state;
mod util;

use crate::api::{auth_routes, cors_routes, data_routes, protected_routes, public_routes, question_routes};
use crate::clients::embedder::Embedder;
use crate::clients::vector_store::VectorStore;
use crate::config::config;
use crate::state::AppState;

/// Address the HTTP server binds to.
const BIND_ADDR: &str = "0.0.0.0:3000";

/// Assemble the full application router. Kept separate from `run` so tests can build the
/// router and exercise handlers without binding a socket.
pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .merge(public_routes())
        .merge(question_routes(state.clone()))
        .merge(auth_routes(state.clone()))
        .merge(protected_routes(state.clone()))
        .merge(data_routes(state))
        .layer(cors_routes())
}

/// Build shared state and serve on 0.0.0.0:3000.
pub async fn run() {
    // Default to `info`; override per-target with RUST_LOG (e.g. `RUST_LOG=auli_server=debug`
    // to see score arrays / the full RAG prompt, or `=trace` for everything).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    config().log_summary();
    domain::entities::init();

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config().database_url)
        .await
        .expect("Falha ao conectar ao PostgreSQL");
    println!("🗄️  PostgreSQL database connected successfully!");

    let vector = Arc::new(VectorStore::new(config().vector_db_path.clone()));
    println!("📦 Vector store em {}", config().vector_db_path);

    // Load the embedding model once before serving (slow: loads/downloads the ONNX model).
    let embedder = Arc::new(
        Embedder::new(config().embed_cache_dir.clone().into(), config().embed_threads)
            .expect("Falha ao inicializar o embedder (fastembed/BGE-M3)"),
    );
    println!("🧠 Embedder fastembed (BGE-M3) carregado");

    let state = Arc::new(AppState {
        pool,
        vector,
        embedder,
        secret: config().jwt_secret.clone(),
        access_min: config().jwt_access_expiration_minutes,
        refresh_days: config().jwt_refresh_expiration_days,
        verify_h: config().verification_token_expiry_hours,
        reset_h: config().password_reset_token_expiry_hours,
    });

    println!("----------------------------------------------------");
    println!("Auli Server v0.2.0 - Local embeddings + vector store");
    println!("----------------------------------------------------");

    let app = app(state);

    // Bind first, then announce — so the success line can't print if the port is taken.
    let listener = TcpListener::bind(BIND_ADDR)
        .await
        .unwrap_or_else(|e| panic!("Falha ao escutar em {BIND_ADDR}: {e}"));
    println!("✅ Server started successfully at {BIND_ADDR}");
    println!(" ");

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("Falha no servidor HTTP");
}
