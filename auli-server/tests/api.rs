// Integration test exercising the router through the library seam (no socket, no DB).

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use tower::ServiceExt; // for `oneshot`

use auli_server::api::public_routes;

#[tokio::test]
async fn health_returns_200() {
    let mut request = Request::builder().uri("/v1/health").body(Body::empty()).unwrap();
    // /v1/health extracts ConnectInfo<SocketAddr>; supply it since there's no real connection.
    request.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));

    let response = public_routes().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
