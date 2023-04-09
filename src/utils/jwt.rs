use jsonwebtoken::{EncodingKey, Header};
use chrono::{Duration, Utc};

#[derive(Debug, serde::Serialize)]
struct JWTClaims {
    sub: String,
    iat: usize,
    exp: usize,
}

pub fn sign_jwt(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = JWTClaims {
        sub: user_id.to_string(),
        iat: Utc::now().timestamp() as usize,
        exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
    };

    let secret = "secret".as_ref();
    let header = Header::default();
    let encoding_key = EncodingKey::from_secret(secret);
    let token = jsonwebtoken::encode(&header, &claims, &encoding_key)?;

    Ok(token)
}
