use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Local;

use crate::api::dto::{EntityQuery, InputServices, QuestionResponse};
use crate::clients::ingest::load_collection;
use crate::domain::collections::{self, parse_blocks, parse_blocks_from_text};
use crate::domain::entities::get_entity;
use crate::state::AppState;
use crate::util::run_blocking;

// GET /v1/{kind}/list?entity=<id> — list the embeddings stored for one kind of one entity.
#[axum::debug_handler]
pub async fn list_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(kind): Path<String>,
    Query(q): Query<EntityQuery>,
) -> impl IntoResponse {
    let now_total: SystemTime = SystemTime::now();
    log_start(&format!("Listando coleção '{}'", kind), addr);

    let cfg = match get_entity(q.entity.as_deref()) {
        Ok(cfg) => cfg,
        Err(e) => return Json(serde_json::json!({ "status": "Erro", "message": e })),
    };
    let collection = match collections::from_kind(&kind) {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({ "status": "Erro", "message": e })),
    };

    let collection_name = cfg.collection(collection.kind);
    let vector = state.vector.clone();
    let docs = match run_blocking(move || vector.list(&collection_name)).await {
        Ok(docs) => docs,
        Err(e) => return Json(serde_json::json!({ "status": "Erro", "message": e.to_string() })),
    };
    let message = docs
        .iter()
        .enumerate()
        .map(|(i, d)| format!("\n// {}\n{}\n", i + 1, d))
        .collect::<String>();

    log_end(&format!("Listar coleção '{}'", kind), now_total);
    Json(serde_json::json!({ "status": "Ok", "message": message }))
}

// GET /v1/{kind}/load_from_file?entity=<id> — ingest one kind from the entity's source file.
#[axum::debug_handler]
pub async fn load_from_file_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(kind): Path<String>,
    Query(q): Query<EntityQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<QuestionResponse>)> {
    let now_total: SystemTime = SystemTime::now();
    log_start(&format!("Carregando coleção '{}' de arquivo", kind), addr);

    let (cfg, collection) = resolve(q.entity.as_deref(), &kind, &question_label(&kind))?;

    let input_filename = cfg.data_file(collection.file);
    println!("Input filename: {}", input_filename);

    let blocks = match parse_blocks(&input_filename, collection.delimiter) {
        Ok(b) => b,
        Err(e) => {
            println!("Erro na análise do arquivo {}: {}", input_filename, e);
            Vec::new()
        }
    };

    let result = load_collection(state.vector.clone(), state.embedder.clone(), cfg, collection, blocks).await;
    let response = build_response(&question_label(&kind), result);

    log_end(&format!("Carregar coleção '{}' de arquivo", kind), now_total);
    Ok(response)
}

// POST /v1/{kind}/load_from_web — ingest one kind from a web-submitted text body.
#[axum::debug_handler]
pub async fn load_from_web_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(kind): Path<String>,
    Json(req): Json<InputServices>,
) -> Result<impl IntoResponse, (StatusCode, Json<QuestionResponse>)> {
    let now_total: SystemTime = SystemTime::now();
    log_start(&format!("Carregando coleção '{}' da web", kind), addr);

    let (cfg, collection) = resolve(req.entity.as_deref(), &kind, &question_label(&kind))?;

    let blocks = parse_blocks_from_text(&req.text, collection.delimiter);

    let result = load_collection(state.vector.clone(), state.embedder.clone(), cfg, collection, blocks).await;
    let response = build_response(&question_label(&kind), result);

    log_end(&format!("Carregar coleção '{}' da web", kind), now_total);
    Ok(response)
}

// ---- helpers ------------------------------------------------------------

type LoadResult = Result<
    (
        &'static crate::domain::entities::EntityConfig,
        &'static crate::domain::collections::Collection,
    ),
    (StatusCode, Json<QuestionResponse>),
>;

// Resolve entity + collection, mapping either failure to a BAD_REQUEST error response.
fn resolve(entity: Option<&str>, kind: &str, label: &str) -> LoadResult {
    let cfg = get_entity(entity).map_err(|e| err_response(label, e))?;
    let collection = collections::from_kind(kind).map_err(|e| err_response(label, e))?;
    Ok((cfg, collection))
}

fn err_response(label: &str, msg: String) -> (StatusCode, Json<QuestionResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(QuestionResponse {
            status: "Erro".to_string(),
            question: label.to_string(),
            answer: msg,
        }),
    )
}

fn question_label(kind: &str) -> String {
    format!("Carregando coleção '{}' - embeddings", kind)
}

fn build_response(label: &str, result: crate::errors::Result<String>) -> (StatusCode, Json<QuestionResponse>) {
    match result {
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(QuestionResponse {
                status: "Erro".to_string(),
                question: label.to_string(),
                answer: e.to_string(),
            }),
        ),
        Ok(answer) => (
            StatusCode::OK,
            Json(QuestionResponse {
                status: "Ok".to_string(),
                question: label.to_string(),
                answer: if answer.is_empty() {
                    "Nenhum dado foi carregado".to_string()
                } else {
                    answer
                },
            }),
        ),
    }
}

fn log_start(what: &str, addr: SocketAddr) {
    println!("--------------------------------------------");
    println!("--> Início da Rotina {} do servidor.", what);
    println!("IP Origem   : {addr}");
    let local_time = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("Local time : {local_time}");
}

fn log_end(what: &str, started: SystemTime) {
    println!("Fim da Rotina {}.", what);
    let tempo_total = started.elapsed().unwrap().as_millis();
    println!("Tempo total: {tempo_total:6} milisegundos");
    println!("--------------------------------------------\n");
}
