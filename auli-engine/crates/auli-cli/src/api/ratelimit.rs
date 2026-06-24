//! Per-IP rate limiting for the public question route — the only path that calls the paid
//! external LLM. Keyed by the real client IP (proxy headers first: we sit behind a Cloudflare
//! Tunnel, so the socket peer is the proxy, not the caller). Because office networks NAT many
//! machines behind one public IP, the limit is effectively **per-network/organization**.
//!
//! Quota: **1 request/second sustained, bursts up to 2** (GCRA, via `governor`).

use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, Quota, RateLimiter};
use serde_json::json;

/// Keyed (per-IP) in-memory GCRA limiter. State per key is a few bytes; for help-desk volumes
/// (hundreds of organization IPs) memory is negligible, so we don't background-GC the map.
pub type IpRateLimiter = RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>;

/// Build the limiter for `/v1/question`: 1 req/s sustained, burst capacity 2.
pub fn question_rate_limiter() -> Arc<IpRateLimiter> {
    let quota = Quota::per_second(NonZeroU32::new(1).unwrap()).allow_burst(NonZeroU32::new(2).unwrap());
    Arc::new(RateLimiter::keyed(quota))
}

/// Middleware: admit or reject by the caller's IP. Runs **before** the handler, so a rejected
/// request never reaches the LLM. Returns `429` with a friendly pt-BR body when the bucket is empty.
pub async fn rate_limit(
    State(limiter): State<Arc<IpRateLimiter>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    req: Request<Body>,
    next: Next,
) -> Response {
    let ip = client_ip(&headers).unwrap_or_else(|| addr.ip());
    match limiter.check_key(&ip) {
        Ok(()) => next.run(req).await,
        Err(_) => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({ "error": "Muitas requisições. Aguarde alguns instantes e tente novamente." })),
        )
            .into_response(),
    }
}

/// Real client IP from the usual proxy headers, in priority order, taking the leftmost value of
/// each (the original client in an `X-Forwarded-For`-style chain). Returns `None` if none carry a
/// parseable address, so the caller can fall back to the socket peer.
pub(crate) fn client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    const IP_HEADERS: &[&str] = &[
        "CF-Connecting-IP",         // Cloudflare
        "True-Client-IP",           // Cloudflare Enterprise
        "X-Real-IP",                // Nginx
        "X-Forwarded-For",          // most common (can be spoofed)
        "X-Cluster-Client-IP",      // GCP Load Balancer
        "X-Original-Forwarded-For", // AWS ALB
    ];

    IP_HEADERS.iter().find_map(|name| {
        let value = headers.get(*name)?.to_str().ok()?;
        let first = value.split(',').next().unwrap_or(value).trim();
        first.parse::<IpAddr>().ok()
    })
}
