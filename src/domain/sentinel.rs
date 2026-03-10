use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::domain::adapters::YieldAdapter;
use crate::domain::engine::{self, DeriskAction, HealthStatus, VaultConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelConfig {
    pub poll_interval_secs: u64,
    pub auto_derisk_enabled: bool,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 60,
            auto_derisk_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelStatus {
    pub running: bool,
    pub last_check: Option<String>,
    pub checks_completed: u64,
    pub last_health: Vec<HealthStatus>,
    pub last_action: Option<DeriskAction>,
}

impl Default for SentinelStatus {
    fn default() -> Self {
        Self {
            running: false,
            last_check: None,
            checks_completed: 0,
            last_health: Vec::new(),
            last_action: None,
        }
    }
}

pub struct Sentinel {
    config: SentinelConfig,
    vault_config: Arc<RwLock<VaultConfig>>,
    adapters: Vec<Box<dyn YieldAdapter>>,
    status: Arc<RwLock<SentinelStatus>>,
}

impl Sentinel {
    pub fn new(
        config: SentinelConfig,
        vault_config: Arc<RwLock<VaultConfig>>,
        adapters: Vec<Box<dyn YieldAdapter>>,
    ) -> Self {
        Self {
            config,
            vault_config,
            adapters,
            status: Arc::new(RwLock::new(SentinelStatus::default())),
        }
    }

    pub fn status_handle(&self) -> Arc<RwLock<SentinelStatus>> {
        Arc::clone(&self.status)
    }

    /// Run a single health check cycle.
    pub async fn check_once(&self) -> Vec<HealthStatus> {
        let mut results = Vec::with_capacity(self.adapters.len());

        for adapter in &self.adapters {
            let health = match adapter.health_check().await {
                Ok(h) => h,
                Err(e) => {
                    warn!(adapter = adapter.name(), error = %e, "health check failed");
                    HealthStatus {
                        adapter_name: adapter.name().to_string(),
                        score: 0.0,
                        oracle_fresh: false,
                        liquidity_adequate: false,
                        utilisation_rate: 1.0,
                        details: format!("health check error: {e}"),
                    }
                }
            };
            results.push(health);
        }

        let vault_cfg = self.vault_config.read().await;
        let action = engine::should_derisk(&vault_cfg, &results);

        match &action {
            DeriskAction::Hold => {
                info!("sentinel check: Hold — all adapters healthy");
            }
            DeriskAction::Migrate { from, to } => {
                warn!(from = %from, to = ?to, "sentinel check: Migrate recommended");
            }
            DeriskAction::EmergencyWithdraw { adapter } => {
                error!(adapter = %adapter, "sentinel check: EmergencyWithdraw triggered");
            }
        }

        let now = chrono::Utc::now().to_rfc3339();
        let mut status = self.status.write().await;
        status.last_check = Some(now);
        status.checks_completed += 1;
        status.last_health = results.clone();
        status.last_action = Some(action);

        results
    }

    /// Run the monitoring loop until a shutdown signal is received.
    pub async fn run(self: Arc<Self>, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        {
            let mut status = self.status.write().await;
            status.running = true;
        }
        info!(
            interval_secs = self.config.poll_interval_secs,
            "sentinel monitoring loop started"
        );

        let interval = tokio::time::Duration::from_secs(self.config.poll_interval_secs);

        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    self.check_once().await;
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("sentinel received shutdown signal");
                        break;
                    }
                }
            }
        }

        let mut status = self.status.write().await;
        status.running = false;
        info!("sentinel monitoring loop stopped");
    }
}
