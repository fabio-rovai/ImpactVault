use axum::{Router, routing::get, Json};
use serde_json::json;

/// Build the REST API router with health, sentinel, and adapter endpoints.
pub fn api_router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/sentinel/status", get(sentinel_status))
        .route("/adapters", get(list_adapters))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

async fn sentinel_status() -> Json<serde_json::Value> {
    // Placeholder — will be wired to real sentinel status later
    Json(json!({
        "running": false,
        "checks_completed": 0,
        "message": "sentinel not yet wired to API"
    }))
}

async fn list_adapters() -> Json<serde_json::Value> {
    Json(json!({
        "adapters": [
            {"name": "sovereign_bond", "risk_position": "Sovereign"},
            {"name": "aave_savings", "risk_position": "StablecoinSavings"}
        ]
    }))
}
