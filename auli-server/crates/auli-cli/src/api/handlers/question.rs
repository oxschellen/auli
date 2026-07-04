use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use tracing::{debug, info};

use crate::api::dto::{Answer, Question};
use crate::api::ratelimit::client_ip;
use crate::rag::exec_all_question;
use crate::state::AppState;

pub async fn question_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<Question>,
) -> impl IntoResponse {
    let started = Instant::now();

    let client_ip = client_ip(&headers).unwrap_or_else(|| addr.ip());
    let entity = req.entity;
    let question = req.question;

    info!(ip = %client_ip, entity = entity.as_deref().unwrap_or("rs"), "Consulta: {}", question);

    let answer = exec_all_question(state.collections.clone(), state.embedder.clone(), question.clone(), entity)
        .await
        .unwrap_or_else(|e| e.to_string());

    debug!("Consulta concluída em {} ms", started.elapsed().as_millis());

    (StatusCode::OK, Json(Answer { question, answer }))
}
