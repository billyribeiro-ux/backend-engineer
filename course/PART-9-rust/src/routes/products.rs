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
    models::product::{AddPriceRequest, CreateProductRequest},
    state::AppState,
};

pub async fn list(State(state): State<AppState>) -> AppResult<Json<Value>> {
    let products = sqlx::query!(
        r#"
        SELECT
            p.id, p.name, p.description, p.is_active,
            pr.id as "price_id?", pr.amount_cents as "amount_cents?", pr.currency as "currency?"
        FROM products p
        LEFT JOIN prices pr ON pr.product_id = p.id AND pr.is_current = true
        WHERE p.deleted_at IS NULL AND p.is_active = true
        ORDER BY p.created_at ASC
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::Database)?;

    let result: Vec<Value> = products
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "description": p.description,
                "is_active": p.is_active,
                "current_price": p.price_id.map(|id| serde_json::json!({
                    "id": id,
                    "amount_cents": p.amount_cents,
                    "currency": p.currency,
                })),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "products": result })))
}

pub async fn get_one(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    let product = sqlx::query!(
        r#"
        SELECT p.id, p.name, p.description, p.is_active,
               pr.amount_cents as "amount_cents?", pr.currency as "currency?"
        FROM products p
        LEFT JOIN prices pr ON pr.product_id = p.id AND pr.is_current = true
        WHERE p.id = $1 AND p.deleted_at IS NULL
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::not_found("Product"))?;

    Ok(Json(serde_json::json!({
        "id": product.id,
        "name": product.name,
        "description": product.description,
        "is_active": product.is_active,
        "current_price": product.amount_cents.map(|a| serde_json::json!({
            "amount_cents": a,
            "currency": product.currency,
        })),
    })))
}

pub async fn create(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateProductRequest>,
) -> AppResult<(StatusCode, Json<Value>)> {
    require_permission(&state.db, &auth, "products:write").await?;

    if payload.name.is_empty() {
        return Err(AppError::bad_request_field("MISSING_NAME", "Product name is required.", "name"));
    }
    if payload.amount_cents < 0 {
        return Err(AppError::bad_request("INVALID_PRICE", "Price cannot be negative."));
    }

    let currency = payload.currency.unwrap_or_else(|| "usd".to_string());

    let mut tx = state.db.begin().await.map_err(AppError::Database)?;

    let product = sqlx::query!(
        r#"
        INSERT INTO products (id, name, description)
        VALUES (uuid_generate_v4(), $1, $2)
        RETURNING id, name, description, is_active, created_at
        "#,
        payload.name,
        payload.description
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    let price = sqlx::query!(
        r#"
        INSERT INTO prices (id, product_id, amount_cents, currency, applies_to, is_current)
        VALUES (uuid_generate_v4(), $1, $2, $3, 'future_members', true)
        RETURNING id, amount_cents, currency
        "#,
        product.id,
        payload.amount_cents,
        currency
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // Audit log
    sqlx::query!(
        r#"
        INSERT INTO audit_logs (id, actor_id, actor_type, action, resource_type, resource_id, metadata)
        VALUES (uuid_generate_v4(), $1, 'user', 'product.created', 'product', $2, $3)
        "#,
        auth.id,
        product.id,
        serde_json::json!({ "name": payload.name, "amount_cents": payload.amount_cents })
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "product": {
                "id": product.id,
                "name": product.name,
                "description": product.description,
                "is_active": product.is_active,
                "created_at": product.created_at,
            },
            "price": {
                "id": price.id,
                "amount_cents": price.amount_cents,
                "currency": price.currency,
            }
        })),
    ))
}

pub async fn update(
    auth: AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> AppResult<Json<Value>> {
    require_permission(&state.db, &auth, "products:write").await?;

    let product = sqlx::query!(
        r#"
        UPDATE products
        SET
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            is_active = COALESCE($3, is_active),
            updated_at = NOW()
        WHERE id = $4 AND deleted_at IS NULL
        RETURNING id, name, description, is_active
        "#,
        payload["name"].as_str(),
        payload["description"].as_str(),
        payload["is_active"].as_bool(),
        id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::not_found("Product"))?;

    Ok(Json(serde_json::json!({
        "product": {
            "id": product.id,
            "name": product.name,
            "description": product.description,
            "is_active": product.is_active,
        }
    })))
}

pub async fn add_price(
    auth: AuthUser,
    Path(product_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<AddPriceRequest>,
) -> AppResult<(StatusCode, Json<Value>)> {
    require_permission(&state.db, &auth, "prices:write").await?;

    let valid_applies_to = ["future_members", "all_members", "from_date"];
    if !valid_applies_to.contains(&payload.applies_to.as_str()) {
        return Err(AppError::bad_request(
            "INVALID_APPLIES_TO",
            "applies_to must be one of: future_members, all_members, from_date",
        ));
    }
    if payload.applies_to == "from_date" && payload.applies_from_date.is_none() {
        return Err(AppError::bad_request(
            "MISSING_DATE",
            "applies_from_date is required when applies_to is from_date.",
        ));
    }

    // Verify product exists
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM products WHERE id = $1 AND deleted_at IS NULL) as \"exists!\"",
        product_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(AppError::Database)?;

    if !exists {
        return Err(AppError::not_found("Product"));
    }

    let currency = payload.currency.clone().unwrap_or_else(|| "usd".to_string());

    let mut tx = state.db.begin().await.map_err(AppError::Database)?;

    // Mark old price as not current
    sqlx::query!(
        "UPDATE prices SET is_current = false WHERE product_id = $1 AND is_current = true",
        product_id
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // Insert new price
    let new_price = sqlx::query!(
        r#"
        INSERT INTO prices (id, product_id, amount_cents, currency, applies_to, applies_from_date, is_current)
        VALUES (uuid_generate_v4(), $1, $2, $3, $4, $5, true)
        RETURNING id, amount_cents, currency, applies_to, created_at
        "#,
        product_id,
        payload.amount_cents,
        currency,
        payload.applies_to,
        payload.applies_from_date
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // Audit log
    sqlx::query!(
        r#"
        INSERT INTO audit_logs (id, actor_id, actor_type, action, resource_type, resource_id, metadata)
        VALUES (uuid_generate_v4(), $1, 'user', 'price.updated', 'product', $2, $3)
        "#,
        auth.id,
        product_id,
        serde_json::json!({
            "new_amount_cents": payload.amount_cents,
            "applies_to": payload.applies_to,
        })
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "price": {
                "id": new_price.id,
                "amount_cents": new_price.amount_cents,
                "currency": new_price.currency,
                "applies_to": new_price.applies_to,
                "created_at": new_price.created_at,
            }
        })),
    ))
}
