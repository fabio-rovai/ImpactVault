use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use impactvault::domain::adapters::{TxRequest, YieldAdapter};
use impactvault::domain::engine::*;
use impactvault::domain::sentinel::{Sentinel, SentinelConfig};

// ---------------------------------------------------------------------------
// Mock adapter with configurable health
// ---------------------------------------------------------------------------

struct TestAdapter {
    adapter_name: String,
    health_score: f64,
    risk: RiskSpectrum,
}

#[async_trait]
impl YieldAdapter for TestAdapter {
    fn name(&self) -> &str {
        &self.adapter_name
    }

    fn risk_position(&self) -> RiskSpectrum {
        self.risk
    }

    async fn deposit(&self, amount: u128) -> anyhow::Result<TxRequest> {
        Ok(TxRequest {
            to: "0x1234".into(),
            data: format!("0x{:064x}", amount),
            value: "0".into(),
            chain_id: 11155111,
        })
    }

    async fn withdraw(&self, amount: u128) -> anyhow::Result<TxRequest> {
        Ok(TxRequest {
            to: "0x1234".into(),
            data: format!("0x{:064x}", amount),
            value: "0".into(),
            chain_id: 11155111,
        })
    }

    async fn current_yield_apy(&self) -> anyhow::Result<f64> {
        Ok(4.5)
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        Ok(HealthStatus {
            adapter_name: self.adapter_name.clone(),
            score: self.health_score,
            oracle_fresh: self.health_score > 0.3,
            liquidity_adequate: self.health_score > 0.2,
            utilisation_rate: 1.0 - self.health_score,
            details: "test adapter".into(),
        })
    }

    async fn tvl(&self) -> anyhow::Result<u128> {
        Ok(1_000_000)
    }
}

// ---------------------------------------------------------------------------
// Scenario 1: Full healthy pipeline (Hold)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_full_pipeline_healthy() {
    // 1. Create vault config with both sources approved
    let config = VaultConfig {
        approved_sources: vec![RiskSpectrum::Sovereign, RiskSpectrum::StablecoinSavings],
        max_exposure_per_source: 100,
        concentration_limit: 60,
        derisking_health_threshold: 0.5,
        auto_derisk_enabled: true,
        source_weights: std::collections::HashMap::new(),
    };

    // 2. Create adapters
    let adapters: Vec<Box<dyn YieldAdapter>> = vec![
        Box::new(TestAdapter {
            adapter_name: "sovereign_bond".into(),
            health_score: 0.95,
            risk: RiskSpectrum::Sovereign,
        }),
        Box::new(TestAdapter {
            adapter_name: "aave_savings".into(),
            health_score: 0.9,
            risk: RiskSpectrum::StablecoinSavings,
        }),
    ];

    // 3. Recommend allocation for 1M deposit
    let plan = recommend_allocation(&config, 1_000_000);
    assert_eq!(plan.allocations.len(), 2);
    let total: u128 = plan.allocations.iter().map(|a| a.amount).sum();
    assert_eq!(total, 1_000_000);

    // 4. Build portfolio from allocation
    let mut portfolio = Portfolio::new();
    for alloc in &plan.allocations {
        portfolio.add_allocation(alloc.clone());
    }
    assert_eq!(portfolio.total_deposited(), 1_000_000);

    // 5. Create sentinel and run health check
    let sentinel = Sentinel::new(
        SentinelConfig::default(),
        Arc::new(RwLock::new(config.clone())),
        adapters,
    );
    let health = sentinel.check_once().await;
    assert_eq!(health.len(), 2);

    // 6. Evaluate risk with real health data
    let assessment = evaluate_risk(&config, &portfolio, &health);
    assert!(assessment.overall_health > 0.8);
    assert!(assessment.breaches.is_empty());
    assert!(matches!(assessment.recommended_action, DeriskAction::Hold));

    // 7. Verify sentinel status updated
    let handle = sentinel.status_handle();
    let status = handle.read().await;
    assert_eq!(status.checks_completed, 1);
    assert!(matches!(status.last_action, Some(DeriskAction::Hold)));
}

// ---------------------------------------------------------------------------
// Scenario 2: Degraded adapter triggers Migrate
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_full_pipeline_derisking() {
    // Simulate a degraded adapter scenario
    let config = VaultConfig::default(); // sovereign only, threshold 0.5

    let adapters: Vec<Box<dyn YieldAdapter>> = vec![Box::new(TestAdapter {
        adapter_name: "degraded_adapter".into(),
        health_score: 0.3,
        risk: RiskSpectrum::StablecoinSavings,
    })];

    let sentinel = Sentinel::new(
        SentinelConfig::default(),
        Arc::new(RwLock::new(config.clone())),
        adapters,
    );

    let health = sentinel.check_once().await;

    // Engine should recommend migration
    let action = should_derisk(&config, &health);
    assert!(matches!(action, DeriskAction::Migrate { .. }));

    // Verify status reflects the action
    let handle = sentinel.status_handle();
    let status = handle.read().await;
    assert!(matches!(
        status.last_action,
        Some(DeriskAction::Migrate { .. })
    ));
}

// ---------------------------------------------------------------------------
// Scenario 3: Critical adapter triggers EmergencyWithdraw
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_full_pipeline_emergency() {
    let config = VaultConfig::default();

    let adapters: Vec<Box<dyn YieldAdapter>> = vec![Box::new(TestAdapter {
        adapter_name: "critical_adapter".into(),
        health_score: 0.1,
        risk: RiskSpectrum::StablecoinSavings,
    })];

    let sentinel = Sentinel::new(
        SentinelConfig::default(),
        Arc::new(RwLock::new(config.clone())),
        adapters,
    );

    let health = sentinel.check_once().await;
    let action = should_derisk(&config, &health);
    assert!(matches!(action, DeriskAction::EmergencyWithdraw { .. }));
}
