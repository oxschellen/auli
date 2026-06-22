use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State}, // FromRef, FromRequestParts, Path
    http,
    http::{Response, StatusCode},
    middleware::Next,
    response::IntoResponse,
    Extension,
    Json, // , Redirect
};

use chrono::Local;

use argon2::password_hash::{rand_core::OsRng, PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use bcrypt::verify as verify_bcrypt;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::state::AppState;

// use chrono::{Duration, Utc};

use crate::auth::jwt::{decode_jwt, encode_jwt};

// use jsonwebtoken::{decode, DecodingKey, EncodingKey, Header, Validation};

// use lettre::{
//     message::{header::ContentType, MultiPart, SinglePart},
//     // transport::smtp::authentication::Credentials,
//     Message,
//     SmtpTransport,
//     Transport,
// };

// use rand::{rng, Rng};

// use sqlx::{postgres::PgPoolOptions, PgPool};
// use std::{env, sync::Arc};

// use tower_http::trace::TraceLayer;

// use crate::user::user_types::{Auth, C, E, R, Reg, User};

// use uuid::Uuid;

// fn tok() -> String {
//     const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
//                             abcdefghijklmnopqrstuvwxyz\
//                             0123456789";
//     (0..64)
//         .map(|_| {
//             let idx = rng().random_range(0..CHARSET.len());
//             CHARSET[idx] as char
//         })
//         .collect()
// }

// fn hash(t: &str) -> String {
//     Argon2::default()
//         .hash_password(t.as_bytes(), &SaltString::generate(&mut OsRng))
//         .unwrap()
//         .to_string()
// }

// fn jwt(id: Uuid, sec: &str, min: i64, typ: &str) -> R<String> {
//     jsonwebtoken::encode(
//         &Header::default(),
//         &C {
//             sub: id,
//             exp: (Utc::now() + Duration::minutes(min)).timestamp() as usize,
//             typ: typ.into(),
//         },
//         &EncodingKey::from_secret(sec.as_bytes()),
//     )
//     .map_err(|e| E::Other(e.into()))
// }

// async fn tokens(id: Uuid, s: &S) -> R<Auth> {
//     sqlx::query("UPDATE refresh_tokens SET revoked=TRUE WHERE user_id=$1 AND revoked=FALSE")
//         .bind(id)
//         .execute(&s.pool)
//         .await?;
//     let rt = tok();
//     let exp = Utc::now() + Duration::days(s.refresh_days);
//     sqlx::query("INSERT INTO refresh_tokens (user_id,token_hash,expires_at) VALUES ($1,$2,$3)")
//         .bind(id)
//         .bind(hash(&rt))
//         .bind(exp)
//         .execute(&s.pool)
//         .await?;
//     Ok(Auth {
//         access_token: jwt(id, &s.secret, s.access_min, "access")?,
//         refresh_token: rt,
//         expires_in: s.access_min * 60,
//     })
// }

pub struct AuthError {
    message: String,
    status_code: StatusCode,
}

impl AuthError {
    fn new(message: impl Into<String>, status_code: StatusCode) -> Self {
        Self {
            message: message.into(),
            status_code,
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response<Body> {
        let body = Json(json!({
            "error": self.message,
        }));

        (self.status_code, body).into_response()
    }
}

// Middleware function to authorize requests
pub async fn auth_middleware(State(state): State<Arc<AppState>>, mut req: Request, next: Next) -> Result<Response<Body>, AuthError> {
    let auth_header = req.headers_mut().get(http::header::AUTHORIZATION);

    let auth_header = match auth_header {
        Some(header) => header
            .to_str()
            .map_err(|_| AuthError::new("Invalid authorization header", StatusCode::FORBIDDEN))?,
        None => return Err(AuthError::new("Please add the JWT token to the header", StatusCode::FORBIDDEN)),
    };

    let mut header = auth_header.split_whitespace();

    let (scheme, token) = (header.next(), header.next());

    if !matches!(scheme, Some(s) if s.eq_ignore_ascii_case("bearer")) || header.next().is_some() {
        return Err(AuthError::new(
            "Authorization header must be `Bearer <token>`",
            StatusCode::UNAUTHORIZED,
        ));
    }

    let token = token.ok_or_else(|| AuthError::new("Token not found in authorization header", StatusCode::UNAUTHORIZED))?;

    let token_data = match decode_jwt(token.to_string()) {
        Ok(data) => data,
        Err(_) => return Err(AuthError::new("Unable to decode token", StatusCode::UNAUTHORIZED)),
    };

    // Fetch the user details from the database
    let current_user = match retrieve_user_by_email(&state.pool, &token_data.claims.email).await {
        Ok(Some(user)) if user.is_verified => CurrentUser { email: user.email },
        Ok(Some(_)) => return Err(AuthError::new("User is not verified", StatusCode::FORBIDDEN)),
        Ok(None) => return Err(AuthError::new("You are not an authorized user", StatusCode::UNAUTHORIZED)),
        Err(_) => {
            return Err(AuthError::new(
                "Unable to validate authenticated user",
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    };

    req.extensions_mut().insert(current_user);
    Ok(next.run(req).await)
}

#[derive(sqlx::FromRow)]
struct DbUser {
    email: String,
    password_hash: String,
    is_verified: bool,
}

async fn retrieve_user_by_email(pool: &sqlx::postgres::PgPool, email: &str) -> Result<Option<DbUser>, sqlx::Error> {
    sqlx::query_as::<_, DbUser>("SELECT email, password_hash, is_verified FROM users WHERE LOWER(email) = LOWER($1)")
        .bind(email)
        .fetch_optional(pool)
        .await
}

fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

fn is_duplicate_email(err: &sqlx::Error) -> bool {
    err.as_database_error().and_then(|db_err| db_err.constraint()) == Some("users_email_key")
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|hash| hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    if hash.starts_with("$2a$") || hash.starts_with("$2b$") || hash.starts_with("$2y$") {
        return verify_bcrypt(password, hash).map_err(|_| AuthError::new("Unable to verify password", StatusCode::INTERNAL_SERVER_ERROR));
    }

    let parsed_hash =
        PasswordHash::new(hash).map_err(|_| AuthError::new("Unable to verify password", StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok())
}

fn validate_registration(email: &str, password: &str) -> Result<(), AuthError> {
    if email.is_empty() || password.is_empty() {
        return Err(AuthError::new("Email and password are required", StatusCode::BAD_REQUEST));
    }

    if !email.contains('@') {
        return Err(AuthError::new("Invalid email address", StatusCode::BAD_REQUEST));
    }

    if password.len() < 8 {
        return Err(AuthError::new(
            "Password must contain at least 8 characters",
            StatusCode::BAD_REQUEST,
        ));
    }

    Ok(())
}

#[derive(Deserialize)]
pub struct UserSignInData {
    pub email: String,
    pub password: String,
}

// Route handler for user sign-in - non protected
pub async fn sign_in_handler(State(state): State<Arc<AppState>>, Json(user_data): Json<UserSignInData>) -> Result<Json<String>, AuthError> {
    let email = normalize_email(&user_data.email);

    let user = retrieve_user_by_email(&state.pool, &email)
        .await
        .map_err(|_| AuthError::new("Unable to retrieve user", StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| AuthError::new("Invalid credentials", StatusCode::UNAUTHORIZED))?;

    if !user.is_verified {
        return Err(AuthError::new("User is not verified", StatusCode::FORBIDDEN));
    }

    if !verify_password(&user_data.password, &user.password_hash)? {
        return Err(AuthError::new("Invalid credentials", StatusCode::UNAUTHORIZED));
    }

    let token = encode_jwt(user.email).map_err(|_| AuthError::new("Unable to generate token", StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(Json(token))
}

#[derive(Clone)]
pub struct CurrentUser {
    pub email: String,
}

#[derive(Serialize, Deserialize)]
struct UserResponse {
    email: String,
}

pub async fn user_get_handler(Extension(current_user): Extension<CurrentUser>) -> impl IntoResponse {
    Json(UserResponse { email: current_user.email })
}

#[derive(Deserialize)]
pub struct UserRegisterInputData {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct UserRegisterResponse {
    pub email: String,
    pub status: String,
}

#[axum::debug_handler]
pub async fn user_register_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UserRegisterInputData>,
) -> Result<impl IntoResponse, AuthError> {
    let local_time = Local::now();
    let local_time = local_time.format("%Y-%m-%d %H:%M:%S");

    println!("--------------------------------------------");
    println!("Registro de novo usuário");
    println!("Local time : {}", local_time);
    println!(" ");

    let email = normalize_email(&req.email);
    validate_registration(&email, &req.password)?;

    let hashed_password =
        hash_password(&req.password).map_err(|_| AuthError::new("Erro ao gerar o hash da senha", StatusCode::INTERNAL_SERVER_ERROR))?;

    sqlx::query("INSERT INTO users (email, password_hash) VALUES ($1, $2)")
        .bind(&email)
        .bind(&hashed_password)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            if is_duplicate_email(&e) {
                AuthError::new("User already exists", StatusCode::CONFLICT)
            } else {
                AuthError::new("Unable to register user", StatusCode::INTERNAL_SERVER_ERROR)
            }
        })?;

    println!("Email          : {}", email);

    let user_register_response = UserRegisterResponse {
        email,
        status: "User registered pending verification".to_string(),
    };

    println!(" ");
    println!("--------------------------------------------\n");

    Ok((StatusCode::CREATED, Json(user_register_response)))
}

// // Handle bcrypt error properly
// let hashed_password = hash_password(&password).map_err(|_| {
//     let error_response = UserRegisterResponse {
//         email: email.clone(),
//         password: "".to_string(), // Don't include password in error response
//         token: "".to_string(),
//     };
//     (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
// })?;

// let hashed_password2 = Argon2::default()
//     .hash_password(&password.as_bytes(), &SaltString::generate(&mut OsRng))?
//     .to_string();

// #[axum::debug_handler]
// pub async fn user_register_tmp_handler(State(s): State<Arc<S>>, Json(p): Json<Reg>) -> R<Json<Auth>> {
//     let h = Argon2::default()
//         .hash_password(p.password.as_bytes(), &SaltString::generate(&mut OsRng))?
//         .to_string();

//     let u: Option<User> = sqlx::query_as(
//         "INSERT INTO users (email,password_hash,is_verified) VALUES ($1,$2,FALSE) RETURNING id,email,password_hash,is_verified",
//     )
//     .bind(&p.email)
//     .bind(&h)
//     .fetch_optional(&s.pool)
//     .await
//     .map_err(|e| {
//         if e.as_database_error().and_then(|d| d.constraint()) == Some("users_email_key") {
//             E::Exists
//         } else {
//             e.into()
//         }
//     })?;
//     let u = u.ok_or_else(|| E::Other(anyhow::anyhow!("insert failed")))?;

//     let vt = tok();
//     let ve = Utc::now() + Duration::hours(s.verify_h);
//     sqlx::query("INSERT INTO verification_tokens (user_id,token_hash,expires_at) VALUES ($1,$2,$3)")
//         .bind(u.id)
//         .bind(hash(&vt))
//         .bind(ve)
//         .execute(&s.pool)
//         .await?;

//     let url = format!("{}/verify/{}", s.base, vt);
//     let msg = Message::builder()
//         .from(s.from.parse().unwrap())
//         .to(p.email.parse().unwrap())
//         .subject("Verify Email")
//         .multipart(
//             MultiPart::alternative().singlepart(SinglePart::plain(url.clone())).singlepart(
//                 SinglePart::builder()
//                     .header(ContentType::TEXT_HTML)
//                     .body(format!("<a href=\"{}\">Verify</a>", url)),
//             ),
//         )?;
//     let smtp_clone = s.smtp.clone();
//     tokio::spawn(async move {
//         let _ = smtp_clone.send(&msg);
//     });

//     tokens(u.id, &s).await.map(Json)
// }
