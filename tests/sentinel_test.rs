use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use impactvault::domain::adapters::{TxRequest, YieldAdapter};
use impactvault::domain::engine::{DeriskAction, HealthStatus, RiskSpectrum, VaultConfig};
use impactvault::domain::sentinel::{Sentinel, SentinelConfig};

// ---------------------------------------------------------------------------
// Mock adapter
// ---------------------------------------------------------------------------

struct MockAdapter {
    health_score: f64,
}

#[async_trait]
impl YieldAdapter for MockAdapter {
    fn name(&self) -> &str {
        "mock"
    }

    fn risk_position(&self) -> RiskSpectrum {
        RiskSpectrum::Sovereign
    }

    async fn deposit(&self, _amount: u128) -> anyhow::Result<TxRequest> {
        todo!()
    }

    async fn withdraw(&self, _amount: u128) -> anyhow::Result<TxRequest> {
        todo!()
    }

    async fn current_yield_apy(&self) -> anyhow::Result<f64> {
        Ok(4.0)
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        Ok(HealthStatus {
            adapter_name: "mock".into(),
            score: self.health_score,
            oracle_fresh: self.health_score > 0.3,
            liquidity_adequate: self.health_score > 0.2,
            utilisation_rate: 1.0 - self.health_score,
            details: "mock adapter".into(),
        })
    }

    async fn tvl(&self) -> anyhow::Result<u128> {
        Ok(0)
    }
}

// ---------------------------------------------------------------------------
// Helper to build a sentinel with a given health score
// ---------------------------------------------------------------------------

fn make_sentinel(health_score: f64) -> Sentinel {
    let adapters: Vec<Box<dyn YieldAdapter>> =
        vec![Box::new(MockAdapter { health_score })];
    let vault_config = Arc::new(RwLock::new(VaultConfig::default()));
    Sentinel::new(SentinelConfig::default(), vault_config, adapters)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_sentinel_config_defaults() {
    let config = SentinelConfig::default();
    assert_eq!(config.poll_interval_secs, 60);
    assert!(config.auto_derisk_enabled);
}

#[tokio::test]
async fn test_check_once_healthy() {
    // Score 0.9 is well above the 0.5 threshold => Hold
    let sentinel = make_sentinel(0.9);
    let results = sentinel.check_once().await;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].adapter_name, "mock");
    assert!((results[0].score - 0.9).abs() < f64::EPSILON);

    let handle = sentinel.status_handle();
    let status = handle.read().await;
    assert_eq!(status.checks_completed, 1);
    match &status.last_action {
        Some(DeriskAction::Hold) => {} // expected
        other => panic!("expected Hold, got {:?}", other),
    }
}

#[tokio::test]
async fn test_check_once_unhealthy() {
    // Score 0.3 is below 0.5 threshold but >= 0.2 => Migrate
    let sentinel = make_sentinel(0.3);
    let results = sentinel.check_once().await;

    assert_eq!(results.len(), 1);
    assert!((results[0].score - 0.3).abs() < f64::EPSILON);

    let handle = sentinel.status_handle();
    let status = handle.read().await;
    match &status.last_action {
        Some(DeriskAction::Migrate { from, .. }) => {
            assert_eq!(from, "mock");
        }
        other => panic!("expected Migrate, got {:?}", other),
    }
}

#[tokio::test]
async fn test_check_once_critical() {
    // Score 0.1 is below 0.2 => EmergencyWithdraw
    let sentinel = make_sentinel(0.1);
    let results = sentinel.check_once().await;

    assert_eq!(results.len(), 1);
    assert!((results[0].score - 0.1).abs() < f64::EPSILON);

    let handle = sentinel.status_handle();
    let status = handle.read().await;
    match &status.last_action {
        Some(DeriskAction::EmergencyWithdraw { adapter }) => {
            assert_eq!(adapter, "mock");
        }
        other => panic!("expected EmergencyWithdraw, got {:?}", other),
    }
}

#[tokio::test]
async fn test_status_updates_after_check() {
    let sentinel = make_sentinel(0.8);

    // Before any check
    {
        let handle = sentinel.status_handle();
        let status = handle.read().await;
        assert_eq!(status.checks_completed, 0);
        assert!(status.last_check.is_none());
    }

    sentinel.check_once().await;
    {
        let handle = sentinel.status_handle();
        let status = handle.read().await;
        assert_eq!(status.checks_completed, 1);
        assert!(status.last_check.is_some());
    }

    sentinel.check_once().await;
    {
        let handle = sentinel.status_handle();
        let status = handle.read().await;
        assert_eq!(status.checks_completed, 2);
    }
}
