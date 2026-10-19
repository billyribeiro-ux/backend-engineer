use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
};
use uuid::Uuid;

use crate::{
    auth::jwt::verify_access_token,
    errors::AppError,
    state::AppState,
};

/// Authenticated user, extracted from the JWT in the Authorization header.
/// Using this as a handler parameter automatically requires authentication.
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub organization_id: Option<Uuid>,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract the Authorization header
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("Authentication required. Please log in."))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::unauthorized("Invalid authorization format. Use: Bearer <token>"));
        }

        let token = &auth_header[7..];

        // Verify the JWT
        let claims = verify_access_token(token, &state.config.jwt_access_secret)?;

        // Verify the user still exists and isn't soft-deleted
        let user = sqlx::query!(
            r#"
            SELECT id, email, organization_id
            FROM users
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            claims.user_id
        )
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::unauthorized("Account not found or has been deactivated."))?;

        Ok(AuthUser {
            id: user.id,
            email: user.email,
            role: claims.role,
            organization_id: user.organization_id,
        })
    }
}

/// Optional authentication — doesn't reject if no token is provided.
/// Use for endpoints that behave differently for authenticated vs anonymous users.
pub struct OptionalAuthUser(pub Option<AuthUser>);

#[async_trait]
impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match AuthUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(OptionalAuthUser(Some(user))),
            Err(_) => Ok(OptionalAuthUser(None)),
        }
    }
}
