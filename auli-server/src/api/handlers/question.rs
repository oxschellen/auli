use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use tracing::{debug, info};

use crate::api::dto::{Answer, Question};
use crate::rag::exec_all_question;
use crate::state::AppState;

// #[axum::debug_handler]
pub async fn question_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<Question>,
) -> impl IntoResponse {
    let started = SystemTime::now();

    let client_ip = get_client_ip(&headers).unwrap_or_else(|| addr.ip().to_string());
    let entity = req.entity;
    let question = req.question;

    info!(ip = %client_ip, entity = entity.as_deref().unwrap_or("rs"), "Consulta: {}", question);

    let answer = exec_all_question(state.vector.clone(), state.embedder.clone(), question.clone(), entity)
        .await
        .unwrap_or_else(|e| e.to_string());

    debug!("Consulta concluída em {} ms", started.elapsed().unwrap().as_millis());

    (StatusCode::OK, Json(Answer { question, answer }))
}

// Extract the real client IP from the usual proxy headers, in priority order. For each, take
// the leftmost value (the original client in an `X-Forwarded-For`-style chain). Returns None if
// none carry a usable value, so the caller can fall back to the socket address.
fn get_client_ip(headers: &HeaderMap) -> Option<String> {
    const IP_HEADERS: &[&str] = &[
        "CF-Connecting-IP",        // Cloudflare
        "True-Client-IP",          // Cloudflare Enterprise
        "X-Real-IP",               // Nginx
        "X-Forwarded-For",         // most common (can be spoofed)
        "X-Cluster-Client-IP",     // GCP Load Balancer
        "X-Original-Forwarded-For", // AWS ALB
    ];

    IP_HEADERS.iter().find_map(|name| {
        let value = headers.get(*name)?.to_str().ok()?;
        let ip = value.split(',').next().unwrap_or(value).trim();
        (!ip.is_empty() && ip != "unknown").then(|| ip.to_string())
    })
}
