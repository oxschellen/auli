use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::Local;
use serde_json::json;

use auli_core::corpus;

use crate::api::dto::EntityQuery;
use crate::entities::get_entity;
use crate::state::AppState;

// GET /v1/{kind}/list?entity=<id> — list the documents stored for one kind of one entity.
// Read-only: serves from the eager-loaded ReadStore. Ingestion is `auli update`, not a route.
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
        Err(e) => return Json(json!({ "status": "Erro", "message": e })),
    };
    let collection = match corpus::from_kind(&kind) {
        Ok(c) => c,
        Err(e) => return Json(json!({ "status": "Erro", "message": e })),
    };

    let collection_name = cfg.collection(collection.kind);
    let docs = match state.collections.get(&collection_name) {
        Some(store) => store.list(),
        None => {
            return Json(json!({ "status": "Erro", "message": format!("Coleção '{}' não carregada", collection_name) }))
        }
    };
    let message = docs
        .iter()
        .enumerate()
        .map(|(i, d)| format!("\n// {}\n{}\n", i + 1, d))
        .collect::<String>();

    log_end(&format!("Listar coleção '{}'", kind), now_total);
    Json(json!({ "status": "Ok", "message": message }))
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
