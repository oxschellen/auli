use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http,
    http::{Response, StatusCode},
    middleware::Next,
    response::IntoResponse,
    Extension, Json,
};

use chrono::Local;

use argon2::password_hash::{rand_core::OsRng, PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use bcrypt::verify as verify_bcrypt;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::state::AppState;

use crate::auth::jwt::{decode_jwt, encode_jwt};

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
