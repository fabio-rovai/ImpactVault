use async_trait::async_trait;

use crate::domain::adapters::{TxRequest, YieldAdapter};
use crate::domain::engine::{HealthStatus, RiskSpectrum};

pub struct LiquidStakingAdapter {
    wsteth_address: String,
    chain_id: u64,
    #[allow(dead_code)]
    rpc_url: String,
}

impl LiquidStakingAdapter {
    pub fn new(wsteth_address: String, chain_id: u64, rpc_url: String) -> Self {
        Self {
            wsteth_address,
            chain_id,
            rpc_url,
        }
    }
}

#[async_trait]
impl YieldAdapter for LiquidStakingAdapter {
    fn name(&self) -> &str {
        "liquid_staking"
    }

    fn risk_position(&self) -> RiskSpectrum {
        RiskSpectrum::LiquidStaking
    }

    async fn deposit(&self, amount: u128) -> anyhow::Result<TxRequest> {
        // wstETH.wrap(uint256 _stETHAmount) selector: 0xea598cb0
        let amount_hex = format!("{amount:064x}");
        let calldata = format!("0xea598cb0{amount_hex}");

        Ok(TxRequest {
            to: self.wsteth_address.clone(),
            data: calldata,
            value: "0".to_string(),
            chain_id: self.chain_id,
        })
    }

    async fn withdraw(&self, amount: u128) -> anyhow::Result<TxRequest> {
        // wstETH.unwrap(uint256 _wstETHAmount) selector: 0xde0e9a3e
        let amount_hex = format!("{amount:064x}");
        let calldata = format!("0xde0e9a3e{amount_hex}");

        Ok(TxRequest {
            to: self.wsteth_address.clone(),
            data: calldata,
            value: "0".to_string(),
            chain_id: self.chain_id,
        })
    }

    async fn current_yield_apy(&self) -> anyhow::Result<f64> {
        // Mock for testnet: Lido PoS staking rewards ~3.5% APY
        Ok(3.5)
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        // Mock for testnet: Lido liquid staking healthy
        Ok(HealthStatus {
            adapter_name: self.name().to_string(),
            score: 0.88,
            oracle_fresh: true,
            liquidity_adequate: true,
            utilisation_rate: 0.0,
            details: "Lido wstETH liquid staking healthy (testnet mock)".to_string(),
        })
    }

    async fn tvl(&self) -> anyhow::Result<u128> {
        // Mock: no on-chain query yet
        Ok(0)
    }
}
