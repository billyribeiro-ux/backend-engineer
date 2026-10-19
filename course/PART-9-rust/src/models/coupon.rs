use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct Coupon {
    pub id: Uuid,
    pub code: String,
    pub discount_type: String,    // 'percent' or 'fixed'
    pub discount_value: i32,
    pub max_uses: Option<i32>,
    pub current_uses: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub applies_to: String,
    pub stripe_coupon_id: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CouponValidationResponse {
    pub valid: bool,
    pub reason: Option<String>,
    pub message: Option<String>,
    pub discount_type: Option<String>,
    pub discount_value: Option<i32>,
    pub original_amount_cents: Option<i32>,
    pub final_amount_cents: Option<i32>,
    pub savings_cents: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ValidateCouponRequest {
    pub code: String,
    pub product_id: Option<Uuid>,
    pub plan_id: Option<Uuid>,
}

pub fn apply_discount(amount_cents: i32, discount_type: &str, discount_value: i32) -> i32 {
    match discount_type {
        "percent" => {
            let discount = (amount_cents as f64 * (discount_value as f64 / 100.0)) as i32;
            (amount_cents - discount).max(0)
        }
        "fixed" => (amount_cents - discount_value).max(0),
        _ => amount_cents,
    }
}
