use impactvault::domain::adapters::aave_savings::AaveSavingsAdapter;
use impactvault::domain::adapters::sovereign_bond::SovereignBondAdapter;
use impactvault::domain::adapters::YieldAdapter;
use impactvault::domain::engine::RiskSpectrum;

fn make_sovereign() -> SovereignBondAdapter {
    SovereignBondAdapter::new(
        "0x1234567890abcdef1234567890abcdef12345678".to_string(),
        11155111, // Sepolia
        "https://rpc.sepolia.example.com".to_string(),
    )
}

fn make_aave() -> AaveSavingsAdapter {
    AaveSavingsAdapter::new(
        "0xPoolAddress000000000000000000000000000000".to_string(),
        "0xAssetAddress00000000000000000000000000000".to_string(),
        11155111,
        "https://rpc.sepolia.example.com".to_string(),
    )
}

// --- SovereignBondAdapter tests ---

#[test]
fn sovereign_bond_metadata() {
    let adapter = make_sovereign();
    assert_eq!(adapter.name(), "sovereign_bond");
    assert_eq!(adapter.risk_position(), RiskSpectrum::Sovereign);
}

#[tokio::test]
async fn sovereign_bond_deposit_returns_valid_tx() {
    let adapter = make_sovereign();
    let tx = adapter.deposit(1_000_000).await.unwrap();

    assert_eq!(tx.to, "0x1234567890abcdef1234567890abcdef12345678");
    assert!(tx.data.starts_with("0x6e553f65"), "deposit selector mismatch");
    assert_eq!(tx.value, "0");
    assert_eq!(tx.chain_id, 11155111);
}

#[tokio::test]
async fn sovereign_bond_withdraw_returns_valid_tx() {
    let adapter = make_sovereign();
    let tx = adapter.withdraw(500_000).await.unwrap();

    assert_eq!(tx.to, "0x1234567890abcdef1234567890abcdef12345678");
    assert!(
        tx.data.starts_with("0xb460af94"),
        "withdraw selector mismatch"
    );
    assert_eq!(tx.value, "0");
    assert_eq!(tx.chain_id, 11155111);
}

#[tokio::test]
async fn sovereign_bond_health_check_returns_healthy() {
    let adapter = make_sovereign();
    let health = adapter.health_check().await.unwrap();

    assert_eq!(health.adapter_name, "sovereign_bond");
    assert!(health.score > 0.9, "sovereign bond should be highly healthy");
    assert!(health.oracle_fresh);
    assert!(health.liquidity_adequate);
}

// --- AaveSavingsAdapter tests ---

#[test]
fn aave_savings_metadata() {
    let adapter = make_aave();
    assert_eq!(adapter.name(), "aave_savings");
    assert_eq!(adapter.risk_position(), RiskSpectrum::StablecoinSavings);
}

#[tokio::test]
async fn aave_savings_deposit_contains_correct_selector() {
    let adapter = make_aave();
    let tx = adapter.deposit(2_000_000).await.unwrap();

    assert!(
        tx.data.starts_with("0x617ba037"),
        "Aave supply selector mismatch"
    );
    assert_eq!(tx.value, "0");
    assert_eq!(tx.chain_id, 11155111);
}

#[tokio::test]
async fn aave_savings_withdraw_contains_correct_selector() {
    let adapter = make_aave();
    let tx = adapter.withdraw(1_000_000).await.unwrap();

    assert!(
        tx.data.starts_with("0x69328dec"),
        "Aave withdraw selector mismatch"
    );
    assert_eq!(tx.value, "0");
    assert_eq!(tx.chain_id, 11155111);
}

#[tokio::test]
async fn aave_savings_health_check_returns_adequate() {
    let adapter = make_aave();
    let health = adapter.health_check().await.unwrap();

    assert_eq!(health.adapter_name, "aave_savings");
    assert!(health.score > 0.5, "aave should report adequate health");
    assert!(health.oracle_fresh);
    assert!(health.liquidity_adequate);
    assert!(
        health.utilisation_rate > 0.0,
        "aave should report non-zero utilisation"
    );
}

// --- Trait object test ---

#[tokio::test]
async fn both_adapters_work_as_trait_objects() {
    let adapters: Vec<Box<dyn YieldAdapter>> =
        vec![Box::new(make_sovereign()), Box::new(make_aave())];

    assert_eq!(adapters.len(), 2);

    for adapter in &adapters {
        // Each adapter should be callable through the trait interface
        let _tx = adapter.deposit(100).await.unwrap();
        let _yield_apy = adapter.current_yield_apy().await.unwrap();
        let health = adapter.health_check().await.unwrap();
        assert!(health.score > 0.0);

        let _tvl = adapter.tvl().await.unwrap();
    }

    // Verify distinct identities through trait
    assert_eq!(adapters[0].name(), "sovereign_bond");
    assert_eq!(adapters[1].name(), "aave_savings");
    assert_eq!(adapters[0].risk_position(), RiskSpectrum::Sovereign);
    assert_eq!(adapters[1].risk_position(), RiskSpectrum::StablecoinSavings);
}
