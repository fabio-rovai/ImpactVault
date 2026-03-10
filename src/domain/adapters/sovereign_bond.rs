use async_trait::async_trait;

use crate::domain::adapters::{TxRequest, YieldAdapter};
use crate::domain::engine::{HealthStatus, RiskSpectrum};

pub struct SovereignBondAdapter {
    contract_address: String,
    chain_id: u64,
    #[allow(dead_code)]
    rpc_url: String,
}

impl SovereignBondAdapter {
    pub fn new(contract_address: String, chain_id: u64, rpc_url: String) -> Self {
        Self {
            contract_address,
            chain_id,
            rpc_url,
        }
    }
}

#[async_trait]
impl YieldAdapter for SovereignBondAdapter {
    fn name(&self) -> &str {
        "sovereign_bond"
    }

    fn risk_position(&self) -> RiskSpectrum {
        RiskSpectrum::Sovereign
    }

    async fn deposit(&self, amount: u128) -> anyhow::Result<TxRequest> {
        // ERC-4626 deposit(uint256,address) selector: 0x6e553f65
        // ABI-encode: amount as uint256 (32 bytes) + receiver as address (32 bytes, zero-padded)
        let amount_hex = format!("{amount:064x}");
        // Placeholder receiver: zero address (will be replaced by wallet at signing time)
        let receiver_hex = format!("{:064x}", 0u128);
        let calldata = format!("0x6e553f65{amount_hex}{receiver_hex}");

        Ok(TxRequest {
            to: self.contract_address.clone(),
            data: calldata,
            value: "0".to_string(),
            chain_id: self.chain_id,
        })
    }

    async fn withdraw(&self, amount: u128) -> anyhow::Result<TxRequest> {
        // ERC-4626 withdraw(uint256,address,address) selector: 0xb460af94
        let amount_hex = format!("{amount:064x}");
        let receiver_hex = format!("{:064x}", 0u128);
        let owner_hex = format!("{:064x}", 0u128);
        let calldata = format!("0xb460af94{amount_hex}{receiver_hex}{owner_hex}");

        Ok(TxRequest {
            to: self.contract_address.clone(),
            data: calldata,
            value: "0".to_string(),
            chain_id: self.chain_id,
        })
    }

    async fn current_yield_apy(&self) -> anyhow::Result<f64> {
        // Mock for testnet: sovereign bond ~4.5% APY
        Ok(4.5)
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        // Mock for testnet: sovereign bonds are the safest tier
        Ok(HealthStatus {
            adapter_name: self.name().to_string(),
            score: 0.95,
            oracle_fresh: true,
            liquidity_adequate: true,
            utilisation_rate: 0.0,
            details: "Sovereign bond vault healthy (testnet mock)".to_string(),
        })
    }

    async fn tvl(&self) -> anyhow::Result<u128> {
        // Mock: no on-chain query yet
        Ok(0)
    }
}
