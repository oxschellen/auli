//! Teste end-to-end da rota `POST /v1/retrieve` contra packs REAIS.
//!
//! Os contratos de erro (kind/entidade inválidos, tetos de `top_k`) são cobertos por testes
//! unitários de `validar_kind_top_k`/`validar_retrieve` no próprio handler — puros, sem modelo e
//! sem registry. Aqui fica só o fluxo feliz, que exige o BGE-M3 e os packs gerados, e por isso
//! segue a convenção do workspace (D-MCP-4 / `packs_smoke.rs`): `#[ignore]` + gate por env.
//!
//! Rodar com (os caminhos são relativos ao CWD do binário de teste, que é o diretório DESTE
//! crate — daí os três níveis até a raiz do repo, onde moram `data/` e `models/`):
//!   AULI_DATA_DIR=../../../data EMBED_CACHE_DIR=../../../models cargo test -p auli-cli --release \
//!     --test retrieve_api -- --nocapture --ignored
//!
//! Verificado nesta forma em 2026-07-20: 2 passed (sc/pareceres e rs/faqs contra packs reais).
//!
//! O `AULI_DATA_DIR` precisa vir de FORA (não é setado aqui): `entities::ENTITIES` é um
//! `LazyLock` lido no primeiro acesso, então mexer no ambiente dentro do teste seria uma corrida.

use std::net::SocketAddr;
use std::sync::Arc;

use auli_cli::api::retrieve_routes;
use auli_cli::state::AppState;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use tower::ServiceExt; // for `oneshot`

fn data_dir() -> Option<String> {
    std::env::var("AULI_DATA_DIR").ok()
}

/// Monta o `AppState` real: packs carregados do disco + embedder + raiz da árvore `docs/`.
fn estado_real(data: &str) -> Arc<AppState> {
    let collections = auli_cli::packs::load_all(data).expect("carregar packs");
    let cache = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "./models".into());
    let embedder = Arc::new(auli_core::embed::Embedder::new(cache.into(), 16).expect("carregar embedder"));
    let engine =
        Arc::new(auli_retrieval::Engine::new(collections, embedder, std::path::PathBuf::from(data)));
    let anonimizador = Arc::new(auli_anon::Anonimizador::novo().expect("carregar anonimizador"));
    Arc::new(AppState { engine, anonimizador })
}

/// POST com `ConnectInfo` injetado — o middleware de rate limit extrai o IP do socket, e sem essa
/// extensão o extractor falharia antes de chegar ao handler.
fn post(uri: &str, corpo: serde_json::Value) -> Request<Body> {
    let mut req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(corpo.to_string()))
        .unwrap();
    req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 40000))));
    req
}

#[tokio::test]
#[ignore = "carrega o modelo BGE-M3 e os packs reais (lento); rode com --ignored e AULI_DATA_DIR"]
async fn retrieve_de_pareceres_devolve_metadados_com_score_e_sem_corpo() {
    let Some(data) = data_dir() else {
        eprintln!("AULI_DATA_DIR não setado — pulando");
        return;
    };

    let state = estado_real(&data);
    let limiter = auli_cli::api::ratelimit::question_rate_limiter();
    let app = retrieve_routes(state, limiter);

    let resp = app
        .oneshot(post(
            "/v1/retrieve",
            serde_json::json!({
                "question": "crédito de ICMS na aquisição de energia elétrica",
                "entity": "sc",
                "kind": "pareceres",
                "top_k": 5
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["entity"], "sc");
    assert_eq!(json["kind"], "pareceres");

    let pareceres = json["pareceres"].as_array().expect("vetor pareceres presente");
    assert!(!pareceres.is_empty(), "a busca deveria achar pareceres em sc");
    assert!(pareceres.len() <= 5, "top_k=5 é teto: veio {}", pareceres.len());

    // O contrato da rota: metadados + score, NUNCA o corpo (que é o que `obter_parecer` serve).
    for p in pareceres {
        assert!(p["numero"].is_string(), "numero presente: {p}");
        assert!(p["assunto"].is_string(), "assunto presente: {p}");
        assert!(p["link"].is_string(), "link presente: {p}");
        assert!(p["score"].is_number(), "score presente: {p}");
        assert!(p.get("corpo").is_none(), "corpo NUNCA vai na busca: {p}");
    }

    // Distância cosseno: ordenada best-first e não-negativa.
    let scores: Vec<f64> = pareceres.iter().map(|p| p["score"].as_f64().unwrap()).collect();
    assert!(scores.windows(2).all(|w| w[0] <= w[1]), "best-first: {scores:?}");
    assert!(scores[0] >= 0.0, "distância cosseno é não-negativa");

    // O vetor do outro kind vem vazio, mas PRESENTE (contrato do RetrieveResponse).
    assert_eq!(json["hits"], serde_json::json!([]));

    eprintln!("top-{} scores: {scores:?}", scores.len());
}

#[tokio::test]
#[ignore = "carrega o modelo BGE-M3 e os packs reais (lento); rode com --ignored e AULI_DATA_DIR"]
async fn retrieve_de_kind_generico_devolve_hits_e_pareceres_vazio() {
    let Some(data) = data_dir() else {
        eprintln!("AULI_DATA_DIR não setado — pulando");
        return;
    };

    let state = estado_real(&data);
    let limiter = auli_cli::api::ratelimit::question_rate_limiter();
    let app = retrieve_routes(state, limiter);

    let resp = app
        .oneshot(post(
            "/v1/retrieve",
            serde_json::json!({
                "question": "certidão negativa de débitos",
                "entity": "rs",
                "kind": "faqs",
                "top_k": 3
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["kind"], "faqs");
    let hits = json["hits"].as_array().expect("vetor hits presente");
    assert!(!hits.is_empty(), "a busca deveria achar faqs em rs");
    assert!(hits[0]["texto"].is_string());
    assert!(hits[0]["score"].is_number());
    // Kind genérico não preenche `pareceres` — mas o campo existe.
    assert_eq!(json["pareceres"], serde_json::json!([]));
}
