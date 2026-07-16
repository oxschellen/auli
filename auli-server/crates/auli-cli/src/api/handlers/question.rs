use std::sync::Arc;
use std::time::Instant;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use tracing::debug;

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

    // A anonimização (pergunta → LLM/log) e a restauração da resposta vivem em `exec_all_question`,
    // onde o `mapping` fica no escopo da requisição. O handler devolve a pergunta ORIGINAL ao front.
    let answer = exec_all_question(
        state.collections.clone(),
        state.embedder.clone(),
        state.anonimizador.clone(),
        question.clone(),
        entity,
        query_type,
    )
    .await
    .unwrap_or_else(|e| e.to_string());

    debug!("Consulta concluída em {} ms", started.elapsed().as_millis());

    (StatusCode::OK, Json(Answer { question, answer }))
}
