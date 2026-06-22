use axum::{extract::Json, http::StatusCode, response::IntoResponse};

use serde::{Deserialize, Serialize};

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};

use rsa::pkcs8::LineEnding;
use rsa::{
    pkcs8::{EncodePrivateKey, EncodePublicKey},
    rand_core::OsRng,
    RsaPrivateKey, RsaPublicKey,
};

// JSON Web Token (JWT) - RFC 7519
// JSON Web Algorithms (JWA) - RSA using SHA-256
// https://datatracker.ietf.org/doc/html/rfc7519
// https://www.jwt.io/

use crate::config::config;

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,
    pub iat: usize,
    pub email: String,
}

pub fn encode_jwt(email: String) -> Result<String, StatusCode> {
    // Load RSA private key from static variable loaded from environment on startup
    let private_key_pem = &config().jwt_rsa_private_key;

    let now = Utc::now();
    let expire: chrono::TimeDelta = Duration::minutes(config().jwt_access_expiration_minutes);
    let exp: usize = (now + expire).timestamp() as usize;
    let iat: usize = now.timestamp() as usize;

    let claim = Claims { iat, exp, email };

    // Use RSA-256 algorithm
    let header = Header::new(jsonwebtoken::Algorithm::RS256);

    encode(
        &header,
        &claim,
        &EncodingKey::from_rsa_pem(private_key_pem.as_bytes()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn decode_jwt(jwt: String) -> Result<TokenData<Claims>, StatusCode> {
    // Load RSA public key from static variable loaded from environment on startup
    let public_key_pem = &config().jwt_rsa_public_key;

    let validation = Validation::new(jsonwebtoken::Algorithm::RS256);

    decode(
        &jwt,
        &DecodingKey::from_rsa_pem(public_key_pem.as_bytes()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        &validation,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// Utility function to generate RSA keypair - for testing purposes

// Struct for output of JWT RSA Keypair
#[derive(Serialize)]
pub struct RsaKeyPair {
    pub rsa_private_key: String,
    pub rsa_public_key: String,
}

// Route handler to generate RSA keypair - non protected
pub async fn gen_rsa_keypair_handler() -> Result<impl IntoResponse, (StatusCode, Json<RsaKeyPair>)> {
    let mut rng_os = OsRng;

    // Generate a 2048-bit RSA private key
    let private_key = RsaPrivateKey::new(&mut rng_os, 2048).unwrap();
    let public_key = RsaPublicKey::from(&private_key);

    // Convert keys to PEM format strings
    let rsa_private_key = private_key.to_pkcs8_pem(LineEnding::LF).unwrap().to_string();
    let rsa_public_key = public_key.to_public_key_pem(LineEnding::LF).unwrap().to_string();

    let response_obj = RsaKeyPair {
        rsa_private_key,
        rsa_public_key,
    };

    Ok((StatusCode::OK, Json(response_obj)))
}
