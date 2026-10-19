use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub stripe_product_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct Price {
    pub id: Uuid,
    pub product_id: Option<Uuid>,
    pub plan_id: Option<Uuid>,
    pub amount_cents: i32,
    pub currency: String,
    pub applies_to: String,
    pub applies_from_date: Option<chrono::NaiveDate>,
    pub stripe_price_id: Option<String>,
    pub is_current: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct Plan {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub interval: String,
    pub interval_count: i32,
    pub is_active: bool,
    pub stripe_product_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub description: Option<String>,
    pub amount_cents: i32,
    pub currency: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddPriceRequest {
    pub amount_cents: i32,
    pub currency: Option<String>,
    pub applies_to: String,
    pub applies_from_date: Option<chrono::NaiveDate>,
}
