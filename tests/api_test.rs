use axum_test::TestServer;
use impactvault::gateway::api::api_router;

#[test]
fn test_api_router_builds() {
    // Just verify the router can be constructed without panic
    let _ = api_router();
}

#[tokio::test]
async fn test_health_endpoint() {
    let server = TestServer::new(api_router());
    let resp = server.get("/health").await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn test_sentinel_status_endpoint() {
    let server = TestServer::new(api_router());
    let resp = server.get("/sentinel/status").await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn test_list_adapters_endpoint() {
    let server = TestServer::new(api_router());
    let resp = server.get("/adapters").await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn test_adapter_health_endpoint() {
    let server = TestServer::new(api_router());
    let resp = server.get("/adapters/aave_savings/health").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["adapter"], "aave_savings");
    assert_eq!(body["status"], "healthy");
}

#[tokio::test]
async fn test_vault_status_endpoint() {
    let server = TestServer::new(api_router());
    let resp = server.get("/vault/vault-001/status").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["vault_id"], "vault-001");
    assert!(body["tvl"].is_number());
    assert!(body["allocations"].is_array());
}

#[tokio::test]
async fn test_vault_risk_endpoint() {
    let server = TestServer::new(api_router());
    let resp = server.get("/vault/vault-001/risk").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["vault_id"], "vault-001");
    assert_eq!(body["overall_risk"], "low");
}

#[tokio::test]
async fn test_yield_history_endpoint() {
    let server = TestServer::new(api_router());
    let resp = server.get("/yield/history").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["events"].is_array());
}

#[tokio::test]
async fn test_disbursements_endpoint() {
    let server = TestServer::new(api_router());
    let resp = server.get("/disbursements").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["total_disbursed"], 0);
    assert_eq!(body["recipient_count"], 0);
    assert!(body["disbursements"].is_array());
}

#[tokio::test]
async fn test_risk_assessment_endpoint() {
    let server = TestServer::new(api_router());
    let resp = server.get("/risk/assessment").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["overall_risk"], "low");
    assert!(body["categories"].is_object());
}
