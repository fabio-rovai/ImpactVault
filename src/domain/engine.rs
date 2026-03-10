use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskSpectrum {
    Sovereign,
    StablecoinSavings,
    LiquidStaking,
    DiversifiedLending,
    MultiStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub approved_sources: Vec<RiskSpectrum>,
    pub max_exposure_per_source: u8,       // percentage 0-100
    pub concentration_limit: u8,           // max % in single adapter
    pub derisking_health_threshold: f64,   // 0.0-1.0
    pub auto_derisk_enabled: bool,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            approved_sources: vec![RiskSpectrum::Sovereign],
            max_exposure_per_source: 100,
            concentration_limit: 80,
            derisking_health_threshold: 0.5,
            auto_derisk_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Allocation {
    pub source: RiskSpectrum,
    pub adapter_name: String,
    pub amount: u128, // wei
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Portfolio {
    allocations: Vec<Allocation>,
}

impl Portfolio {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_deposited(&self) -> u128 {
        self.allocations.iter().map(|a| a.amount).sum()
    }

    pub fn allocations(&self) -> &[Allocation] {
        &self.allocations
    }

    pub fn add_allocation(&mut self, alloc: Allocation) {
        self.allocations.push(alloc);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub adapter_name: String,
    pub score: f64,           // 0.0 (critical) to 1.0 (healthy)
    pub oracle_fresh: bool,
    pub liquidity_adequate: bool,
    pub utilisation_rate: f64,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeriskAction {
    Hold,
    Migrate { from: String, to: RiskSpectrum },
    EmergencyWithdraw { adapter: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_health: f64,
    pub breaches: Vec<String>,
    pub recommended_action: DeriskAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationPlan {
    pub allocations: Vec<Allocation>,
}

// --- Task 4: Risk Evaluation Logic ---

pub fn evaluate_risk(
    config: &VaultConfig,
    portfolio: &Portfolio,
    health_data: &[HealthStatus],
) -> RiskAssessment {
    let mut breaches = Vec::new();
    let total = portfolio.total_deposited();

    let overall_health = if health_data.is_empty() {
        1.0
    } else {
        health_data.iter().map(|h| h.score).sum::<f64>() / health_data.len() as f64
    };

    // Check individual adapter health
    for h in health_data {
        if h.score < config.derisking_health_threshold {
            breaches.push(format!(
                "health_breach: {} score {:.2} < threshold {:.2}",
                h.adapter_name, h.score, config.derisking_health_threshold
            ));
        }
        if !h.oracle_fresh {
            breaches.push(format!("oracle_stale: {}", h.adapter_name));
        }
        if !h.liquidity_adequate {
            breaches.push(format!("liquidity_low: {}", h.adapter_name));
        }
    }

    // Check concentration limits
    if total > 0 {
        for alloc in portfolio.allocations() {
            let pct = (alloc.amount as f64 / total as f64 * 100.0) as u8;
            if pct > config.concentration_limit {
                breaches.push(format!(
                    "concentration_breach: {} at {}% exceeds {}% limit",
                    alloc.adapter_name, pct, config.concentration_limit
                ));
            }
        }
    }

    let recommended_action = if breaches.is_empty() {
        DeriskAction::Hold
    } else {
        // Find the worst-scoring adapter across all health data
        let worst = health_data
            .iter()
            .min_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        match worst {
            Some(h) if h.score < 0.2 => DeriskAction::EmergencyWithdraw {
                adapter: h.adapter_name.clone(),
            },
            Some(h) => DeriskAction::Migrate {
                from: h.adapter_name.clone(),
                to: RiskSpectrum::Sovereign,
            },
            // Breaches exist but no health data (e.g., concentration-only breach)
            None => DeriskAction::Hold,
        }
    };

    RiskAssessment {
        overall_health,
        breaches,
        recommended_action,
    }
}

// --- Task 5: Allocation Logic ---

pub fn recommend_allocation(config: &VaultConfig, deposit_amount: u128) -> AllocationPlan {
    let sources = &config.approved_sources;
    if sources.is_empty() {
        return AllocationPlan {
            allocations: vec![],
        };
    }

    if sources.len() == 1 {
        return AllocationPlan {
            allocations: vec![Allocation {
                source: sources[0],
                adapter_name: adapter_name_for(sources[0]),
                amount: deposit_amount,
            }],
        };
    }

    let max_per_source = deposit_amount * config.concentration_limit as u128 / 100;
    let mut remaining = deposit_amount;
    let mut allocations = Vec::new();

    for &source in sources {
        if remaining == 0 {
            break;
        }
        let amount = remaining.min(max_per_source);
        allocations.push(Allocation {
            source,
            adapter_name: adapter_name_for(source),
            amount,
        });
        remaining -= amount;
    }

    if remaining > 0 {
        if let Some(first) = allocations.first_mut() {
            first.amount += remaining;
        }
    }

    AllocationPlan { allocations }
}

fn adapter_name_for(spectrum: RiskSpectrum) -> String {
    match spectrum {
        RiskSpectrum::Sovereign => "sovereign_bond".into(),
        RiskSpectrum::StablecoinSavings => "aave_savings".into(),
        RiskSpectrum::LiquidStaking => "liquid_staking".into(),
        RiskSpectrum::DiversifiedLending => "compound_lending".into(),
        RiskSpectrum::MultiStrategy => "multi_strategy".into(),
    }
}

// --- Task 6: Derisking Logic ---

pub fn should_derisk(config: &VaultConfig, health_data: &[HealthStatus]) -> DeriskAction {
    if !config.auto_derisk_enabled {
        return DeriskAction::Hold;
    }

    let mut worst_score = 1.0_f64;
    let mut worst_adapter = String::new();

    for h in health_data {
        if h.score < worst_score {
            worst_score = h.score;
            worst_adapter = h.adapter_name.clone();
        }
    }

    if worst_score >= config.derisking_health_threshold {
        DeriskAction::Hold
    } else if worst_score < 0.2 {
        DeriskAction::EmergencyWithdraw {
            adapter: worst_adapter,
        }
    } else {
        DeriskAction::Migrate {
            from: worst_adapter,
            to: RiskSpectrum::Sovereign,
        }
    }
}
