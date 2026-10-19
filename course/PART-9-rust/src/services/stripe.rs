/// Stripe service — HTTP client for calling the Stripe API.
///
/// We use reqwest to call Stripe's REST API directly.
/// In production, consider the `stripe-rust` crate for type-safe Stripe calls.

use reqwest::Client;
use serde_json::Value;

use crate::config::Config;
use crate::errors::AppError;

pub struct StripeClient {
    client: Client,
    secret_key: String,
    base_url: String,
}

impl StripeClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            secret_key: config.stripe_secret_key.clone(),
            base_url: "https://api.stripe.com/v1".to_string(),
        }
    }

    async fn post(&self, path: &str, params: &[(&str, &str)]) -> Result<Value, AppError> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .basic_auth(&self.secret_key, None::<&str>)
            .form(params)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Stripe request failed: {}", e))
            .map_err(AppError::Internal)?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse Stripe response: {}", e))
            .map_err(AppError::Internal)?;

        if !status.is_success() {
            let error_msg = body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown Stripe error");
            return Err(AppError::bad_request("STRIPE_ERROR", error_msg));
        }

        Ok(body)
    }

    pub async fn create_customer(&self, email: &str, user_id: &str) -> Result<Value, AppError> {
        self.post(
            "/customers",
            &[
                ("email", email),
                ("metadata[user_id]", user_id),
            ],
        )
        .await
    }

    pub async fn create_product(&self, name: &str, product_id: &str) -> Result<Value, AppError> {
        self.post(
            "/products",
            &[
                ("name", name),
                ("metadata[product_id]", product_id),
            ],
        )
        .await
    }

    pub async fn create_price(
        &self,
        stripe_product_id: &str,
        amount_cents: i32,
        currency: &str,
    ) -> Result<Value, AppError> {
        let amount_str = amount_cents.to_string();
        self.post(
            "/prices",
            &[
                ("product", stripe_product_id),
                ("unit_amount", &amount_str),
                ("currency", currency),
            ],
        )
        .await
    }

    pub async fn create_recurring_price(
        &self,
        stripe_product_id: &str,
        amount_cents: i32,
        currency: &str,
        interval: &str,
        interval_count: i32,
    ) -> Result<Value, AppError> {
        let amount_str = amount_cents.to_string();
        let interval_count_str = interval_count.to_string();
        self.post(
            "/prices",
            &[
                ("product", stripe_product_id),
                ("unit_amount", &amount_str),
                ("currency", currency),
                ("recurring[interval]", interval),
                ("recurring[interval_count]", &interval_count_str),
            ],
        )
        .await
    }

    pub async fn cancel_subscription(&self, stripe_sub_id: &str) -> Result<Value, AppError> {
        let response = self
            .client
            .delete(format!("{}/subscriptions/{}", self.base_url, stripe_sub_id))
            .basic_auth(&self.secret_key, None::<&str>)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Stripe request failed: {}", e))
            .map_err(AppError::Internal)?;

        let body: Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse Stripe response: {}", e))
            .map_err(AppError::Internal)?;

        Ok(body)
    }
}
