// use axum::response::IntoResponse;
// use axum::http::StatusCode;
// use axum::Json;
// use serde::{Deserialize, Serialize};
// use serde_json;
// use thiserror::Error;
// use uuid::Uuid;

// #[derive(Debug, Error)]
// pub enum E {
//     #[error("Invalid credentials")]
//     Auth,
//     #[error("User exists")]
//     Exists,
//     #[error("Invalid token")]
//     Token,
//     #[error("Not verified")]
//     Unverified,
//     #[error("Server error")]
//     Other(#[from] anyhow::Error),
// }

// impl From<sqlx::Error> for E {
//     fn from(err: sqlx::Error) -> Self {
//         E::Other(err.into())
//     }
// }

// impl From<argon2::password_hash::Error> for E {
//     fn from(err: argon2::password_hash::Error) -> Self {
//         E::Other(anyhow::anyhow!("{}", err))
//     }
// }

// impl From<lettre::error::Error> for E {
//     fn from(err: lettre::error::Error) -> Self {
//         E::Other(err.into())
//     }
// }

// impl IntoResponse for E {
//     fn into_response(self) -> axum::response::Response {
//         match self {
//             E::Auth => (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"Invalid credentials"}))).into_response(),
//             E::Unverified => (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"Verify email first"}))).into_response(),
//             E::Exists => (StatusCode::CONFLICT, Json(serde_json::json!({"error":"User exists"}))).into_response(),
//             _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"Server error"}))).into_response(),
//         }
//     }
// }

// type R<T> = Result<T, E>;

// #[derive(Serialize, Deserialize)]
// pub struct C {
//     sub: Uuid,
//     exp: usize,
//     typ: String,
// }
// #[derive(Deserialize)]
// pub struct UserLogin {
//     email: String,
//     password: String,
// }
// #[derive(Deserialize)]
// pub struct Reg {
//     email: String,
//     password: String,
// }
// #[derive(Deserialize)]
// pub struct Ref {
//     refresh_token: String,
// }
// #[derive(Deserialize)]
// pub struct ResetReq {
//     email: String,
// }
// #[derive(Deserialize)]
// pub struct ResetConf {
//     password: String,
// }
// #[derive(Serialize)]
// pub struct Auth {
//     access_token: String,
//     refresh_token: String,
//     expires_in: i64,
// }
// #[derive(sqlx::FromRow)]
// pub struct User {
//     id: Uuid,
//     email: String,
//     password_hash: String,
//     is_verified: bool,
// }
