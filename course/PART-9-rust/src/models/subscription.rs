use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub plan_id: Uuid,
    pub locked_price_id: Option<Uuid>,
    pub stripe_subscription_id: Option<String>,
    pub status: String,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Subscription {
    pub fn has_active_access(&self) -> bool {
        matches!(self.status.as_str(), "active" | "trialing")
    }

    pub fn is_canceled(&self) -> bool {
        self.status == "canceled"
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub plan_id: Uuid,
    pub payment_method_id: String,
    pub coupon_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CancelSubscriptionRequest {
    pub immediately: Option<bool>,
}
