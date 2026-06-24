use auli_cli::api::public_routes;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt; // for `oneshot`

// Builds the public router and confirms GET /v1/health responds 200 — no socket, no DB.
#[tokio::test]
async fn health_returns_200() {
    let app = public_routes();
    let response = app
        .oneshot(Request::builder().uri("/v1/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
