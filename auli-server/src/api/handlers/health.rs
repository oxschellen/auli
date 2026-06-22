use std::net::SocketAddr;
use std::time::SystemTime;

use axum::{extract::ConnectInfo, response::IntoResponse, Json};
use chrono::Local;

#[axum::debug_handler]
pub async fn health_handler(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    let now_total: SystemTime = SystemTime::now();

    println!("--------------------------------------------");
    println!("--> Início da Rotina Health do servidor");
    println!("IP Origem   : {addr}");

    let local_time = Local::now();
    let local_time = local_time.format("%Y-%m-%d %H:%M:%S");
    println!("Local time : {local_time}");

    let message = format!("O servidor Auli está online: {local_time}");

    let json_response = serde_json::json!({
        "status": "Ok",
        "message": message,
    });

    println!("Mensagem : {message}");
    println!("Fim da Rotina Health  do servidor");

    let tempo_total = now_total.elapsed().unwrap().as_nanos();
    println!("Tempo total: {tempo_total:6} nanosegundos");
    println!("--------------------------------------------\n");

    Json(json_response)
}
