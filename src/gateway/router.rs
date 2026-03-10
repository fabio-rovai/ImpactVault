/// Route a tool name to the module that should handle it, based on prefix.
///
/// Returns a string identifying the target module. When domain modules are
/// built out, this will drive actual dispatch; for now it is used for
/// introspection and testing.
pub fn route_tool(name: &str) -> &'static str {
    if name.starts_with("lineage_") {
        "orchestration::lineage"
    } else if name.starts_with("enforcer_") {
        "orchestration::enforcer"
    } else if name.starts_with("pattern_") {
        "orchestration::patterns"
    } else if name.starts_with("vault_") {
        "domain::vault"
    } else if name.starts_with("adapter_") {
        "domain::adapter"
    } else if name.starts_with("sentinel_") {
        "orchestration::sentinel"
    } else if name.starts_with("risk_") {
        "domain::risk"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_all_prefixes() {
        assert_eq!(route_tool("lineage_track"), "orchestration::lineage");
        assert_eq!(route_tool("enforcer_check"), "orchestration::enforcer");
        assert_eq!(route_tool("pattern_analyze"), "orchestration::patterns");
        assert_eq!(route_tool("vault_status"), "domain::vault");
        assert_eq!(route_tool("adapter_list"), "domain::adapter");
        assert_eq!(route_tool("sentinel_status"), "orchestration::sentinel");
        assert_eq!(route_tool("risk_evaluate"), "domain::risk");
        assert_eq!(route_tool("unknown_tool"), "unknown");
    }
}
