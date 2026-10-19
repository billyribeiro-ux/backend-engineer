use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthUser,
    errors::{AppError, AppResult},
    middleware::rbac::require_permission,
    models::subscription::{CancelSubscriptionRequest, CreateSubscriptionRequest},
    policies::{can_cancel_subscription, can_view_subscription},
    state::AppState,
};

pub async fn get_mine(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    let subscription = sqlx::query!(
        r#"
        SELECT
            s.id, s.status, s.trial_ends_at, s.current_period_start,
            s.current_period_end, s.cancel_at_period_end, s.canceled_at,
            p.name as plan_name, p.interval, p.interval_count,
            pr.amount_cents as "amount_cents?", pr.currency as "currency?"
        FROM subscriptions s
        JOIN plans p ON p.id = s.plan_id
        LEFT JOIN prices pr ON pr.id = s.locked_price_id
        WHERE s.user_id = $1
          AND s.status IN ('active', 'trialing', 'past_due')
          AND s.deleted_at IS NULL
        LIMIT 1
        "#,
        auth.id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?;

    match subscription {
        Some(s) => Ok(Json(serde_json::json!({
            "subscription": {
                "id": s.id,
                "status": s.status,
                "plan": {
                    "name": s.plan_name,
                    "interval": s.interval,
                    "interval_count": s.interval_count,
                },
                "price": s.amount_cents.map(|cents| serde_json::json!({
                    "amount_cents": cents,
                    "currency": s.currency,
                })),
                "trial_ends_at": s.trial_ends_at,
                "current_period_start": s.current_period_start,
                "current_period_end": s.current_period_end,
                "cancel_at_period_end": s.cancel_at_period_end,
                "canceled_at": s.canceled_at,
            }
        }))),
        None => Ok(Json(serde_json::json!({ "subscription": null }))),
    }
}

pub async fn get_one(
    auth: AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    // Allow users with read_own (member viewing their own) OR read_all (admin/support)
    let has_own = crate::middleware::rbac::check_permission(&state.db, &auth.role, "subscriptions:read_own").await?;
    let has_all = crate::middleware::rbac::check_permission(&state.db, &auth.role, "subscriptions:read_all").await?;
    if !has_own && !has_all {
        return Err(AppError::forbidden("You do not have permission to view subscriptions."));
    }

    let subscription = sqlx::query_as!(
        crate::models::subscription::Subscription,
        r#"
        SELECT id, user_id, organization_id, plan_id, locked_price_id, stripe_subscription_id,
               status, trial_ends_at, current_period_start, current_period_end,
               cancel_at_period_end, canceled_at, created_at, updated_at, deleted_at
        FROM subscriptions
        WHERE id = $1 AND deleted_at IS NULL
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::not_found("Subscription"))?;

    // ABAC: check if user can view this subscription
    if !can_view_subscription(&auth, &subscription) {
        return Err(AppError::not_found("Subscription")); // Return 404 not 403 — don't confirm existence
    }

    Ok(Json(serde_json::json!({ "subscription": subscription })))
}

pub async fn list_all(
    auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    require_permission(&state.db, &auth, "subscriptions:read_all").await?;

    let subscriptions = sqlx::query!(
        r#"
        SELECT s.id, s.status, s.created_at, u.email, u.display_name, p.name as plan_name
        FROM subscriptions s
        JOIN users u ON u.id = s.user_id
        JOIN plans p ON p.id = s.plan_id
        WHERE s.deleted_at IS NULL
        ORDER BY s.created_at DESC
        LIMIT 100
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::Database)?;

    let result: Vec<serde_json::Value> = subscriptions
        .iter()
        .map(|s| serde_json::json!({
            "id": s.id,
            "status": s.status,
            "created_at": s.created_at,
            "user": {
                "email": s.email,
                "display_name": s.display_name,
            },
            "plan_name": s.plan_name,
        }))
        .collect();

    Ok(Json(serde_json::json!({ "subscriptions": result })))
}

pub async fn create(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateSubscriptionRequest>,
) -> AppResult<(StatusCode, Json<Value>)> {
    // Check for existing active subscription
    let existing = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM subscriptions
            WHERE user_id = $1
              AND status IN ('active', 'trialing', 'past_due')
              AND deleted_at IS NULL
        ) as "exists!"
        "#,
        auth.id
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::Database)?;

    if existing {
        return Err(AppError::conflict(
            "SUBSCRIPTION_EXISTS",
            "You already have an active subscription.",
        ));
    }

    // Get plan and current price
    let plan_price = sqlx::query!(
        r#"
        SELECT p.id as plan_id, p.name, p.interval, p.interval_count,
               pr.id as price_id, pr.amount_cents, pr.stripe_price_id
        FROM plans p
        JOIN prices pr ON pr.plan_id = p.id AND pr.is_current = true
        WHERE p.id = $1 AND p.deleted_at IS NULL AND p.is_active = true
        "#,
        payload.plan_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::not_found("Plan"))?;

    // NOTE: In a full implementation, you would:
    // 1. Create Stripe subscription (requires Stripe API call)
    // 2. Store locally with locked_price_id = plan_price.price_id
    // For this example, we show the local DB portion:

    let stripe_subscription_id = format!("sub_placeholder_{}", uuid::Uuid::new_v4());

    let use_trial = state.config.feature_trials_enabled;
    let trial_days = state.config.trial_days;

    let subscription = if use_trial {
        sqlx::query!(
            r#"
            INSERT INTO subscriptions (
                id, user_id, plan_id, locked_price_id, stripe_subscription_id,
                status, trial_ends_at, current_period_start, current_period_end
            )
            VALUES (
                uuid_generate_v4(), $1, $2, $3, $4, 'trialing',
                NOW() + ($5 * INTERVAL '1 day'),
                NOW(), NOW() + ($5 * INTERVAL '1 day')
            )
            RETURNING id, status, created_at
            "#,
            auth.id,
            plan_price.plan_id,
            plan_price.price_id,
            stripe_subscription_id,
            trial_days as f64,
        )
        .fetch_one(&state.db)
        .await
        .map_err(AppError::Database)?
    } else {
        sqlx::query!(
            r#"
            INSERT INTO subscriptions (
                id, user_id, plan_id, locked_price_id, stripe_subscription_id,
                status, current_period_start, current_period_end
            )
            VALUES (uuid_generate_v4(), $1, $2, $3, $4, 'active', NOW(), NOW() + INTERVAL '1 month')
            RETURNING id, status, created_at
            "#,
            auth.id,
            plan_price.plan_id,
            plan_price.price_id,
            stripe_subscription_id,
        )
        .fetch_one(&state.db)
        .await
        .map_err(AppError::Database)?
    };

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "subscription": {
                "id": subscription.id,
                "status": subscription.status,
                "created_at": subscription.created_at,
            }
        })),
    ))
}

pub async fn cancel_mine(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CancelSubscriptionRequest>,
) -> AppResult<Json<Value>> {
    let subscription = sqlx::query_as!(
        crate::models::subscription::Subscription,
        r#"
        SELECT id, user_id, organization_id, plan_id, locked_price_id, stripe_subscription_id,
               status, trial_ends_at, current_period_start, current_period_end,
               cancel_at_period_end, canceled_at, created_at, updated_at, deleted_at
        FROM subscriptions
        WHERE user_id = $1
          AND status IN ('active', 'trialing', 'past_due')
          AND deleted_at IS NULL
        LIMIT 1
        "#,
        auth.id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::not_found("Subscription"))?;

    // ABAC policy check
    if !can_cancel_subscription(&auth, &subscription) {
        if subscription.is_canceled() {
            return Err(AppError::conflict("ALREADY_CANCELED", "This subscription is already canceled."));
        }
        return Err(AppError::forbidden("You cannot cancel this subscription."));
    }

    let immediately = payload.immediately.unwrap_or(false);

    let updated = if immediately {
        sqlx::query!(
            r#"
            UPDATE subscriptions
            SET status = 'canceled', canceled_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING id, status, canceled_at
            "#,
            subscription.id
        )
        .fetch_one(&state.db)
        .await
        .map_err(AppError::Database)?
    } else {
        sqlx::query!(
            r#"
            UPDATE subscriptions
            SET cancel_at_period_end = true, updated_at = NOW()
            WHERE id = $1
            RETURNING id, status, canceled_at
            "#,
            subscription.id
        )
        .fetch_one(&state.db)
        .await
        .map_err(AppError::Database)?
    };

    // Audit log
    sqlx::query!(
        r#"
        INSERT INTO audit_logs (id, actor_id, actor_type, action, resource_type, resource_id, metadata)
        VALUES (uuid_generate_v4(), $1, 'user', 'subscription.canceled', 'subscription', $2, $3)
        "#,
        auth.id,
        subscription.id,
        serde_json::json!({ "immediately": immediately })
    )
    .execute(&state.db)
    .await
    .map_err(AppError::Database)?;

    Ok(Json(serde_json::json!({
        "subscription": {
            "id": updated.id,
            "status": updated.status,
            "canceled_at": updated.canceled_at,
        }
    })))
}
