use axum::{
    routing::{get, patch, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;

pub mod auth;
pub mod products;
pub mod subscriptions;
pub mod users;
pub mod webhooks;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check (no auth)
        .route("/health", get(health_check))
        // Auth routes (no auth required)
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/refresh", post(auth::refresh))
        .route("/api/v1/auth/logout", post(auth::logout))
        // User routes (auth required)
        .route("/api/v1/users/me", get(users::get_me).patch(users::update_me))
        // Product routes
        .route("/api/v1/products", get(products::list).post(products::create))
        .route(
            "/api/v1/products/:id",
            get(products::get_one).patch(products::update),
        )
        .route("/api/v1/products/:id/prices", post(products::add_price))
        // Subscription routes
        .route(
            "/api/v1/subscriptions",
            get(subscriptions::list_all).post(subscriptions::create),
        )
        .route("/api/v1/subscriptions/me", get(subscriptions::get_mine))
        .route(
            "/api/v1/subscriptions/me/cancel",
            patch(subscriptions::cancel_mine),
        )
        .route(
            "/api/v1/subscriptions/:id",
            get(subscriptions::get_one),
        )
        // Webhook (no JWT auth — Stripe signature verification instead)
        .route("/api/v1/webhooks/stripe", post(webhooks::stripe_webhook))
        // Attach shared state
        .with_state(state)
        // Middleware layers
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive()) // Configure CORS properly in production
}

async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
