use actix_web::{HttpRequest, http::Error, Responder, HttpResponse};
use jsonwebtoken::{EncodingKey, Header, decode, Validation};
use chrono::{Duration, Utc};
use serde::{Serialize, Deserialize};
use std::env;
use dotenv::dotenv;

#[derive(Debug, serde::Serialize)]
struct JWTClaims {
    sub: String,
    iat: usize,
    exp: usize,
}

pub fn sign_jwt(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    // Load environment variables from .env file
    dotenv().ok();

    // Get secret key from environment variable
    let secret_key = env::var("JSON_SECRET")
        .expect("JSON_SECRET environment variable not set");

    let claims = JWTClaims {
        sub: user_id.to_string(),
        iat: Utc::now().timestamp() as usize,
        exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
    };

    let header = Header::default();
    let encoding_key = EncodingKey::from_secret(secret_key.as_bytes());
    let token = jsonwebtoken::encode(&header, &claims, &encoding_key)?;

    Ok(token)
}
