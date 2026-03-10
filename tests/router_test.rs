use impactvault::gateway::router::route_tool;

#[test]
fn test_route_by_prefix() {
    assert_eq!(route_tool("lineage_track"), "orchestration::lineage");
    assert_eq!(route_tool("enforcer_check"), "orchestration::enforcer");
    assert_eq!(route_tool("pattern_analyze"), "orchestration::patterns");
    assert_eq!(route_tool("vault_status"), "domain::vault");
    assert_eq!(route_tool("adapter_list"), "domain::adapter");
    assert_eq!(route_tool("sentinel_status"), "orchestration::sentinel");
    assert_eq!(route_tool("risk_evaluate"), "domain::risk");
    assert_eq!(route_tool("unknown_tool"), "unknown");
}

#[test]
fn test_route_empty_string() {
    assert_eq!(route_tool(""), "unknown");
}

#[test]
fn test_route_prefix_only() {
    assert_eq!(route_tool("enforcer_"), "orchestration::enforcer");
}
