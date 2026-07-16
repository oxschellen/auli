use std::sync::Arc;
use std::time::Instant;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use tracing::{debug, info, warn};

use crate::api::dto::{Answer, Question};
use crate::rag::{exec_all_question, QueryType};
use crate::state::AppState;

pub async fn question_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<Question>,
) -> impl IntoResponse {
    let started = Instant::now();

    let entity = req.entity;
    let question = req.question;
    let query_type = QueryType::from_code(req.query_type);

    // Anonimiza a pergunta uma vez, na entrada. Fail-closed: em erro, usa o placeholder fixo —
    // nunca deixa o texto cru passar como se estivesse anonimizado.
    let question_sanitizada = state
        .anonimizador
        .anonimizar(&question)
        .map(|a| a.texto)
        .unwrap_or_else(|e| {
            warn!("anonimização falhou: {e}");
            auli_anon::TEXTO_FALLBACK_ERRO.to_string()
        });

    // stdout: sem IP e com a pergunta anonimizada (stdout costuma ser capturado — superfície de vazamento).
    info!(
        entity = entity.as_deref().unwrap_or("rs"),
        query_type = ?query_type,
        "Consulta: {}", question_sanitizada
    );

    let answer = exec_all_question(
        state.collections.clone(),
        state.embedder.clone(),
        question.clone(),
        question_sanitizada,
        entity,
        query_type,
    )
    .await
    .unwrap_or_else(|e| e.to_string());

    debug!("Consulta concluída em {} ms", started.elapsed().as_millis());

    (StatusCode::OK, Json(Answer { question, answer }))
}
