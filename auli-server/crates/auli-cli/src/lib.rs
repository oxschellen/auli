//! `auli-cli` library — the application layer. Holds the server (axum/RAG) and the update
//! (vectorizer) flows; the `auli` binary in `main.rs` is a thin clap dispatcher over the two.
//!
//! Layering: depends on `auli-core` (domain) and `vector-store` (storage). The server links only
//! the read face (`ReadStore`, via `packs`); the writer (`vector_store::Writer`) is used solely by
//! `update` — so the server is incapable of mutating packs by construction.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;

pub mod api;
pub mod config;
pub mod entities;
pub mod error;
pub mod llm;
pub mod packs;
pub mod rag;
pub mod state;
pub mod update;
mod util;

use crate::api::{cors_routes, data_routes, public_routes, question_routes};
use crate::config::config;
use crate::state::AppState;

pub use update::run_update;

/// Assemble the full application router. Kept separate from `run_server` so tests can build the
/// router and exercise handlers without binding a socket.
pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .merge(public_routes())
        .merge(question_routes(state.clone()))
        .merge(data_routes(state))
        .layer(cors_routes())
}

/// Build shared state from pre-built packs and serve on `<bind>:<port>`.
/// `bind` default é `0.0.0.0` (instância única); multi-instância atrás de
/// reverse proxy deve passar `127.0.0.1` para não expor a porta na rede.
pub async fn run_server(packs_dir: Option<String>, port: u16, bind: String) {
    // Default to `info`; override per-target with RUST_LOG (e.g. `RUST_LOG=auli_cli=debug`
    // to see score arrays / the full RAG prompt, or `=trace` for everything).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    config().log_summary();
    entities::init();

    // Packs live under `<data>/<id>/packs/`. With no `--packs-dir`, fall back to the shared data
    // root (`AULI_DATA_DIR`, default `./data`) — the same dir the registry/prompts load from — so
    // the registry and the packs can never resolve to different roots by accident.
    let packs_dir = packs_dir.unwrap_or_else(|| entities::data_dir().to_string_lossy().into_owned());

    // Eager-load + validate all packs before serving (refuse to start on incompatible data).
    let collections = packs::load_all(&packs_dir).expect("Falha ao carregar os pacotes de vetores");
    println!("📦 Pacotes carregados de {}", packs_dir);

    // Load the embedding model once before serving (slow: loads/downloads the ONNX model).
    let embedder = Arc::new(
        auli_core::embed::Embedder::new(config().embed_cache_dir.clone().into(), config().embed_threads)
            .expect("Falha ao inicializar o embedder (fastembed/BGE-M3)"),
    );
    println!("🧠 Embedder fastembed (BGE-M3) carregado");

    let state = Arc::new(AppState {
        collections: Arc::new(collections),
        embedder,
    });

    println!("----------------------------------------------------");
    println!("Auli Server v{} - Read-only packs + in-process embeddings", env!("CARGO_PKG_VERSION"));
    println!("----------------------------------------------------");

    let app = app(state);

    let bind_addr = format!("{bind}:{port}");
    // Bind first, then announce — so the success line can't print if the port is taken.
    let listener = TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("Falha ao escutar em {bind_addr}: {e}"));
    println!("✅ Server started successfully at {bind_addr}");
    println!(" ");

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("Falha no servidor HTTP");
}
