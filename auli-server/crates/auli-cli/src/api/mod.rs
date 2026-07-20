use std::sync::Arc;

use axum::{
    http::{header, HeaderValue, Method},
    middleware,
    routing::{get, post},
    Router,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tower_http::cors::CorsLayer;

pub mod dto;
mod handlers;
pub mod ratelimit;

use crate::state::AppState;

use handlers::{health_handler, list_handler, question_handler, retrieve_handler};
use ratelimit::SharedLimiter;

// Rota pública sem estado: health-check.
pub fn public_routes() -> Router {
    Router::new().route("/v1/health", get(health_handler))
}

// Rota pública de perguntas (caminho RAG ativo). Precisa do estado compartilhado.
// Protegida por um limitador de taxa por IP (1 req/s, burst 2) — é o único caminho que chama o LLM.
// O limiter vem de FORA (D-MCP-6): é o mesmo objeto usado por `/v1/retrieve`, porque o recurso
// disputado é o embedder, comum às duas rotas.
pub fn question_routes(state: Arc<AppState>, limiter: SharedLimiter) -> Router {
    Router::new()
        .route("/v1/question", post(question_handler))
        .layer(middleware::from_fn_with_state(limiter, ratelimit::rate_limit))
        .with_state(state)
}

// Rota pública de retrieval puro (sem LLM): embeda a pergunta localmente, varre a coleção e
// devolve os documentos com score. CPU-bound no embedder ⇒ divide o MESMO limiter do question
// (D-MCP-6) — `question_rate_limiter()` constrói um limiter novo a cada chamada, então instanciar
// um por rota dobraria a cota efetiva por IP sobre o mesmo recurso.
pub fn retrieve_routes(state: Arc<AppState>, limiter: SharedLimiter) -> Router {
    Router::new()
        .route("/v1/retrieve", post(retrieve_handler))
        .layer(middleware::from_fn_with_state(limiter, ratelimit::rate_limit))
        .with_state(state)
}

// Face MCP: um serviço tower aninhado em /mcp, servido pelo MESMO processo/porta — os dois
// protocolos compartilham o `Arc<Engine>` (e portanto o embedder já carregado). O factory roda
// por sessão e só clona Arcs: barato.
//
// Limiter PRÓPRIO e mais folgado (D-MCP-6, já na v1): o handshake MCP faz várias requisições em
// sequência e quebraria sob a cota do question — mas quota diferente ≠ sem quota.
//
// O layer de CORS do `app()` envolve /mcp também; é inócuo para clientes MCP, que não são
// browsers (o Claude conecta a partir da nuvem da Anthropic, sem header Origin).
pub fn mcp_routes(state: Arc<AppState>) -> Router {
    let engine = state.engine.clone();
    let service = StreamableHttpService::new(
        move || Ok(crate::mcp::AuliMcp::new(engine.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default().with_allowed_hosts(MCP_ALLOWED_HOSTS),
    );
    Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(ratelimit::mcp_rate_limiter(), ratelimit::rate_limit))
}

/// Hosts aceitos no header `Host` do `/mcp` — guarda de **DNS rebinding** do rmcp.
///
/// O default do `StreamableHttpServerConfig` é só loopback (`localhost`/`127.0.0.1`/`::1`), pensado
/// para servidores MCP rodando na máquina do usuário: qualquer outro `Host` é recusado com
/// *"rejected request with disallowed Host header"*. Atrás do tunnel o header chega como
/// `api.auli.com.br`, então **a lista precisa incluir o hostname público** ou o endpoint só
/// funciona em localhost.
///
/// Mantém o loopback: é o que o `scripts/mcp-smoke.sh` e o `claude mcp add ... localhost:3000` usam.
/// Entrada sem porta casa com qualquer porta (o smoke roda em portas alternativas).
///
/// Hardcoded como as origens do CORS logo abaixo — mesma natureza (a identidade pública do
/// serviço) e mesmo lugar para editar ao trocar de domínio.
const MCP_ALLOWED_HOSTS: [&str; 5] =
    ["localhost", "127.0.0.1", "::1", "api.auli.com.br", "auli.com.br"];

// Rota de listagem de dados (somente leitura), genérica por `{kind}`. Pública.
// A ingestão NÃO é uma rota — é o `auli update`.
pub fn data_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/{kind}/list", get(list_handler))
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
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true)
}

#[cfg(test)]
mod tests {
    use super::MCP_ALLOWED_HOSTS;

    /// Regressão: o `/mcp` atrás do tunnel recebe `Host: api.auli.com.br`, e o default do rmcp
    /// (só loopback, guarda de DNS rebinding) o recusa. O smoke de protocolo NÃO pega isso —
    /// roda em localhost, que o default já permite. Este teste é o que sobra para pegar.
    #[test]
    fn mcp_allowed_hosts_inclui_o_hostname_publico() {
        assert!(
            MCP_ALLOWED_HOSTS.contains(&"api.auli.com.br"),
            "sem o hostname público, o /mcp só funciona em localhost: {MCP_ALLOWED_HOSTS:?}"
        );
    }

    /// E o loopback tem que continuar valendo: é o que o `mcp-smoke.sh` e o
    /// `claude mcp add --transport http auli-local http://localhost:3000/mcp` usam.
    #[test]
    fn mcp_allowed_hosts_mantem_o_loopback() {
        for h in ["localhost", "127.0.0.1", "::1"] {
            assert!(MCP_ALLOWED_HOSTS.contains(&h), "loopback '{h}' saiu da lista");
        }
    }
}
