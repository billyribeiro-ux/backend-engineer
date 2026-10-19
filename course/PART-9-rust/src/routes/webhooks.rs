use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde_json::Value;

use crate::{errors::AppError, state::AppState};

type HmacSha256 = Hmac<Sha256>;

/// Verify Stripe webhook signature.
/// Stripe signs requests with HMAC-SHA256 using your webhook secret.
fn verify_stripe_signature(
    payload: &[u8],
    signature_header: &str,
    secret: &str,
) -> Result<(), AppError> {
    // Parse the Stripe-Signature header: "t=timestamp,v1=signature1,v1=signature2"
    let mut timestamp = None;
    let mut signatures: Vec<&str> = Vec::new();

    for part in signature_header.split(',') {
        if let Some(ts) = part.strip_prefix("t=") {
            timestamp = Some(ts);
        } else if let Some(sig) = part.strip_prefix("v1=") {
            signatures.push(sig);
        }
    }

    let timestamp = timestamp.ok_or_else(|| {
        AppError::bad_request("INVALID_SIGNATURE", "Missing timestamp in Stripe signature.")
    })?;

    if signatures.is_empty() {
        return Err(AppError::bad_request("INVALID_SIGNATURE", "Missing signatures in Stripe header."));
    }

    // Construct the signed payload: "timestamp.payload"
    let signed_payload = format!("{}.{}", timestamp, std::str::from_utf8(payload).unwrap_or(""));

    // Compute the expected HMAC
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| AppError::bad_request("SIGNATURE_ERROR", "Invalid webhook secret."))?;
    mac.update(signed_payload.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    // Compare (constant-time comparison to prevent timing attacks)
    let verified = signatures
        .iter()
        .any(|sig| constant_time_eq::constant_time_eq(sig.as_bytes(), expected.as_bytes()));

    if !verified {
        return Err(AppError::bad_request("INVALID_SIGNATURE", "Webhook signature verification failed."));
    }

    Ok(())
}

pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 1. Verify Stripe signature
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Err(_) = verify_stripe_signature(&body, signature, &state.config.stripe_webhook_secret) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid signature" })),
        ));
    }

    // 2. Parse the event
    let event: Value = serde_json::from_slice(&body).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid JSON" })),
        )
    })?;

    let event_id = event["id"].as_str().unwrap_or("").to_string();
    let event_type = event["type"].as_str().unwrap_or("").to_string();

    // 3. Respond immediately to Stripe (process asynchronously)
    // In production, push to a queue. For this example, process inline.
    let state_clone = state.clone();
    let event_clone = event.clone();
    let event_id_clone = event_id.clone();
    let event_type_clone = event_type.clone();

    tokio::spawn(async move {
        if let Err(e) = process_event(state_clone, event_id_clone, event_type_clone, event_clone).await {
            tracing::error!("Webhook processing error: {:?}", e);
        }
    });

    Ok(Json(serde_json::json!({ "received": true })))
}

async fn process_event(
    state: AppState,
    event_id: String,
    event_type: String,
    payload: Value,
) -> Result<(), anyhow::Error> {
    // Check idempotency: has this event been processed?
    let existing = sqlx::query!(
        "SELECT id, processed_at FROM webhook_events WHERE stripe_event_id = $1",
        event_id
    )
    .fetch_optional(&state.db)
    .await?;

    if let Some(e) = &existing {
        if e.processed_at.is_some() {
            tracing::info!("Webhook {} already processed, skipping", event_id);
            return Ok(());
        }
    }

    // Insert event record if not exists
    if existing.is_none() {
        sqlx::query!(
            r#"
            INSERT INTO webhook_events (id, stripe_event_id, event_type, payload)
            VALUES (uuid_generate_v4(), $1, $2, $3)
            ON CONFLICT (stripe_event_id) DO NOTHING
            "#,
            event_id,
            event_type,
            payload
        )
        .execute(&state.db)
        .await?;
    }

    // Route to the correct handler
    let result = match event_type.as_str() {
        "customer.subscription.updated" => {
            on_subscription_updated(&state, &payload["data"]["object"]).await
        }
        "customer.subscription.deleted" => {
            on_subscription_deleted(&state, &payload["data"]["object"]).await
        }
        "invoice.paid" => {
            on_invoice_paid(&state, &payload["data"]["object"]).await
        }
        "invoice.payment_failed" => {
            on_invoice_payment_failed(&state, &payload["data"]["object"]).await
        }
        _ => {
            tracing::debug!("Unhandled webhook event type: {}", event_type);
            Ok(())
        }
    };

    // Mark as processed (or record error)
    match result {
        Ok(()) => {
            sqlx::query!(
                "UPDATE webhook_events SET processed_at = NOW() WHERE stripe_event_id = $1",
                event_id
            )
            .execute(&state.db)
            .await?;
        }
        Err(e) => {
            sqlx::query!(
                "UPDATE webhook_events SET error = $1 WHERE stripe_event_id = $2",
                e.to_string(),
                event_id
            )
            .execute(&state.db)
            .await?;
            return Err(e);
        }
    }

    Ok(())
}

async fn on_subscription_updated(state: &AppState, subscription: &Value) -> Result<(), anyhow::Error> {
    let stripe_sub_id = subscription["id"].as_str().unwrap_or("");
    let status = subscription["status"].as_str().unwrap_or("");
    let cancel_at_period_end = subscription["cancel_at_period_end"].as_bool().unwrap_or(false);

    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = $1, cancel_at_period_end = $2, updated_at = NOW()
        WHERE stripe_subscription_id = $3
        "#,
        status,
        cancel_at_period_end,
        stripe_sub_id
    )
    .execute(&state.db)
    .await?;

    tracing::info!("Subscription {} updated to status: {}", stripe_sub_id, status);
    Ok(())
}

async fn on_subscription_deleted(state: &AppState, subscription: &Value) -> Result<(), anyhow::Error> {
    let stripe_sub_id = subscription["id"].as_str().unwrap_or("");

    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = 'canceled', canceled_at = NOW(), updated_at = NOW()
        WHERE stripe_subscription_id = $1
        "#,
        stripe_sub_id
    )
    .execute(&state.db)
    .await?;

    tracing::info!("Subscription {} deleted/canceled", stripe_sub_id);
    Ok(())
}

async fn on_invoice_paid(state: &AppState, invoice: &Value) -> Result<(), anyhow::Error> {
    let stripe_sub_id = invoice["subscription"].as_str().unwrap_or("");
    if stripe_sub_id.is_empty() {
        return Ok(()); // One-time purchase, not a subscription
    }

    sqlx::query!(
        "UPDATE subscriptions SET status = 'active', updated_at = NOW() WHERE stripe_subscription_id = $1",
        stripe_sub_id
    )
    .execute(&state.db)
    .await?;

    tracing::info!("Subscription {} marked active after invoice paid", stripe_sub_id);
    Ok(())
}

async fn on_invoice_payment_failed(state: &AppState, invoice: &Value) -> Result<(), anyhow::Error> {
    let stripe_sub_id = invoice["subscription"].as_str().unwrap_or("");
    if stripe_sub_id.is_empty() {
        return Ok(());
    }

    sqlx::query!(
        "UPDATE subscriptions SET status = 'past_due', updated_at = NOW() WHERE stripe_subscription_id = $1",
        stripe_sub_id
    )
    .execute(&state.db)
    .await?;

    tracing::warn!("Invoice payment failed for subscription {}", stripe_sub_id);
    Ok(())
}
