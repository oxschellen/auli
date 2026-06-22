use std::sync::Arc;

use axum::{
    http::{header, HeaderValue, Method},
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

mod dto;
mod handlers;

use crate::auth::jwt::gen_rsa_keypair_handler;
use crate::auth::{auth_middleware, sign_in_handler, user_get_handler, user_register_handler};
use crate::state::AppState;

use handlers::{
    health_handler, list_handler, load_from_file_handler, load_from_web_handler, question_handler,
};

// Rotas públicas sem estado: health-check e geração de par de chaves RSA (utilitário).
pub fn public_routes() -> Router {
    Router::new()
        .route("/v1/health", get(health_handler))
        .route("/v1/gen_rsa_keypair", get(gen_rsa_keypair_handler))
}

// Rota pública de perguntas (caminho RAG ativo). Precisa do estado compartilhado.
pub fn question_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/question", post(question_handler))
        .with_state(state)
}

// Rotas de autenticação (login e registro). Públicas, mas dependem do banco no estado.
pub fn auth_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/signin", post(sign_in_handler))
        .route("/register", post(user_register_handler))
        .with_state(state)
}

// Rotas protegidas por JWT: exigem `Authorization: Bearer <token>` de um usuário verificado.
pub fn protected_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/protected", get(user_get_handler))
        .route("/v1/protected_question", post(question_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state)
}

// Rotas de gerenciamento de dados (ingestão/listagem), genéricas por `{kind}`. Exigem JWT.
pub fn data_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/{kind}/list", get(list_handler))
        .route("/v1/{kind}/load_from_file", get(load_from_file_handler))
        .route("/v1/{kind}/load_from_web", post(load_from_web_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
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
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true)
}
