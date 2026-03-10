use std::sync::Arc;
use tokio::sync::Mutex;
use impactvault::gateway::server::ImpactVaultServer;
use impactvault::orchestration::enforcer::Enforcer;
use impactvault::store::state::StateDb;
use tempfile::TempDir;

fn setup() -> (TempDir, ImpactVaultServer) {
    let dir = TempDir::new().unwrap();
    let db = StateDb::open(&dir.path().join("test.db")).unwrap();
    let enforcer = Arc::new(Mutex::new(Enforcer::new()));
    let server = ImpactVaultServer::new(db, enforcer);
    (dir, server)
}

#[test]
fn test_server_has_tools() {
    let (_dir, server) = setup();
    let tools = server.list_tool_definitions();
    assert!(!tools.is_empty(), "Server should register at least one tool");

    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(
        names.contains(&"enforcer_check"),
        "Expected 'enforcer_check' tool, found: {:?}",
        names
    );
}

#[test]
fn test_server_has_core_tools() {
    let (_dir, server) = setup();
    let tools = server.list_tool_definitions();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    // Lineage
    assert!(names.contains(&"lineage_record"), "Missing lineage_record");
    assert!(names.contains(&"lineage_events"), "Missing lineage_events");
    assert!(names.contains(&"lineage_timeline"), "Missing lineage_timeline");

    // Enforcer
    assert!(names.contains(&"enforcer_check"), "Missing enforcer_check");
    assert!(names.contains(&"enforcer_rules"), "Missing enforcer_rules");
    assert!(names.contains(&"enforcer_log"), "Missing enforcer_log");
    assert!(names.contains(&"enforcer_toggle_rule"), "Missing enforcer_toggle_rule");

    // Patterns
    assert!(names.contains(&"pattern_analyze"), "Missing pattern_analyze");
    assert!(names.contains(&"pattern_list"), "Missing pattern_list");

    // Vault
    assert!(names.contains(&"vault_status"), "Missing vault_status");
    assert!(names.contains(&"vault_risk"), "Missing vault_risk");

    // Adapters
    assert!(names.contains(&"adapter_list"), "Missing adapter_list");
    assert!(names.contains(&"adapter_health"), "Missing adapter_health");

    // Sentinel
    assert!(names.contains(&"sentinel_status"), "Missing sentinel_status");

    // Risk
    assert!(names.contains(&"risk_evaluate"), "Missing risk_evaluate");

    // DPGA
    assert!(names.contains(&"dpga_list"), "Missing dpga_list");

    // Rebalance (under vault_ prefix)
    assert!(names.contains(&"vault_rebalance"), "Missing vault_rebalance");
}

#[test]
fn test_server_tool_count() {
    let (_dir, server) = setup();
    let tools = server.list_tool_definitions();
    // 3 lineage + 4 enforcer + 2 patterns + 3 vault + 2 adapter + 1 sentinel + 1 risk + 1 dpga = 17
    assert_eq!(
        tools.len(), 17,
        "Expected 17 tools, found: {}",
        tools.len()
    );
}

#[test]
fn test_server_tools_have_descriptions() {
    let (_dir, server) = setup();
    let tools = server.list_tool_definitions();
    for tool in &tools {
        assert!(
            tool.description.is_some(),
            "Tool '{}' is missing a description",
            tool.name
        );
        let desc = tool.description.as_ref().unwrap();
        assert!(
            !desc.is_empty(),
            "Tool '{}' has an empty description",
            tool.name
        );
    }
}

#[test]
fn test_server_is_clone() {
    let (_dir, server) = setup();
    let _cloned = server.clone();
}
