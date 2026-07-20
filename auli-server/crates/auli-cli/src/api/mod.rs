use std::sync::Arc;

use axum::{
    http::{header, HeaderValue, Method},
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

pub mod dto;
mod handlers;
pub mod ratelimit;

use crate::state::AppState;

use handlers::{health_handler, list_handler, question_handler, retrieve_handler};
use ratelimit::SharedLimiter;

// Rota pública sem estado: health-check.
pub fn public_routes() -> Router {
    Router::new().route("/v1/health", get(health_handler))
}

// Rota pública de perguntas (caminho RAG ativo). Precisa do estado compartilhado.
// Protegida por um limitador de taxa por IP (1 req/s, burst 2) — é o único caminho que chama o LLM.
// O limiter vem de FORA (D-MCP-6): é o mesmo objeto usado por `/v1/retrieve`, porque o recurso
// disputado é o embedder, comum às duas rotas.
pub fn question_routes(state: Arc<AppState>, limiter: SharedLimiter) -> Router {
    Router::new()
        .route("/v1/question", post(question_handler))
        .layer(middleware::from_fn_with_state(limiter, ratelimit::rate_limit))
        .with_state(state)
}

// Rota pública de retrieval puro (sem LLM): embeda a pergunta localmente, varre a coleção e
// devolve os documentos com score. CPU-bound no embedder ⇒ divide o MESMO limiter do question
// (D-MCP-6) — `question_rate_limiter()` constrói um limiter novo a cada chamada, então instanciar
// um por rota dobraria a cota efetiva por IP sobre o mesmo recurso.
pub fn retrieve_routes(state: Arc<AppState>, limiter: SharedLimiter) -> Router {
    Router::new()
        .route("/v1/retrieve", post(retrieve_handler))
        .layer(middleware::from_fn_with_state(limiter, ratelimit::rate_limit))
        .with_state(state)
}

// Rota de listagem de dados (somente leitura), genérica por `{kind}`. Pública.
// A ingestão NÃO é uma rota — é o `auli update`.
pub fn data_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/{kind}/list", get(list_handler))
        .with_state(state)
}

// CORS: origens permitidas fixas (auli.com.br + portas de desenvolvimento local).
// Métodos GET/POST/OPTIONS, com credenciais habilitadas.
pub fn cors_routes() -> CorsLayer {
    let origins: Vec<HeaderValue> = [
        "https://auli.com.br",
        "https://www.auli.com.br",
        "https://api.auli.com.br",
        "http://localhost:3000",
        "http://localhost:5173",
        "http://localhost:8080",
    ]
    .iter()
    .filter_map(|o| o.parse().ok())
    .collect();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true)
}
