#[test]
fn test_api_router_builds() {
    // Just verify the router can be constructed without panic
    let _ = impactvault::gateway::api::api_router();
}
