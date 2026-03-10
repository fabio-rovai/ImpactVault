use impactvault::domain::adapters::compound_lending::CompoundLendingAdapter;
use impactvault::domain::adapters::YieldAdapter;
use impactvault::domain::engine::RiskSpectrum;

fn make_compound() -> CompoundLendingAdapter {
    CompoundLendingAdapter::new(
        "0xCometAddress000000000000000000000000000000".to_string(),
        "0xAssetAddress00000000000000000000000000000".to_string(),
        11155111, // Sepolia
        "https://rpc.sepolia.example.com".to_string(),
    )
}

#[test]
fn test_compound_lending_name() {
    let adapter = make_compound();
    assert_eq!(adapter.name(), "compound_lending");
}

#[test]
fn test_compound_lending_risk_position() {
    let adapter = make_compound();
    assert_eq!(adapter.risk_position(), RiskSpectrum::DiversifiedLending);
}

#[tokio::test]
async fn test_compound_deposit_calldata() {
    let adapter = make_compound();
    let tx = adapter.deposit(1_000_000).await.unwrap();

    assert_eq!(tx.chain_id, 11155111);
    assert!(!tx.data.is_empty(), "calldata should not be empty");
    assert_eq!(
        tx.to, "0xCometAddress000000000000000000000000000000",
        "to address should match Comet contract"
    );
    assert!(
        tx.data.starts_with("0xf2b9fdb8"),
        "deposit selector mismatch: expected Comet.supply selector"
    );
}

#[tokio::test]
async fn test_compound_health_check() {
    let adapter = make_compound();
    let health = adapter.health_check().await.unwrap();

    assert_eq!(health.adapter_name, "compound_lending");
    assert!(
        health.score > 0.5 && health.score < 1.0,
        "health score should be in reasonable range, got {}",
        health.score
    );
}

#[tokio::test]
async fn test_compound_yield_apy() {
    let adapter = make_compound();
    let apy = adapter.current_yield_apy().await.unwrap();

    assert!(
        apy > 0.0 && apy < 10.0,
        "APY should be between 0 and 10, got {}",
        apy
    );
}
