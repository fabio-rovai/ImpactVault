use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// Top-level configuration for ImpactVault.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub enforcer: EnforcerConfig,
    pub lineage: LineageConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            enforcer: EnforcerConfig::default(),
            lineage: LineageConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file.
    ///
    /// Missing sections/fields fall back to defaults via `#[serde(default)]`.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;
        Ok(config)
    }
}

/// General paths and directories.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub data_dir: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            data_dir: "~/.impactvault".into(),
        }
    }
}

/// Policy enforcer settings.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct EnforcerConfig {
    pub enabled: bool,
    pub default_action: String,
    #[serde(default)]
    pub rules: Vec<RuleConfig>,
}

impl Default for EnforcerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_action: "block".into(),
            rules: Vec::new(),
        }
    }
}

/// A single enforcer rule defined in TOML config.
#[derive(Debug, Deserialize, Clone)]
pub struct RuleConfig {
    pub name: String,
    pub description: Option<String>,
    pub action: String,
    pub enabled: Option<bool>,
    pub condition: RuleConditionConfig,
}

/// Flat TOML representation of a rule condition.
#[derive(Debug, Deserialize, Clone)]
pub struct RuleConditionConfig {
    /// "MissingInWindow" or "RepeatWithout"
    #[serde(rename = "type")]
    pub kind: String,
    pub trigger: Option<String>,
    pub required: Option<String>,
    pub window: Option<usize>,
    pub category: Option<String>,
    pub count: Option<usize>,
}

/// Lineage tracking HTTP API settings.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct LineageConfig {
    pub http_port: u16,
}

impl Default for LineageConfig {
    fn default() -> Self {
        Self { http_port: 0 }
    }
}

/// Expand a leading `~` or `~/` in a path to the user's home directory.
pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") || path == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return path.replacen("~", &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}
