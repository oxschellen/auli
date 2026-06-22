use std::sync::Arc;

use axum::{
    http::{header, HeaderValue, Method},
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

pub mod dto;
mod handlers;

use crate::state::AppState;

use handlers::{health_handler, list_handler, question_handler};

// Rota pública sem estado: health-check.
pub fn public_routes() -> Router {
    Router::new().route("/v1/health", get(health_handler))
}

// Rota pública de perguntas (caminho RAG ativo). Precisa do estado compartilhado.
pub fn question_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/question", post(question_handler))
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
