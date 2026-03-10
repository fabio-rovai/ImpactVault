use axum::{Router, extract::Path, routing::get, Json};
use serde_json::json;

/// Build the REST API router with health, sentinel, and adapter endpoints.
pub fn api_router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/sentinel/status", get(sentinel_status))
        .route("/adapters", get(list_adapters))
        .route("/adapters/{name}/health", get(adapter_health))
        .route("/vault/{id}/status", get(vault_status))
        .route("/vault/{id}/risk", get(vault_risk))
        .route("/yield/history", get(yield_history))
        .route("/disbursements", get(disbursements))
        .route("/risk/assessment", get(risk_assessment))
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

async fn adapter_health(Path(name): Path<String>) -> Json<serde_json::Value> {
    Json(json!({
        "adapter": name,
        "status": "healthy",
        "last_check": "2025-01-01T00:00:00Z",
        "uptime_pct": 99.9
    }))
}

async fn vault_status(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({
        "vault_id": id,
        "tvl": 1_000_000,
        "current_yield": 0.045,
        "total_disbursed": 50_000,
        "allocations": [
            {"adapter": "sovereign_bond", "amount": 600_000},
            {"adapter": "aave_savings", "amount": 400_000}
        ]
    }))
}

async fn vault_risk(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({
        "vault_id": id,
        "overall_risk": "low",
        "risk_score": 0.15,
        "factors": [
            {"name": "smart_contract", "score": 0.1},
            {"name": "market", "score": 0.2},
            {"name": "liquidity", "score": 0.15}
        ]
    }))
}

async fn yield_history() -> Json<serde_json::Value> {
    Json(json!({
        "events": []
    }))
}

async fn disbursements() -> Json<serde_json::Value> {
    Json(json!({
        "total_disbursed": 0,
        "recipient_count": 0,
        "disbursements": []
    }))
}

async fn risk_assessment() -> Json<serde_json::Value> {
    Json(json!({
        "overall_risk": "low",
        "risk_score": 0.12,
        "categories": {
            "smart_contract": 0.1,
            "market": 0.15,
            "liquidity": 0.1,
            "counterparty": 0.05
        },
        "recommendations": [
            "Maintain current diversification strategy",
            "Monitor sovereign bond yields for rate changes"
        ]
    }))
}
