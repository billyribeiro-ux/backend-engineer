use axum::{extract::State, Json};
use serde_json::Value;

use crate::{
    auth::extractor::AuthUser,
    errors::{AppError, AppResult},
    models::user::{UpdateProfileRequest, UserResponse},
    state::AppState,
};

pub async fn get_me(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    let user = sqlx::query!(
        r#"
        SELECT u.id, u.email, u.display_name, u.organization_id, u.created_at
        FROM users u
        WHERE u.id = $1 AND u.deleted_at IS NULL
        "#,
        auth.id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::not_found("User"))?;

    // Get subscription status
    let subscription = sqlx::query!(
        r#"
        SELECT status, current_period_end, trial_ends_at
        FROM subscriptions
        WHERE user_id = $1 AND deleted_at IS NULL
          AND status IN ('active', 'trialing', 'past_due')
        LIMIT 1
        "#,
        auth.id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?;

    Ok(Json(serde_json::json!({
        "id": user.id,
        "email": user.email,
        "display_name": user.display_name,
        "role": auth.role,
        "organization_id": user.organization_id,
        "created_at": user.created_at,
        "subscription": subscription.map(|s| serde_json::json!({
            "status": s.status,
            "current_period_end": s.current_period_end,
            "trial_ends_at": s.trial_ends_at,
        })),
    })))
}

pub async fn update_me(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateProfileRequest>,
) -> AppResult<Json<UserResponse>> {
    // ABAC: users can only update their own profile (enforced by using auth.id as the target)
    let user = sqlx::query!(
        r#"
        UPDATE users
        SET
            display_name = COALESCE($1, display_name),
            updated_at = NOW()
        WHERE id = $2 AND deleted_at IS NULL
        RETURNING id, email, display_name, organization_id, created_at
        "#,
        payload.display_name,
        auth.id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::not_found("User"))?;

    Ok(Json(UserResponse {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        role: auth.role,
        organization_id: user.organization_id,
        created_at: user.created_at,
    }))
}
