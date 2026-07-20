//! Per-IP rate limiting for the public question route — the only path that calls the paid
//! external LLM. Keyed by the real client IP. We sit behind a **Cloudflare Tunnel**, so the socket
//! peer is the proxy; the real caller is in Cloudflare's `CF-Connecting-IP` header. We trust **only**
//! Cloudflare's headers (`CF-Connecting-IP` / `True-Client-IP`) — never the generic, caller-settable
//! `X-Forwarded-For`/`X-Real-IP` family, which any client hitting the port directly (the `--bind
//! 0.0.0.0` default allows this) could forge to bypass the limit and grow the key map unboundedly.
//! With no Cloudflare header present we fall back to the socket peer. Because office networks NAT
//! many machines behind one public IP, the limit is effectively **per-network/organization**.
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

/// Um limiter compartilhável entre rotas. Cada chamada de construtor cria um limiter NOVO (com sua
/// própria cota por IP), então rotas que protegem o MESMO recurso precisam receber o mesmo `Arc` —
/// construir um por rota dobraria a cota efetiva. Ver D-MCP-6.
pub type SharedLimiter = Arc<IpRateLimiter>;

/// Build the limiter for `/v1/question`: 1 req/s sustained, burst capacity 2.
///
/// Compartilhado com `/v1/retrieve` (D-MCP-6): o recurso caro e disputado é o embedder, e as duas
/// rotas o tocam. O `app()` constrói UM e injeta nas duas.
pub fn question_rate_limiter() -> SharedLimiter {
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

/// Real client IP from **Cloudflare's** headers only, in priority order, taking the leftmost value
/// (the original client). Returns `None` if neither header carries a parseable address, so the
/// caller falls back to the socket peer. The generic `X-Forwarded-For`/`X-Real-IP` family is
/// deliberately NOT trusted: it is caller-settable and would let a direct-to-port client forge the
/// rate-limit key. Only reinstate a non-CF proxy's header behind an explicit trusted-proxy flag.
pub(crate) fn client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    const IP_HEADERS: &[&str] = &[
        "CF-Connecting-IP", // Cloudflare Tunnel (our deployment)
        "True-Client-IP",   // Cloudflare Enterprise
    ];

    IP_HEADERS.iter().find_map(|name| {
        let value = headers.get(*name)?.to_str().ok()?;
        let first = value.split(',').next().unwrap_or(value).trim();
        first.parse::<IpAddr>().ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_ip_trusts_cloudflare_headers_only() {
        // A forged X-Forwarded-For is ignored -> None (caller falls back to the socket peer).
        let mut spoofed = HeaderMap::new();
        spoofed.insert("X-Forwarded-For", "1.2.3.4".parse().unwrap());
        spoofed.insert("X-Real-IP", "5.6.7.8".parse().unwrap());
        assert_eq!(client_ip(&spoofed), None);

        // CF-Connecting-IP is trusted; leftmost value wins.
        let mut cf = HeaderMap::new();
        cf.insert("CF-Connecting-IP", "203.0.113.7, 10.0.0.1".parse().unwrap());
        assert_eq!(client_ip(&cf), Some("203.0.113.7".parse().unwrap()));
    }
}
