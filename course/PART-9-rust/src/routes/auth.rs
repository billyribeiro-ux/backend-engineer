use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::{
    auth::{
        extractor::AuthUser,
        jwt::{generate_access_token, generate_refresh_token, verify_refresh_token},
        password::{hash_password, verify_password},
    },
    errors::{AppError, AppResult},
    models::user::{AuthResponse, LoginRequest, RegisterRequest, UserResponse},
    state::AppState,
};

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    // Validate input
    if payload.email.is_empty() || payload.password.is_empty() {
        return Err(AppError::bad_request("MISSING_FIELDS", "Email and password are required."));
    }
    if payload.password.len() < 8 {
        return Err(AppError::bad_request_field(
            "PASSWORD_TOO_SHORT",
            "Password must be at least 8 characters.",
            "password",
        ));
    }

    let email = payload.email.to_lowercase();

    // Check for duplicate email
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1 AND deleted_at IS NULL) as \"exists!\"",
        email
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::Database)?;

    if exists {
        return Err(AppError::conflict("EMAIL_EXISTS", "An account with this email already exists."));
    }

    let password_hash = hash_password(&payload.password)?;

    // Create user
    let user = sqlx::query!(
        r#"
        INSERT INTO users (id, email, password_hash, display_name)
        VALUES (uuid_generate_v4(), $1, $2, $3)
        RETURNING id, email, display_name, organization_id, created_at
        "#,
        email,
        password_hash,
        payload.display_name
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::Database)?;

    let access_token = generate_access_token(
        user.id,
        "member",
        user.organization_id,
        &state.config.jwt_access_secret,
        state.config.jwt_access_expires_in_secs,
    )?;

    let refresh_token = generate_refresh_token(
        user.id,
        &state.config.jwt_refresh_secret,
        state.config.jwt_refresh_expires_in_secs,
    )?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            user: UserResponse {
                id: user.id,
                email: user.email,
                display_name: user.display_name,
                role: "member".to_string(),
                organization_id: user.organization_id,
                created_at: user.created_at,
            },
            access_token,
            refresh_token,
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let email = payload.email.to_lowercase();

    // Find user (same error for invalid email or password — prevents user enumeration)
    let user = sqlx::query!(
        r#"
        SELECT u.id, u.email, u.password_hash, u.display_name, u.role, u.organization_id, u.created_at
        FROM users u
        WHERE u.email = $1 AND u.deleted_at IS NULL
        "#,
        email
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?;

    let invalid_creds_err = AppError::unauthorized("Invalid email or password.");

    let user = match user {
        Some(u) => u,
        None => return Err(invalid_creds_err),
    };

    // Verify password
    if !verify_password(&payload.password, &user.password_hash)? {
        return Err(invalid_creds_err);
    }

    // Update last_login_at
    sqlx::query!(
        "UPDATE users SET last_login_at = NOW() WHERE id = $1",
        user.id
    )
    .execute(&state.db)
    .await
    .map_err(AppError::Database)?;

    let access_token = generate_access_token(
        user.id,
        &user.role,
        user.organization_id,
        &state.config.jwt_access_secret,
        state.config.jwt_access_expires_in_secs,
    )?;

    let refresh_token = generate_refresh_token(
        user.id,
        &state.config.jwt_refresh_secret,
        state.config.jwt_refresh_expires_in_secs,
    )?;

    Ok(Json(AuthResponse {
        user: UserResponse {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            role: user.role,
            organization_id: user.organization_id,
            created_at: user.created_at,
        },
        access_token,
        refresh_token,
    }))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> AppResult<Json<Value>> {
    let refresh_token = payload["refresh_token"]
        .as_str()
        .ok_or_else(|| AppError::bad_request("MISSING_TOKEN", "refresh_token is required."))?
        .to_string();

    let claims = verify_refresh_token(&refresh_token, &state.config.jwt_refresh_secret)?;

    // Get user and current role
    let user = sqlx::query!(
        "SELECT id, role, organization_id FROM users WHERE id = $1 AND deleted_at IS NULL",
        claims.user_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::unauthorized("Account not found."))?;

    let new_access_token = generate_access_token(
        user.id,
        &user.role,
        user.organization_id,
        &state.config.jwt_access_secret,
        state.config.jwt_access_expires_in_secs,
    )?;

    Ok(Json(json!({ "access_token": new_access_token })))
}

pub async fn logout(
    _auth: AuthUser,
) -> AppResult<StatusCode> {
    // In a full implementation: invalidate refresh token in DB
    // For now: client clears the cookie
    Ok(StatusCode::NO_CONTENT)
}
