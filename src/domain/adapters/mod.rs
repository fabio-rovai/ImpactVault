pub mod aave_savings;
pub mod compound_lending;
pub mod liquid_staking;
pub mod sovereign_bond;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::engine::{HealthStatus, RiskSpectrum};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRequest {
    pub to: String,    // contract address
    pub data: String,  // hex-encoded calldata
    pub value: String, // wei as string
    pub chain_id: u64,
}

#[async_trait]
pub trait YieldAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn risk_position(&self) -> RiskSpectrum;
    async fn deposit(&self, amount: u128) -> anyhow::Result<TxRequest>;
    async fn withdraw(&self, amount: u128) -> anyhow::Result<TxRequest>;
    async fn current_yield_apy(&self) -> anyhow::Result<f64>;
    async fn health_check(&self) -> anyhow::Result<HealthStatus>;
    async fn tvl(&self) -> anyhow::Result<u128>;
}
