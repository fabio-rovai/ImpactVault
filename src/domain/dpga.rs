use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpgEntry {
    pub name: String,
    pub description: String,
    pub website: String,
    pub repositories: Vec<String>,
    pub stage: String,
    #[serde(default)]
    pub wallet_address: Option<String>,
}

pub fn suggest_recipients(dpgs: &[DpgEntry]) -> Vec<&DpgEntry> {
    dpgs.iter()
        .filter(|d| !d.repositories.is_empty() && d.wallet_address.is_some())
        .collect()
}

pub async fn fetch_dpgs(api_url: &str) -> anyhow::Result<Vec<DpgEntry>> {
    let resp = reqwest::get(api_url).await?;
    let dpgs: Vec<DpgEntry> = resp.json().await?;
    Ok(dpgs)
}
