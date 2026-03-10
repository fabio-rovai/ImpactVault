use std::sync::Arc;
use tokio::sync::Mutex;

use rmcp::{
    ServerHandler, tool, tool_handler, tool_router,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo, Tool},
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::orchestration::enforcer::{Action, Enforcer};
use crate::store::state::StateDb;

// ─── MCP tool input structs ─────────────────────────────────────────────────

// Lineage
#[derive(Deserialize, JsonSchema)]
pub struct LineageRecordInput {
    /// Session ID
    pub session_id: String,
    /// Event type (tool_call, tool_result, file_read, file_write)
    pub event_type: String,
    /// Optional file path
    pub path: Option<String>,
    /// Optional tool name
    pub tool: Option<String>,
    /// Optional metadata (JSON)
    pub meta: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
pub struct LineageEventsInput {
    /// Optional session ID filter
    pub session_id: Option<String>,
    /// Optional event type filter
    pub event_type: Option<String>,
    /// Maximum results
    pub limit: Option<usize>,
}
#[derive(Deserialize, JsonSchema)]
pub struct LineageTimelineInput {
    /// Session ID to get timeline for
    pub session_id: String,
}

// Enforcer
#[derive(Deserialize, JsonSchema)]
pub struct EnforcerCheckInput {
    /// Tool name to check against enforcer rules
    pub tool_name: String,
}
#[derive(Deserialize, JsonSchema)]
pub struct EnforcerLogInput {
    /// Optional session ID filter
    pub session_id: Option<String>,
    /// Maximum entries to return
    pub limit: Option<usize>,
}
#[derive(Deserialize, JsonSchema)]
pub struct EnforcerRuleToggleInput {
    /// Rule name to enable or disable
    pub rule_name: String,
    /// Whether to enable the rule
    pub enabled: bool,
}

// Patterns
#[derive(Deserialize, JsonSchema)]
pub struct PatternListInput {
    /// Optional category filter
    pub category: Option<String>,
}

// ─── ImpactVaultServer ──────────────────────────────────────────────────────

/// MCP server that exposes all ImpactVault tools to Claude via stdin/stdout.
#[derive(Clone)]
pub struct ImpactVaultServer {
    tool_router: ToolRouter<Self>,
    db: StateDb,
    enforcer: Arc<Mutex<Enforcer>>,
}

impl ImpactVaultServer {
    /// Create a new server with all tools wired to domain/orchestration services.
    pub fn new(db: StateDb, enforcer: Arc<Mutex<Enforcer>>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            db,
            enforcer,
        }
    }

    /// Return the list of all registered tool definitions.
    pub fn list_tool_definitions(&self) -> Vec<Tool> {
        self.tool_router.list_all()
    }
}

// ─── Tool definitions ───────────────────────────────────────────────────────

#[tool_router]
impl ImpactVaultServer {

    // ── Lineage ─────────────────────────────────────────────────────────────

    #[tool(name = "lineage_record", description = "Record a lineage event (tool call, file read/write, etc.)")]
    async fn lineage_record(&self, Parameters(input): Parameters<LineageRecordInput>) -> String {
        use crate::orchestration::lineage::{LineageEvent, LineageService};
        let meta = input.meta.and_then(|s| serde_json::from_str(&s).ok());
        let event = LineageEvent {
            seq: None,
            session_id: Some(input.session_id),
            timestamp: chrono::Utc::now().timestamp_millis(),
            event_type: input.event_type,
            path: input.path,
            tool: input.tool,
            meta,
        };
        match LineageService::record_event(&self.db, &event) {
            Ok(seq) => format!(r#"{{"seq":{seq}}}"#),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "lineage_events", description = "Query lineage events, optionally filtered by session or type")]
    async fn lineage_events(&self, Parameters(input): Parameters<LineageEventsInput>) -> String {
        use crate::orchestration::lineage::LineageService;
        match LineageService::get_events(&self.db, input.session_id.as_deref(), input.event_type.as_deref(), input.limit.unwrap_or(50)) {
            Ok(events) => serde_json::to_string(&events).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "lineage_timeline", description = "Get a timeline of events for a session")]
    async fn lineage_timeline(&self, Parameters(input): Parameters<LineageTimelineInput>) -> String {
        use crate::orchestration::lineage::LineageService;
        match LineageService::get_timeline(&self.db, &input.session_id) {
            Ok(timeline) => serde_json::to_string(&timeline).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    // ── Enforcer ────────────────────────────────────────────────────────────

    #[tool(name = "enforcer_check", description = "Check if a tool call is allowed by enforcer rules and record it")]
    async fn enforcer_check(&self, Parameters(input): Parameters<EnforcerCheckInput>) -> String {
        let mut enforcer = self.enforcer.lock().await;
        let verdict = enforcer.pre_check(&input.tool_name);
        enforcer.post_check(&input.tool_name);
        let action_str = match verdict.action {
            Action::Block => "block",
            Action::Warn => "warn",
            Action::Allow => "allow",
        };
        serde_json::json!({
            "action": action_str,
            "rule": verdict.rule,
            "reason": verdict.reason,
        })
        .to_string()
    }

    #[tool(name = "enforcer_log", description = "View the enforcement log, optionally filtered by session")]
    async fn enforcer_log(&self, Parameters(input): Parameters<EnforcerLogInput>) -> String {
        let log = Enforcer::get_log(&self.db, input.session_id.as_deref(), input.limit.unwrap_or(20));
        serde_json::to_string(&log).unwrap_or_default()
    }

    #[tool(name = "enforcer_rules", description = "List all enforcer rules and their enabled status")]
    async fn enforcer_rules(&self) -> String {
        let enforcer = self.enforcer.lock().await;
        let rules: Vec<serde_json::Value> = enforcer.rules().iter().map(|r| {
            serde_json::json!({
                "name": r.name,
                "description": r.description,
                "action": format!("{:?}", r.action),
                "enabled": r.enabled,
            })
        }).collect();
        serde_json::to_string(&rules).unwrap_or_default()
    }

    #[tool(name = "enforcer_toggle_rule", description = "Enable or disable an enforcer rule")]
    async fn enforcer_toggle_rule(&self, Parameters(input): Parameters<EnforcerRuleToggleInput>) -> String {
        // Persist to DB first so the toggle survives hot-reloads
        {
            let conn = self.db.conn();
            match conn.execute(
                "UPDATE rules SET enabled = ?1 WHERE name = ?2",
                rusqlite::params![input.enabled as i32, input.rule_name],
            ) {
                Ok(0) => return format!(r#"{{"error":"Rule '{}' not found in DB"}}"#, input.rule_name),
                Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
                Ok(_) => {}
            }
        }
        // Update in-memory cache
        let mut enforcer = self.enforcer.lock().await;
        let in_memory_updated = enforcer.set_rule_enabled(&input.rule_name, input.enabled);
        if !in_memory_updated {
            return format!(
                r#"{{"ok":true,"rule":"{}","enabled":{},"warning":"rule updated in DB but not found in memory cache; restart to sync"}}"#,
                input.rule_name, input.enabled
            );
        }
        format!(r#"{{"ok":true,"rule":"{}","enabled":{}}}"#, input.rule_name, input.enabled)
    }

    // ── Patterns ────────────────────────────────────────────────────────────

    #[tool(name = "pattern_analyze", description = "Analyze enforcement log to discover patterns across sessions")]
    fn pattern_analyze(&self) -> String {
        use crate::orchestration::patterns::PatternService;
        match PatternService::analyze_enforcement(&self.db) {
            Ok(patterns) => serde_json::to_string(&patterns).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "pattern_list", description = "List discovered patterns, optionally filtered by category")]
    async fn pattern_list(&self, Parameters(input): Parameters<PatternListInput>) -> String {
        use crate::orchestration::patterns::PatternService;
        match PatternService::list(&self.db, input.category.as_deref()) {
            Ok(patterns) => serde_json::to_string(&patterns).unwrap_or_default(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

}

// ─── ServerHandler ──────────────────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for ImpactVaultServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("ImpactVault: risk-curated yield infrastructure for social impact")
    }
}
