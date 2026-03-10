use async_trait::async_trait;

use crate::domain::adapters::{TxRequest, YieldAdapter};
use crate::domain::engine::{HealthStatus, RiskSpectrum};

pub struct CompoundLendingAdapter {
    comet_address: String,
    asset_address: String,
    chain_id: u64,
    #[allow(dead_code)]
    rpc_url: String,
}

impl CompoundLendingAdapter {
    pub fn new(
        comet_address: String,
        asset_address: String,
        chain_id: u64,
        rpc_url: String,
    ) -> Self {
        Self {
            comet_address,
            asset_address,
            chain_id,
            rpc_url,
        }
    }

    /// Encode a 20-byte address as a left-zero-padded 32-byte ABI word.
    fn encode_address(addr: &str) -> String {
        let stripped = addr.strip_prefix("0x").unwrap_or(addr);
        format!("{stripped:0>64}")
    }
}

#[async_trait]
impl YieldAdapter for CompoundLendingAdapter {
    fn name(&self) -> &str {
        "compound_lending"
    }

    fn risk_position(&self) -> RiskSpectrum {
        RiskSpectrum::DiversifiedLending
    }

    async fn deposit(&self, amount: u128) -> anyhow::Result<TxRequest> {
        // Comet.supply(address asset, uint256 amount) selector: 0xf2b9fdb8
        let asset_word = Self::encode_address(&self.asset_address);
        let amount_hex = format!("{amount:064x}");
        let calldata = format!("0xf2b9fdb8{asset_word}{amount_hex}");

        Ok(TxRequest {
            to: self.comet_address.clone(),
            data: calldata,
            value: "0".to_string(),
            chain_id: self.chain_id,
        })
    }

    async fn withdraw(&self, amount: u128) -> anyhow::Result<TxRequest> {
        // Comet.withdraw(address asset, uint256 amount) selector: 0xf3fef3a3
        let asset_word = Self::encode_address(&self.asset_address);
        let amount_hex = format!("{amount:064x}");
        let calldata = format!("0xf3fef3a3{asset_word}{amount_hex}");

        Ok(TxRequest {
            to: self.comet_address.clone(),
            data: calldata,
            value: "0".to_string(),
            chain_id: self.chain_id,
        })
    }

    async fn current_yield_apy(&self) -> anyhow::Result<f64> {
        // Mock for testnet: Compound V3 lending ~3.2% APY
        Ok(3.2)
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        // Mock for testnet: Compound V3 Comet healthy
        Ok(HealthStatus {
            adapter_name: self.name().to_string(),
            score: 0.82,
            oracle_fresh: true,
            liquidity_adequate: true,
            utilisation_rate: 0.68,
            details: "Compound V3 Comet healthy, utilisation moderate (testnet mock)".to_string(),
        })
    }

    async fn tvl(&self) -> anyhow::Result<u128> {
        // Mock: no on-chain query yet
        Ok(0)
    }
}
