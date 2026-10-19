use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct WebhookEvent {
    pub id: Uuid,
    pub stripe_event_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub processed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}
