use async_trait::async_trait;

use crate::domain::adapters::{TxRequest, YieldAdapter};
use crate::domain::engine::{HealthStatus, RiskSpectrum};

pub struct AaveSavingsAdapter {
    pool_address: String,
    asset_address: String,
    chain_id: u64,
    #[allow(dead_code)]
    rpc_url: String,
}

impl AaveSavingsAdapter {
    pub fn new(
        pool_address: String,
        asset_address: String,
        chain_id: u64,
        rpc_url: String,
    ) -> Self {
        Self {
            pool_address,
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
impl YieldAdapter for AaveSavingsAdapter {
    fn name(&self) -> &str {
        "aave_savings"
    }

    fn risk_position(&self) -> RiskSpectrum {
        RiskSpectrum::StablecoinSavings
    }

    async fn deposit(&self, amount: u128) -> anyhow::Result<TxRequest> {
        // Aave V3 Pool.supply(address asset, uint256 amount, address onBehalfOf, uint16 referralCode)
        // selector: 0x617ba037
        let asset_word = Self::encode_address(&self.asset_address);
        let amount_hex = format!("{amount:064x}");
        // onBehalfOf: zero address placeholder (replaced at signing time)
        let on_behalf_of = format!("{:064x}", 0u128);
        // referralCode: 0
        let referral = format!("{:064x}", 0u128);
        let calldata = format!("0x617ba037{asset_word}{amount_hex}{on_behalf_of}{referral}");

        Ok(TxRequest {
            to: self.pool_address.clone(),
            data: calldata,
            value: "0".to_string(),
            chain_id: self.chain_id,
        })
    }

    async fn withdraw(&self, amount: u128) -> anyhow::Result<TxRequest> {
        // Aave V3 Pool.withdraw(address asset, uint256 amount, address to)
        // selector: 0x69328dec
        let asset_word = Self::encode_address(&self.asset_address);
        let amount_hex = format!("{amount:064x}");
        // to: zero address placeholder
        let to_addr = format!("{:064x}", 0u128);
        let calldata = format!("0x69328dec{asset_word}{amount_hex}{to_addr}");

        Ok(TxRequest {
            to: self.pool_address.clone(),
            data: calldata,
            value: "0".to_string(),
            chain_id: self.chain_id,
        })
    }

    async fn current_yield_apy(&self) -> anyhow::Result<f64> {
        // Mock for testnet: Aave stablecoin savings ~3.2% APY
        Ok(3.2)
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        // Mock for testnet: typical healthy Aave pool
        Ok(HealthStatus {
            adapter_name: self.name().to_string(),
            score: 0.85,
            oracle_fresh: true,
            liquidity_adequate: true,
            utilisation_rate: 0.72,
            details: "Aave savings pool healthy, utilisation normal (testnet mock)".to_string(),
        })
    }

    async fn tvl(&self) -> anyhow::Result<u128> {
        // Mock: no on-chain query yet
        Ok(0)
    }
}
