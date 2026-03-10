use impactvault::domain::adapters::liquid_staking::LiquidStakingAdapter;
use impactvault::domain::adapters::YieldAdapter;
use impactvault::domain::engine::RiskSpectrum;

fn make_liquid_staking() -> LiquidStakingAdapter {
    LiquidStakingAdapter::new(
        "0xwstETHAddress0000000000000000000000000000".to_string(),
        11155111, // Sepolia
        "https://rpc.sepolia.example.com".to_string(),
    )
}

#[test]
fn test_liquid_staking_name() {
    let adapter = make_liquid_staking();
    assert_eq!(adapter.name(), "liquid_staking");
}

#[test]
fn test_liquid_staking_risk_position() {
    let adapter = make_liquid_staking();
    assert_eq!(adapter.risk_position(), RiskSpectrum::LiquidStaking);
}

#[tokio::test]
async fn test_liquid_staking_deposit_calldata() {
    let adapter = make_liquid_staking();
    let tx = adapter.deposit(1_000_000).await.unwrap();

    assert_eq!(tx.chain_id, 11155111);
    assert!(!tx.data.is_empty(), "calldata should not be empty");
    assert_eq!(
        tx.to, "0xwstETHAddress0000000000000000000000000000",
        "to address should match wstETH contract"
    );
    assert!(
        tx.data.starts_with("0xea598cb0"),
        "deposit selector mismatch: expected wstETH.wrap selector"
    );
}

#[tokio::test]
async fn test_liquid_staking_health_check() {
    let adapter = make_liquid_staking();
    let health = adapter.health_check().await.unwrap();

    assert_eq!(health.adapter_name, "liquid_staking");
    assert!(
        health.score > 0.5 && health.score < 1.0,
        "health score should be in reasonable range, got {}",
        health.score
    );
}

#[tokio::test]
async fn test_liquid_staking_yield_apy() {
    let adapter = make_liquid_staking();
    let apy = adapter.current_yield_apy().await.unwrap();

    assert!(
        apy > 0.0 && apy < 10.0,
        "APY should be between 0 and 10, got {}",
        apy
    );
}
