use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessTokenClaims {
    pub sub: String,           // user_id as string
    pub user_id: Uuid,
    pub role: String,
    pub organization_id: Option<Uuid>,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefreshTokenClaims {
    pub sub: String,
    pub user_id: Uuid,
    pub exp: i64,
    pub iat: i64,
}

pub fn generate_access_token(
    user_id: Uuid,
    role: &str,
    organization_id: Option<Uuid>,
    secret: &str,
    expires_in_secs: u64,
) -> Result<String, AppError> {
    let now = Utc::now();
    let exp = (now + Duration::seconds(expires_in_secs as i64)).timestamp();

    let claims = AccessTokenClaims {
        sub: user_id.to_string(),
        user_id,
        role: role.to_string(),
        organization_id,
        exp,
        iat: now.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(AppError::Jwt)
}

pub fn generate_refresh_token(
    user_id: Uuid,
    secret: &str,
    expires_in_secs: u64,
) -> Result<String, AppError> {
    let now = Utc::now();
    let exp = (now + Duration::seconds(expires_in_secs as i64)).timestamp();

    let claims = RefreshTokenClaims {
        sub: user_id.to_string(),
        user_id,
        exp,
        iat: now.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(AppError::Jwt)
}

pub fn verify_access_token(token: &str, secret: &str) -> Result<AccessTokenClaims, AppError> {
    let token_data = decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
            AppError::unauthorized("Your session has expired. Please log in again.")
        }
        _ => AppError::unauthorized("Invalid authentication token."),
    })?;

    Ok(token_data.claims)
}

pub fn verify_refresh_token(token: &str, secret: &str) -> Result<RefreshTokenClaims, AppError> {
    let token_data = decode::<RefreshTokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::unauthorized("Invalid or expired refresh token."))?;

    Ok(token_data.claims)
}
