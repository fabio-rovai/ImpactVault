use impactvault::domain::engine::*;

// --- Task 3: Type tests ---

#[test]
fn test_risk_spectrum_ordering() {
    assert!(RiskSpectrum::Sovereign < RiskSpectrum::StablecoinSavings);
}

#[test]
fn test_vault_config_default_has_safe_limits() {
    let config = VaultConfig::default();
    assert!(config.max_exposure_per_source <= 100);
    assert!(config.concentration_limit <= 100);
    assert!(config.derisking_health_threshold > 0.0);
}

#[test]
fn test_portfolio_empty_on_creation() {
    let portfolio = Portfolio::new();
    assert_eq!(portfolio.total_deposited(), 0);
    assert!(portfolio.allocations().is_empty());
}

#[test]
fn test_portfolio_tracks_deposits_correctly() {
    let mut portfolio = Portfolio::new();
    portfolio.add_allocation(Allocation {
        source: RiskSpectrum::Sovereign,
        adapter_name: "sovereign_bond".into(),
        amount: 500_000,
    });
    assert_eq!(portfolio.total_deposited(), 500_000);
    assert_eq!(portfolio.allocations().len(), 1);

    portfolio.add_allocation(Allocation {
        source: RiskSpectrum::StablecoinSavings,
        adapter_name: "aave_savings".into(),
        amount: 300_000,
    });
    assert_eq!(portfolio.total_deposited(), 800_000);
    assert_eq!(portfolio.allocations().len(), 2);
}

// --- Task 4: Risk Evaluation tests ---

#[test]
fn test_evaluate_risk_healthy_portfolio() {
    let mut config = VaultConfig::default();
    config.concentration_limit = 100; // single adapter, allow full concentration
    let mut portfolio = Portfolio::new();
    portfolio.add_allocation(Allocation {
        source: RiskSpectrum::Sovereign,
        adapter_name: "sovereign_bond".into(),
        amount: 1_000_000,
    });
    let health = vec![HealthStatus {
        adapter_name: "sovereign_bond".into(),
        score: 0.95,
        oracle_fresh: true,
        liquidity_adequate: true,
        utilisation_rate: 0.3,
        details: "healthy".into(),
    }];
    let assessment = evaluate_risk(&config, &portfolio, &health);
    assert!(assessment.overall_health > 0.8);
    assert!(assessment.breaches.is_empty());
    assert!(matches!(assessment.recommended_action, DeriskAction::Hold));
}

#[test]
fn test_evaluate_risk_unhealthy_triggers_derisk() {
    let config = VaultConfig::default();
    let mut portfolio = Portfolio::new();
    portfolio.add_allocation(Allocation {
        source: RiskSpectrum::StablecoinSavings,
        adapter_name: "aave_savings".into(),
        amount: 1_000_000,
    });
    let health = vec![HealthStatus {
        adapter_name: "aave_savings".into(),
        score: 0.3,
        oracle_fresh: false,
        liquidity_adequate: false,
        utilisation_rate: 0.95,
        details: "critical".into(),
    }];
    let assessment = evaluate_risk(&config, &portfolio, &health);
    assert!(assessment.overall_health < 0.5);
    assert!(!assessment.breaches.is_empty());
    assert!(!matches!(
        assessment.recommended_action,
        DeriskAction::Hold
    ));
}

#[test]
fn test_evaluate_risk_concentration_breach() {
    let mut config = VaultConfig::default();
    config.approved_sources = vec![RiskSpectrum::Sovereign, RiskSpectrum::StablecoinSavings];
    config.concentration_limit = 50;
    let mut portfolio = Portfolio::new();
    portfolio.add_allocation(Allocation {
        source: RiskSpectrum::StablecoinSavings,
        adapter_name: "aave_savings".into(),
        amount: 800_000,
    });
    portfolio.add_allocation(Allocation {
        source: RiskSpectrum::Sovereign,
        adapter_name: "sovereign_bond".into(),
        amount: 200_000,
    });
    let health = vec![
        HealthStatus {
            adapter_name: "aave_savings".into(),
            score: 0.9,
            oracle_fresh: true,
            liquidity_adequate: true,
            utilisation_rate: 0.3,
            details: "ok".into(),
        },
        HealthStatus {
            adapter_name: "sovereign_bond".into(),
            score: 0.95,
            oracle_fresh: true,
            liquidity_adequate: true,
            utilisation_rate: 0.1,
            details: "ok".into(),
        },
    ];
    let assessment = evaluate_risk(&config, &portfolio, &health);
    assert!(assessment
        .breaches
        .iter()
        .any(|b| b.contains("concentration")));
}

#[test]
fn test_evaluate_risk_stale_oracle_flagged() {
    let mut config = VaultConfig::default();
    config.concentration_limit = 100;
    let mut portfolio = Portfolio::new();
    portfolio.add_allocation(Allocation {
        source: RiskSpectrum::Sovereign,
        adapter_name: "sovereign_bond".into(),
        amount: 1_000_000,
    });
    let health = vec![HealthStatus {
        adapter_name: "sovereign_bond".into(),
        score: 0.9, // healthy score, but oracle is stale
        oracle_fresh: false,
        liquidity_adequate: true,
        utilisation_rate: 0.2,
        details: "oracle stale".into(),
    }];
    let assessment = evaluate_risk(&config, &portfolio, &health);
    assert!(
        assessment.breaches.iter().any(|b| b.contains("oracle")),
        "stale oracle should produce a breach"
    );
    assert!(
        !matches!(assessment.recommended_action, DeriskAction::Hold),
        "stale oracle breach should trigger an action, not Hold"
    );
}

// --- Task 5: Allocation tests ---

#[test]
fn test_recommend_allocation_single_source() {
    let config = VaultConfig::default();
    let plan = recommend_allocation(&config, 1_000_000);
    assert_eq!(plan.allocations.len(), 1);
    assert_eq!(plan.allocations[0].source, RiskSpectrum::Sovereign);
    assert_eq!(plan.allocations[0].amount, 1_000_000);
}

#[test]
fn test_recommend_allocation_two_sources_respects_concentration() {
    let mut config = VaultConfig::default();
    config.approved_sources = vec![RiskSpectrum::Sovereign, RiskSpectrum::StablecoinSavings];
    config.concentration_limit = 60;
    let plan = recommend_allocation(&config, 1_000_000);
    assert_eq!(plan.allocations.len(), 2);
    for alloc in &plan.allocations {
        let pct = (alloc.amount as f64 / 1_000_000.0 * 100.0) as u8;
        assert!(pct <= 60, "allocation {}% exceeds limit", pct);
    }
    let total: u128 = plan.allocations.iter().map(|a| a.amount).sum();
    assert_eq!(total, 1_000_000);
}

// --- Task 6: Derisking tests ---

#[test]
fn test_should_derisk_hold_when_healthy() {
    let config = VaultConfig::default();
    let health = vec![HealthStatus {
        adapter_name: "sovereign_bond".into(),
        score: 0.9,
        oracle_fresh: true,
        liquidity_adequate: true,
        utilisation_rate: 0.2,
        details: "ok".into(),
    }];
    assert!(matches!(
        should_derisk(&config, &health),
        DeriskAction::Hold
    ));
}

#[test]
fn test_should_derisk_migrate_when_degraded() {
    let config = VaultConfig::default();
    let health = vec![HealthStatus {
        adapter_name: "aave_savings".into(),
        score: 0.3,
        oracle_fresh: true,
        liquidity_adequate: true,
        utilisation_rate: 0.8,
        details: "degraded".into(),
    }];
    assert!(matches!(
        should_derisk(&config, &health),
        DeriskAction::Migrate { .. }
    ));
}

#[test]
fn test_should_derisk_emergency_when_critical() {
    let config = VaultConfig::default();
    let health = vec![HealthStatus {
        adapter_name: "aave_savings".into(),
        score: 0.1,
        oracle_fresh: false,
        liquidity_adequate: false,
        utilisation_rate: 0.99,
        details: "critical".into(),
    }];
    assert!(matches!(
        should_derisk(&config, &health),
        DeriskAction::EmergencyWithdraw { .. }
    ));
}

#[test]
fn test_should_derisk_disabled_always_hold() {
    let mut config = VaultConfig::default();
    config.auto_derisk_enabled = false;

    // Even with critical health data, should return Hold when auto_derisk is disabled
    let health = vec![HealthStatus {
        adapter_name: "aave_savings".into(),
        score: 0.05,
        oracle_fresh: false,
        liquidity_adequate: false,
        utilisation_rate: 0.99,
        details: "critical".into(),
    }];
    assert!(
        matches!(should_derisk(&config, &health), DeriskAction::Hold),
        "auto_derisk disabled must always return Hold regardless of health"
    );
}
