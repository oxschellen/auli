use axum::{http::StatusCode, response::IntoResponse};

// GET /v1/health — liveness probe. No state, no DB.
pub async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
