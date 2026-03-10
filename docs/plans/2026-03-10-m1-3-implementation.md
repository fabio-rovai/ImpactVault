# ImpactVault M1-3 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the ImpactVault risk-curated yield infrastructure — core engine, two adapters, smart contracts, sentinel bot, and MCP/REST API — operational on Sepolia testnet.

**Architecture:** Fork of OpenCheir (Rust MCP server). Strip document-specific modules, keep enforcer/lineage/patterns/state infrastructure, add risk engine + adapter + sentinel + contracts domains.

**Tech Stack:** Rust (stable), Axum, Tokio, rmcp, alloy, rusqlite, Foundry (Solidity), Sepolia testnet, Aave V3

---

## Phase 1: Project Setup

### Task 1: Fork OpenCheir and Strip Document Modules

**Files:**
- Copy: `/Users/fabio/Projects/opencheir/` → `/Users/fabio/Projects/impactvault/`
- Delete: `src/domain/qa.rs`, `src/domain/eyes.rs`
- Delete: `src/store/documents.rs`, `src/store/search.rs`
- Delete: `src/orchestration/hive/`, `src/orchestration/skills.rs`, `src/orchestration/supervisor.rs`
- Modify: `Cargo.toml` — rename package, remove unused deps (docx-rs, image, rust-stemmers)
- Modify: `src/lib.rs`, `src/domain/mod.rs`, `src/orchestration/mod.rs`, `src/store/mod.rs` — remove stripped module declarations
- Modify: `src/gateway/router.rs` — remove routes for stripped modules
- Modify: `src/gateway/server.rs` — remove tool definitions for stripped modules
- Modify: `src/main.rs` — rename binary to `impactvault`, update init/serve

**Step 1: Copy the project**

```bash
cp -r /Users/fabio/Projects/opencheir /Users/fabio/Projects/impactvault
cd /Users/fabio/Projects/impactvault
rm -rf .git target
git init
```

**Step 2: Delete document-specific files**

```bash
rm src/domain/qa.rs src/domain/eyes.rs
rm src/store/documents.rs src/store/search.rs
rm -rf src/orchestration/hive src/orchestration/skills.rs src/orchestration/supervisor.rs
rm -rf static  # eyes UI assets
```

**Step 3: Update Cargo.toml**

Change `name = "opencheir"` to `name = "impactvault"`, binary name to `impactvault`.
Remove deps: `docx-rs`, `image`, `rust-stemmers`, `walkdir`, `glob`.
Add deps: `alloy = { version = "1", features = ["providers", "contract", "signers", "network"] }`, `async-trait = "0.1"`.

**Step 4: Fix module declarations**

Update `lib.rs`, `domain/mod.rs`, `orchestration/mod.rs`, `store/mod.rs` to remove references to deleted modules.

**Step 5: Clean gateway — remove stripped tool definitions and routes**

In `server.rs`, remove all tool functions for qa_*, eyes_*, search_*, doc_*, hive_*, skill_*, opencheir_* (keep enforcer_*, lineage_*, pattern_*).
In `router.rs`, remove routes for stripped prefixes.

**Step 6: Update main.rs**

Rename CLI from `opencheir` to `impactvault`. Keep `init` and `serve` subcommands.

**Step 7: Verify it compiles**

```bash
cargo build
```

**Step 8: Commit**

```bash
git add -A
git commit -m "fork: strip OpenCheir to ImpactVault skeleton

Remove document QA, eyes, DOCX parsing, search, hive orchestration,
skills, and supervisor. Keep enforcer, lineage, patterns, state,
config, and MCP server infrastructure."
```

---

### Task 2: Install Foundry and Initialize Contracts Project

**Step 1: Install Foundry**

```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

**Step 2: Initialize Foundry project**

```bash
cd /Users/fabio/Projects/impactvault
forge init contracts --no-git
```

**Step 3: Add OpenZeppelin**

```bash
cd contracts
forge install OpenZeppelin/openzeppelin-contracts --no-git
```

**Step 4: Configure foundry.toml**

```toml
[profile.default]
src = "src"
out = "out"
libs = ["lib"]
solc_version = "0.8.24"

[profile.default.rpc_endpoints]
sepolia = "${SEPOLIA_RPC_URL}"
```

**Step 5: Verify**

```bash
forge build
```

**Step 6: Commit**

```bash
cd /Users/fabio/Projects/impactvault
git add contracts/
git commit -m "feat: initialize Foundry contracts project with OpenZeppelin"
```

---

## Phase 2: Core Risk Engine

### Task 3: Risk Engine Types

**Files:**
- Create: `src/domain/engine.rs`
- Modify: `src/domain/mod.rs` — add `pub mod engine;`
- Create: `tests/engine_test.rs`

**Step 1: Write the failing test**

```rust
// tests/engine_test.rs
use impactvault::domain::engine::*;

#[test]
fn test_risk_spectrum_ordering() {
    assert!(RiskSpectrum::Sovereign < RiskSpectrum::StablecoinSavings);
}

#[test]
fn test_vault_config_default_has_safe_limits() {
    let config = VaultConfig::default();
    assert!(config.max_exposure_per_source <= 100);
    assert!(config.concentration_limit <= 100);
    assert!(config.derisking_health_threshold > 0.0);
}

#[test]
fn test_portfolio_empty_on_creation() {
    let portfolio = Portfolio::new();
    assert_eq!(portfolio.total_deposited(), 0);
    assert!(portfolio.allocations().is_empty());
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test engine_test
```
Expected: FAIL — module not found.

**Step 3: Implement types**

```rust
// src/domain/engine.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskSpectrum {
    Sovereign,
    StablecoinSavings,
    // Future: LiquidStaking, DiversifiedLending, MultiStrategy
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
    pub amount: u128,  // wei
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
    pub score: f64,          // 0.0 (critical) to 1.0 (healthy)
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
```

**Step 4: Run test to verify it passes**

```bash
cargo test --test engine_test
```
Expected: PASS

**Step 5: Commit**

```bash
git add src/domain/engine.rs src/domain/mod.rs tests/engine_test.rs
git commit -m "feat: add core risk engine types"
```

---

### Task 4: Risk Evaluation Logic

**Files:**
- Modify: `src/domain/engine.rs`
- Modify: `tests/engine_test.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn test_evaluate_risk_healthy_portfolio() {
    let config = VaultConfig::default();
    let mut portfolio = Portfolio::new();
    portfolio.add_allocation(Allocation {
        source: RiskSpectrum::Sovereign,
        adapter_name: "sovereign_bond".into(),
        amount: 1_000_000,
    });

    let health = vec![HealthStatus {
        adapter_name: "sovereign_bond".into(),
        score: 0.95,
        oracle_fresh: true,
        liquidity_adequate: true,
        utilisation_rate: 0.3,
        details: "healthy".into(),
    }];

    let assessment = evaluate_risk(&config, &portfolio, &health);
    assert!(assessment.overall_health > 0.8);
    assert!(assessment.breaches.is_empty());
    assert!(matches!(assessment.recommended_action, DeriskAction::Hold));
}

#[test]
fn test_evaluate_risk_unhealthy_triggers_derisk() {
    let config = VaultConfig::default(); // threshold 0.5
    let mut portfolio = Portfolio::new();
    portfolio.add_allocation(Allocation {
        source: RiskSpectrum::StablecoinSavings,
        adapter_name: "aave_savings".into(),
        amount: 1_000_000,
    });

    let health = vec![HealthStatus {
        adapter_name: "aave_savings".into(),
        score: 0.3,
        oracle_fresh: false,
        liquidity_adequate: false,
        utilisation_rate: 0.95,
        details: "critical".into(),
    }];

    let assessment = evaluate_risk(&config, &portfolio, &health);
    assert!(assessment.overall_health < 0.5);
    assert!(!assessment.breaches.is_empty());
    assert!(!matches!(assessment.recommended_action, DeriskAction::Hold));
}

#[test]
fn test_evaluate_risk_concentration_breach() {
    let mut config = VaultConfig::default();
    config.approved_sources = vec![RiskSpectrum::Sovereign, RiskSpectrum::StablecoinSavings];
    config.concentration_limit = 50;

    let mut portfolio = Portfolio::new();
    portfolio.add_allocation(Allocation {
        source: RiskSpectrum::StablecoinSavings,
        adapter_name: "aave_savings".into(),
        amount: 800_000,
    });
    portfolio.add_allocation(Allocation {
        source: RiskSpectrum::Sovereign,
        adapter_name: "sovereign_bond".into(),
        amount: 200_000,
    });

    let health = vec![
        HealthStatus {
            adapter_name: "aave_savings".into(),
            score: 0.9, oracle_fresh: true, liquidity_adequate: true,
            utilisation_rate: 0.3, details: "ok".into(),
        },
        HealthStatus {
            adapter_name: "sovereign_bond".into(),
            score: 0.95, oracle_fresh: true, liquidity_adequate: true,
            utilisation_rate: 0.1, details: "ok".into(),
        },
    ];

    let assessment = evaluate_risk(&config, &portfolio, &health);
    assert!(assessment.breaches.iter().any(|b| b.contains("concentration")));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test engine_test
```
Expected: FAIL — `evaluate_risk` not found.

**Step 3: Implement evaluate_risk**

```rust
// Add to src/domain/engine.rs

pub fn evaluate_risk(
    config: &VaultConfig,
    portfolio: &Portfolio,
    health_data: &[HealthStatus],
) -> RiskAssessment {
    let mut breaches = Vec::new();
    let total = portfolio.total_deposited();

    // Check adapter health scores
    let overall_health = if health_data.is_empty() {
        1.0
    } else {
        health_data.iter().map(|h| h.score).sum::<f64>() / health_data.len() as f64
    };

    // Check individual adapter health
    for h in health_data {
        if h.score < config.derisking_health_threshold {
            breaches.push(format!("health_breach: {} score {:.2} < threshold {:.2}",
                h.adapter_name, h.score, config.derisking_health_threshold));
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

    // Determine action
    let recommended_action = if breaches.is_empty() {
        DeriskAction::Hold
    } else {
        // Find the unhealthiest non-sovereign adapter
        let worst = health_data.iter()
            .filter(|h| h.score < config.derisking_health_threshold)
            .min_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));

        match worst {
            Some(h) if h.score < 0.2 => DeriskAction::EmergencyWithdraw {
                adapter: h.adapter_name.clone(),
            },
            Some(h) => DeriskAction::Migrate {
                from: h.adapter_name.clone(),
                to: RiskSpectrum::Sovereign,
            },
            None => DeriskAction::Hold, // concentration breach only, no critical health
        }
    };

    RiskAssessment {
        overall_health,
        breaches,
        recommended_action,
    }
}
```

**Step 4: Run tests**

```bash
cargo test --test engine_test
```
Expected: PASS

**Step 5: Commit**

```bash
git add src/domain/engine.rs tests/engine_test.rs
git commit -m "feat: implement risk evaluation logic with health/concentration checks"
```

---

### Task 5: Allocation Logic

**Files:**
- Modify: `src/domain/engine.rs`
- Modify: `tests/engine_test.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn test_recommend_allocation_single_source() {
    let config = VaultConfig::default(); // only Sovereign approved
    let plan = recommend_allocation(&config, 1_000_000);
    assert_eq!(plan.allocations.len(), 1);
    assert_eq!(plan.allocations[0].source, RiskSpectrum::Sovereign);
    assert_eq!(plan.allocations[0].amount, 1_000_000);
}

#[test]
fn test_recommend_allocation_two_sources_respects_concentration() {
    let mut config = VaultConfig::default();
    config.approved_sources = vec![RiskSpectrum::Sovereign, RiskSpectrum::StablecoinSavings];
    config.concentration_limit = 60;

    let plan = recommend_allocation(&config, 1_000_000);
    assert_eq!(plan.allocations.len(), 2);
    for alloc in &plan.allocations {
        let pct = (alloc.amount as f64 / 1_000_000.0 * 100.0) as u8;
        assert!(pct <= 60, "allocation {}% exceeds limit", pct);
    }
    let total: u128 = plan.allocations.iter().map(|a| a.amount).sum();
    assert_eq!(total, 1_000_000);
}
```

**Step 2: Implement**

```rust
pub fn recommend_allocation(config: &VaultConfig, deposit_amount: u128) -> AllocationPlan {
    let sources = &config.approved_sources;
    if sources.is_empty() {
        return AllocationPlan { allocations: vec![] };
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

    // Distribute favoring safer (lower-ordinal) sources, respecting concentration limit
    let max_per_source = deposit_amount * config.concentration_limit as u128 / 100;
    let mut remaining = deposit_amount;
    let mut allocations = Vec::new();

    for &source in sources {
        if remaining == 0 { break; }
        let amount = remaining.min(max_per_source);
        allocations.push(Allocation {
            source,
            adapter_name: adapter_name_for(source),
            amount,
        });
        remaining -= amount;
    }

    // If remaining (shouldn't happen with proper config), add to safest
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
    }
}
```

**Step 3: Run tests, commit**

```bash
cargo test --test engine_test
git add src/domain/engine.rs tests/engine_test.rs
git commit -m "feat: implement allocation logic with concentration limits"
```

---

### Task 6: Derisking Logic

**Files:**
- Modify: `src/domain/engine.rs`
- Modify: `tests/engine_test.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn test_should_derisk_hold_when_healthy() {
    let config = VaultConfig::default();
    let health = vec![HealthStatus {
        adapter_name: "sovereign_bond".into(),
        score: 0.9, oracle_fresh: true, liquidity_adequate: true,
        utilisation_rate: 0.2, details: "ok".into(),
    }];
    let action = should_derisk(&config, &health);
    assert!(matches!(action, DeriskAction::Hold));
}

#[test]
fn test_should_derisk_migrate_when_degraded() {
    let config = VaultConfig::default();
    let health = vec![HealthStatus {
        adapter_name: "aave_savings".into(),
        score: 0.3, oracle_fresh: true, liquidity_adequate: true,
        utilisation_rate: 0.8, details: "degraded".into(),
    }];
    let action = should_derisk(&config, &health);
    assert!(matches!(action, DeriskAction::Migrate { .. }));
}

#[test]
fn test_should_derisk_emergency_when_critical() {
    let config = VaultConfig::default();
    let health = vec![HealthStatus {
        adapter_name: "aave_savings".into(),
        score: 0.1, oracle_fresh: false, liquidity_adequate: false,
        utilisation_rate: 0.99, details: "critical".into(),
    }];
    let action = should_derisk(&config, &health);
    assert!(matches!(action, DeriskAction::EmergencyWithdraw { .. }));
}
```

**Step 2: Implement**

```rust
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
        DeriskAction::EmergencyWithdraw { adapter: worst_adapter }
    } else {
        DeriskAction::Migrate {
            from: worst_adapter,
            to: RiskSpectrum::Sovereign,
        }
    }
}
```

**Step 3: Run tests, commit**

```bash
cargo test --test engine_test
git add src/domain/engine.rs tests/engine_test.rs
git commit -m "feat: implement derisking logic with emergency withdraw threshold"
```

---

## Phase 3: Adapter System

### Task 7: YieldAdapter Trait

**Files:**
- Create: `src/domain/adapters/mod.rs`
- Modify: `src/domain/mod.rs` — add `pub mod adapters;`

**Step 1: Define the trait**

```rust
// src/domain/adapters/mod.rs
pub mod sovereign_bond;
pub mod aave_savings;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::domain::engine::{HealthStatus, RiskSpectrum};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRequest {
    pub to: String,      // contract address
    pub data: String,    // hex-encoded calldata
    pub value: String,   // wei as string
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
```

**Step 2: Verify it compiles**

```bash
cargo build
```

**Step 3: Commit**

```bash
git add src/domain/adapters/
git commit -m "feat: define YieldAdapter trait and TxRequest type"
```

---

### Task 8: MockRWAVault Smart Contract

**Files:**
- Create: `contracts/src/mocks/MockRWAVault.sol`
- Create: `contracts/test/MockRWAVault.t.sol`

**Step 1: Write contract test**

```solidity
// contracts/test/MockRWAVault.t.sol
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/mocks/MockRWAVault.sol";

contract MockRWAVaultTest is Test {
    MockRWAVault vault;
    address user = address(0x1);

    function setUp() public {
        vault = new MockRWAVault("Mock Sovereign Bond", "mSOV", 450); // 4.5% APY in bps
        deal(address(vault.asset()), user, 100 ether);
    }

    function test_deposit() public {
        vm.startPrank(user);
        vault.asset().approve(address(vault), 10 ether);
        uint256 shares = vault.deposit(10 ether, user);
        assertGt(shares, 0);
        assertEq(vault.balanceOf(user), shares);
        vm.stopPrank();
    }

    function test_yield_accrual() public {
        vm.startPrank(user);
        vault.asset().approve(address(vault), 10 ether);
        vault.deposit(10 ether, user);
        vm.stopPrank();

        // Simulate 1 year
        vm.warp(block.timestamp + 365 days);
        vault.accrueYield();

        uint256 totalAssets = vault.totalAssets();
        // 4.5% of 10 ether = 0.45 ether
        assertGt(totalAssets, 10 ether);
        assertApproxEqRel(totalAssets, 10.45 ether, 0.01e18); // 1% tolerance
    }

    function test_withdraw() public {
        vm.startPrank(user);
        vault.asset().approve(address(vault), 10 ether);
        uint256 shares = vault.deposit(10 ether, user);
        uint256 assets = vault.redeem(shares, user, user);
        assertEq(assets, 10 ether);
        vm.stopPrank();
    }

    function test_configurable_apy() public view {
        assertEq(vault.apyBps(), 450);
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cd contracts && forge test --match-contract MockRWAVaultTest
```

**Step 3: Implement contract**

```solidity
// contracts/src/mocks/MockRWAVault.sol
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/// @title MockRWAVault - Simulates tokenised sovereign bond yields on testnet
/// @notice ERC-4626 vault that mints synthetic yield at a configurable APY
contract MockRWAVault is ERC4626 {
    uint256 public apyBps; // basis points, e.g. 450 = 4.5%
    uint256 public lastAccrual;
    ERC20 private _underlying;

    constructor(
        string memory name_,
        string memory symbol_,
        uint256 apyBps_
    ) ERC4626(new MockAsset("Mock USD", "mUSD")) ERC20(name_, symbol_) {
        apyBps = apyBps_;
        lastAccrual = block.timestamp;
        _underlying = MockAsset(asset());
    }

    /// @notice Simulate yield accrual based on time elapsed
    function accrueYield() public {
        uint256 elapsed = block.timestamp - lastAccrual;
        if (elapsed == 0) return;

        uint256 totalDeposited = _underlying.balanceOf(address(this));
        uint256 yield_ = totalDeposited * apyBps * elapsed / (10000 * 365 days);

        if (yield_ > 0) {
            MockAsset(asset()).mint(address(this), yield_);
        }
        lastAccrual = block.timestamp;
    }

    function totalAssets() public view override returns (uint256) {
        uint256 elapsed = block.timestamp - lastAccrual;
        uint256 base = _underlying.balanceOf(address(this));
        uint256 pendingYield = base * apyBps * elapsed / (10000 * 365 days);
        return base + pendingYield;
    }
}

/// @title MockAsset - Mintable ERC20 for testnet
contract MockAsset is ERC20 {
    constructor(string memory name_, string memory symbol_) ERC20(name_, symbol_) {}

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}
```

**Step 4: Run test, commit**

```bash
cd contracts && forge test --match-contract MockRWAVaultTest
cd /Users/fabio/Projects/impactvault
git add contracts/
git commit -m "feat: add MockRWAVault ERC-4626 contract with configurable APY"
```

---

### Task 9: ImpactVault Smart Contract

**Files:**
- Create: `contracts/src/ImpactVault.sol`
- Create: `contracts/test/ImpactVault.t.sol`

**Step 1: Write tests**

```solidity
// contracts/test/ImpactVault.t.sol
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/ImpactVault.sol";
import "../src/mocks/MockRWAVault.sol";

contract ImpactVaultTest is Test {
    ImpactVault vault;
    MockAsset asset;
    address admin = address(this);
    address whitelisted = address(0x1);
    address nonWhitelisted = address(0x2);

    function setUp() public {
        asset = new MockAsset("Mock USD", "mUSD");
        vault = new ImpactVault(IERC20(address(asset)), "ImpactVault Share", "ivUSD", admin);
        vault.setWhitelisted(whitelisted, true);
        asset.mint(whitelisted, 100 ether);
        asset.mint(nonWhitelisted, 100 ether);
    }

    function test_whitelisted_can_deposit() public {
        vm.startPrank(whitelisted);
        asset.approve(address(vault), 10 ether);
        uint256 shares = vault.deposit(10 ether, whitelisted);
        assertGt(shares, 0);
        vm.stopPrank();
    }

    function test_non_whitelisted_cannot_deposit() public {
        vm.startPrank(nonWhitelisted);
        asset.approve(address(vault), 10 ether);
        vm.expectRevert("not whitelisted");
        vault.deposit(10 ether, nonWhitelisted);
        vm.stopPrank();
    }

    function test_timelock_on_parameter_change() public {
        vault.proposeYieldSource(address(0xBEEF));
        // Should not be active yet
        assertEq(vault.activeYieldSource(), address(0));
        // Warp past timelock
        vm.warp(block.timestamp + vault.TIMELOCK_DURATION() + 1);
        vault.executeYieldSourceChange();
        assertEq(vault.activeYieldSource(), address(0xBEEF));
    }

    function test_emergency_derisk_only_reduces_risk() public {
        vault.proposeYieldSource(address(0xBEEF));
        vm.warp(block.timestamp + vault.TIMELOCK_DURATION() + 1);
        vault.executeYieldSourceChange();
        // Emergency derisk removes yield source (moves to safety)
        vault.emergencyDerisk();
        assertEq(vault.activeYieldSource(), address(0));
    }
}
```

**Step 2: Implement**

```solidity
// contracts/src/ImpactVault.sol
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/// @title ImpactVault - Risk-curated yield vault for social impact
/// @notice ERC-4626 vault with whitelisting, timelock, and emergency derisk
contract ImpactVault is ERC4626, Ownable {
    uint256 public constant TIMELOCK_DURATION = 2 days;

    mapping(address => bool) public whitelisted;
    address public activeYieldSource;

    // Timelock state
    address public pendingYieldSource;
    uint256 public yieldSourceChangeTimestamp;

    event WhitelistUpdated(address indexed account, bool status);
    event YieldSourceProposed(address indexed source, uint256 executeAfter);
    event YieldSourceChanged(address indexed source);
    event EmergencyDerisk(address indexed previousSource);

    constructor(
        IERC20 asset_,
        string memory name_,
        string memory symbol_,
        address admin
    ) ERC4626(asset_) ERC20(name_, symbol_) Ownable(admin) {}

    function setWhitelisted(address account, bool status) external onlyOwner {
        whitelisted[account] = status;
        emit WhitelistUpdated(account, status);
    }

    function proposeYieldSource(address source) external onlyOwner {
        pendingYieldSource = source;
        yieldSourceChangeTimestamp = block.timestamp + TIMELOCK_DURATION;
        emit YieldSourceProposed(source, yieldSourceChangeTimestamp);
    }

    function executeYieldSourceChange() external onlyOwner {
        require(pendingYieldSource != address(0), "no pending change");
        require(block.timestamp >= yieldSourceChangeTimestamp, "timelock active");
        activeYieldSource = pendingYieldSource;
        pendingYieldSource = address(0);
        yieldSourceChangeTimestamp = 0;
        emit YieldSourceChanged(activeYieldSource);
    }

    /// @notice Emergency derisk — can only remove yield source (move to safety)
    function emergencyDerisk() external onlyOwner {
        address prev = activeYieldSource;
        activeYieldSource = address(0);
        pendingYieldSource = address(0);
        yieldSourceChangeTimestamp = 0;
        emit EmergencyDerisk(prev);
    }

    function _deposit(
        address caller,
        address receiver,
        uint256 assets,
        uint256 shares
    ) internal override {
        require(whitelisted[caller], "not whitelisted");
        super._deposit(caller, receiver, assets, shares);
    }
}
```

**Step 3: Run tests, commit**

```bash
cd contracts && forge test --match-contract ImpactVaultTest
cd /Users/fabio/Projects/impactvault
git add contracts/
git commit -m "feat: add ImpactVault ERC-4626 with whitelisting, timelock, emergency derisk"
```

---

### Task 10: YieldSplitter Smart Contract

**Files:**
- Create: `contracts/src/YieldSplitter.sol`
- Create: `contracts/test/YieldSplitter.t.sol`

**Step 1: Write tests**

```solidity
// contracts/test/YieldSplitter.t.sol
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/YieldSplitter.sol";
import "../src/mocks/MockRWAVault.sol";

contract YieldSplitterTest is Test {
    YieldSplitter splitter;
    MockAsset token;
    address recipient1 = address(0x1);
    address recipient2 = address(0x2);

    function setUp() public {
        token = new MockAsset("Mock USD", "mUSD");
        address[] memory recipients = new address[](2);
        uint256[] memory bps = new uint256[](2);
        recipients[0] = recipient1;
        recipients[1] = recipient2;
        bps[0] = 6000; // 60%
        bps[1] = 4000; // 40%
        splitter = new YieldSplitter(IERC20(address(token)), recipients, bps, address(this));
    }

    function test_distribute_splits_correctly() public {
        token.mint(address(splitter), 1000 ether);
        splitter.distribute();
        assertEq(token.balanceOf(recipient1), 600 ether);
        assertEq(token.balanceOf(recipient2), 400 ether);
    }

    function test_distribute_emits_events() public {
        token.mint(address(splitter), 100 ether);
        vm.expectEmit(true, false, false, true);
        emit YieldSplitter.YieldDistributed(recipient1, 60 ether);
        vm.expectEmit(true, false, false, true);
        emit YieldSplitter.YieldDistributed(recipient2, 40 ether);
        splitter.distribute();
    }

    function test_total_bps_must_equal_10000() public {
        address[] memory r = new address[](1);
        uint256[] memory b = new uint256[](1);
        r[0] = recipient1;
        b[0] = 5000;
        vm.expectRevert("bps must total 10000");
        new YieldSplitter(IERC20(address(token)), r, b, address(this));
    }

    function test_update_recipients_with_timelock() public {
        address[] memory newR = new address[](1);
        uint256[] memory newB = new uint256[](1);
        newR[0] = address(0x3);
        newB[0] = 10000;

        splitter.proposeRecipientUpdate(newR, newB);
        vm.warp(block.timestamp + splitter.TIMELOCK_DURATION() + 1);
        splitter.executeRecipientUpdate();

        token.mint(address(splitter), 100 ether);
        splitter.distribute();
        assertEq(token.balanceOf(address(0x3)), 100 ether);
    }
}
```

**Step 2: Implement**

```solidity
// contracts/src/YieldSplitter.sol
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/// @title YieldSplitter - Distributes yield to impact programme recipients
/// @notice Splits incoming tokens according to basis point allocations
contract YieldSplitter is Ownable {
    uint256 public constant TIMELOCK_DURATION = 2 days;

    IERC20 public token;

    struct Recipient {
        address wallet;
        uint256 bps; // basis points out of 10000
    }

    Recipient[] public recipients;

    // Timelock for recipient updates
    Recipient[] private _pendingRecipients;
    uint256 public recipientUpdateTimestamp;

    event YieldDistributed(address indexed recipient, uint256 amount);
    event RecipientUpdateProposed(uint256 executeAfter);
    event RecipientsUpdated();

    constructor(
        IERC20 token_,
        address[] memory wallets,
        uint256[] memory bps,
        address admin
    ) Ownable(admin) {
        token = token_;
        _setRecipients(wallets, bps);
    }

    function distribute() external {
        uint256 balance = token.balanceOf(address(this));
        require(balance > 0, "nothing to distribute");

        for (uint256 i = 0; i < recipients.length; i++) {
            uint256 amount = balance * recipients[i].bps / 10000;
            token.transfer(recipients[i].wallet, amount);
            emit YieldDistributed(recipients[i].wallet, amount);
        }
    }

    function proposeRecipientUpdate(
        address[] memory wallets,
        uint256[] memory bps
    ) external onlyOwner {
        require(wallets.length == bps.length, "length mismatch");
        uint256 total;
        delete _pendingRecipients;
        for (uint256 i = 0; i < wallets.length; i++) {
            total += bps[i];
            _pendingRecipients.push(Recipient(wallets[i], bps[i]));
        }
        require(total == 10000, "bps must total 10000");
        recipientUpdateTimestamp = block.timestamp + TIMELOCK_DURATION;
        emit RecipientUpdateProposed(recipientUpdateTimestamp);
    }

    function executeRecipientUpdate() external onlyOwner {
        require(_pendingRecipients.length > 0, "no pending update");
        require(block.timestamp >= recipientUpdateTimestamp, "timelock active");
        delete recipients;
        for (uint256 i = 0; i < _pendingRecipients.length; i++) {
            recipients.push(_pendingRecipients[i]);
        }
        delete _pendingRecipients;
        recipientUpdateTimestamp = 0;
        emit RecipientsUpdated();
    }

    function recipientCount() external view returns (uint256) {
        return recipients.length;
    }

    function _setRecipients(address[] memory wallets, uint256[] memory bps) private {
        require(wallets.length == bps.length, "length mismatch");
        uint256 total;
        for (uint256 i = 0; i < wallets.length; i++) {
            total += bps[i];
            recipients.push(Recipient(wallets[i], bps[i]));
        }
        require(total == 10000, "bps must total 10000");
    }
}
```

**Step 3: Run tests, commit**

```bash
cd contracts && forge test --match-contract YieldSplitterTest
cd /Users/fabio/Projects/impactvault
git add contracts/
git commit -m "feat: add YieldSplitter with timelock recipient updates and on-chain events"
```

---

### Task 11: Sovereign Bond Adapter (Rust)

**Files:**
- Create: `src/domain/adapters/sovereign_bond.rs`
- Create: `tests/sovereign_bond_test.rs`

**Step 1: Write failing test**

```rust
// tests/sovereign_bond_test.rs
use impactvault::domain::adapters::YieldAdapter;
use impactvault::domain::adapters::sovereign_bond::SovereignBondAdapter;
use impactvault::domain::engine::RiskSpectrum;

#[test]
fn test_sovereign_bond_metadata() {
    let adapter = SovereignBondAdapter::new(
        "0x1234567890abcdef".into(),
        11155111, // Sepolia chain ID
        "https://rpc.sepolia.org".into(),
    );
    assert_eq!(adapter.name(), "sovereign_bond");
    assert_eq!(adapter.risk_position(), RiskSpectrum::Sovereign);
}

#[tokio::test]
async fn test_deposit_returns_valid_tx() {
    let adapter = SovereignBondAdapter::new(
        "0x1234567890abcdef".into(),
        11155111,
        "https://rpc.sepolia.org".into(),
    );
    let tx = adapter.deposit(1_000_000).await.unwrap();
    assert_eq!(tx.chain_id, 11155111);
    assert!(!tx.data.is_empty());
    assert!(tx.to.starts_with("0x"));
}
```

**Step 2: Implement**

```rust
// src/domain/adapters/sovereign_bond.rs
use async_trait::async_trait;
use crate::domain::engine::{HealthStatus, RiskSpectrum};
use super::{TxRequest, YieldAdapter};

pub struct SovereignBondAdapter {
    contract_address: String,
    chain_id: u64,
    rpc_url: String,
}

impl SovereignBondAdapter {
    pub fn new(contract_address: String, chain_id: u64, rpc_url: String) -> Self {
        Self { contract_address, chain_id, rpc_url }
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
        let calldata = format!(
            "0x6e553f65{:064x}{:064x}",
            amount,
            0u128 // receiver placeholder — caller fills real address
        );
        Ok(TxRequest {
            to: self.contract_address.clone(),
            data: calldata,
            value: "0".into(),
            chain_id: self.chain_id,
        })
    }

    async fn withdraw(&self, amount: u128) -> anyhow::Result<TxRequest> {
        // ERC-4626 withdraw(uint256,address,address) selector: 0xb460af94
        let calldata = format!(
            "0xb460af94{:064x}{:064x}{:064x}",
            amount, 0u128, 0u128
        );
        Ok(TxRequest {
            to: self.contract_address.clone(),
            data: calldata,
            value: "0".into(),
            chain_id: self.chain_id,
        })
    }

    async fn current_yield_apy(&self) -> anyhow::Result<f64> {
        // In production: read apyBps() from contract
        // For now: return expected sovereign bond range
        Ok(4.5)
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        // In production: check contract state, oracle freshness, etc. via RPC
        Ok(HealthStatus {
            adapter_name: self.name().into(),
            score: 0.95,
            oracle_fresh: true,
            liquidity_adequate: true,
            utilisation_rate: 0.0,
            details: "mock: sovereign bonds always healthy on testnet".into(),
        })
    }

    async fn tvl(&self) -> anyhow::Result<u128> {
        // In production: call totalAssets() on contract
        Ok(0)
    }
}
```

**Step 3: Run tests, commit**

```bash
cargo test --test sovereign_bond_test
git add src/domain/adapters/sovereign_bond.rs tests/sovereign_bond_test.rs
git commit -m "feat: implement SovereignBondAdapter wrapping MockRWAVault"
```

---

### Task 12: Aave Savings Adapter (Rust)

**Files:**
- Create: `src/domain/adapters/aave_savings.rs`
- Create: `tests/aave_savings_test.rs`

**Step 1: Write failing test**

```rust
// tests/aave_savings_test.rs
use impactvault::domain::adapters::YieldAdapter;
use impactvault::domain::adapters::aave_savings::AaveSavingsAdapter;
use impactvault::domain::engine::RiskSpectrum;

#[test]
fn test_aave_metadata() {
    let adapter = AaveSavingsAdapter::new(
        "0xaave_pool_address".into(),
        "0xusdc_address".into(),
        11155111,
        "https://rpc.sepolia.org".into(),
    );
    assert_eq!(adapter.name(), "aave_savings");
    assert_eq!(adapter.risk_position(), RiskSpectrum::StablecoinSavings);
}

#[tokio::test]
async fn test_aave_deposit_tx() {
    let adapter = AaveSavingsAdapter::new(
        "0xaave_pool_address".into(),
        "0xusdc_address".into(),
        11155111,
        "https://rpc.sepolia.org".into(),
    );
    let tx = adapter.deposit(1_000_000).await.unwrap();
    assert_eq!(tx.chain_id, 11155111);
    assert!(tx.data.contains("617ba037")); // Aave supply() selector
}
```

**Step 2: Implement**

```rust
// src/domain/adapters/aave_savings.rs
use async_trait::async_trait;
use crate::domain::engine::{HealthStatus, RiskSpectrum};
use super::{TxRequest, YieldAdapter};

pub struct AaveSavingsAdapter {
    pool_address: String,
    asset_address: String,
    chain_id: u64,
    rpc_url: String,
}

impl AaveSavingsAdapter {
    pub fn new(
        pool_address: String,
        asset_address: String,
        chain_id: u64,
        rpc_url: String,
    ) -> Self {
        Self { pool_address, asset_address, chain_id, rpc_url }
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
        let calldata = format!(
            "0x617ba037{:064x}{:064x}{:064x}{:064x}",
            0u128, // asset address placeholder (needs proper ABI encoding)
            amount,
            0u128, // onBehalfOf placeholder
            0u128, // referralCode
        );
        Ok(TxRequest {
            to: self.pool_address.clone(),
            data: calldata,
            value: "0".into(),
            chain_id: self.chain_id,
        })
    }

    async fn withdraw(&self, amount: u128) -> anyhow::Result<TxRequest> {
        // Aave V3 Pool.withdraw(address asset, uint256 amount, address to)
        // selector: 0x69328dec
        let calldata = format!(
            "0x69328dec{:064x}{:064x}{:064x}",
            0u128, amount, 0u128,
        );
        Ok(TxRequest {
            to: self.pool_address.clone(),
            data: calldata,
            value: "0".into(),
            chain_id: self.chain_id,
        })
    }

    async fn current_yield_apy(&self) -> anyhow::Result<f64> {
        // In production: read currentLiquidityRate from Aave pool via RPC
        Ok(3.2) // typical stablecoin supply rate
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        // In production: check pool utilisation, oracle freshness via RPC
        Ok(HealthStatus {
            adapter_name: self.name().into(),
            score: 0.9,
            oracle_fresh: true,
            liquidity_adequate: true,
            utilisation_rate: 0.45,
            details: "mock: Aave V3 Sepolia".into(),
        })
    }

    async fn tvl(&self) -> anyhow::Result<u128> {
        Ok(0)
    }
}
```

**Step 3: Run tests, commit**

```bash
cargo test --test aave_savings_test
git add src/domain/adapters/aave_savings.rs tests/aave_savings_test.rs
git commit -m "feat: implement AaveSavingsAdapter wrapping Aave V3 pool"
```

---

## Phase 4: Sentinel Monitoring

### Task 13: Sentinel Monitoring Loop

**Files:**
- Create: `src/domain/sentinel.rs`
- Modify: `src/domain/mod.rs` — add `pub mod sentinel;`
- Create: `tests/sentinel_test.rs`

**Step 1: Write failing test**

```rust
// tests/sentinel_test.rs
use impactvault::domain::sentinel::SentinelConfig;

#[test]
fn test_sentinel_config_defaults() {
    let config = SentinelConfig::default();
    assert_eq!(config.poll_interval_secs, 60);
    assert!(config.auto_derisk_enabled);
}
```

**Step 2: Implement**

```rust
// src/domain/sentinel.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};
use crate::domain::adapters::YieldAdapter;
use crate::domain::engine::{self, VaultConfig, Portfolio, DeriskAction, HealthStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelConfig {
    pub poll_interval_secs: u64,
    pub auto_derisk_enabled: bool,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 60,
            auto_derisk_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SentinelStatus {
    pub running: bool,
    pub last_check: Option<String>,
    pub checks_completed: u64,
    pub last_health: Vec<HealthStatus>,
    pub last_action: Option<DeriskAction>,
}

pub struct Sentinel {
    config: SentinelConfig,
    vault_config: Arc<RwLock<VaultConfig>>,
    adapters: Vec<Box<dyn YieldAdapter>>,
    status: Arc<RwLock<SentinelStatus>>,
}

impl Sentinel {
    pub fn new(
        config: SentinelConfig,
        vault_config: Arc<RwLock<VaultConfig>>,
        adapters: Vec<Box<dyn YieldAdapter>>,
    ) -> Self {
        Self {
            config,
            vault_config,
            adapters,
            status: Arc::new(RwLock::new(SentinelStatus {
                running: false,
                last_check: None,
                checks_completed: 0,
                last_health: vec![],
                last_action: None,
            })),
        }
    }

    pub fn status_handle(&self) -> Arc<RwLock<SentinelStatus>> {
        self.status.clone()
    }

    /// Run a single health check cycle
    pub async fn check_once(&self) -> Vec<HealthStatus> {
        let mut results = Vec::new();
        for adapter in &self.adapters {
            match adapter.health_check().await {
                Ok(health) => {
                    info!(adapter = adapter.name(), score = health.score, "health check ok");
                    results.push(health);
                }
                Err(e) => {
                    error!(adapter = adapter.name(), error = %e, "health check failed");
                    results.push(HealthStatus {
                        adapter_name: adapter.name().into(),
                        score: 0.0,
                        oracle_fresh: false,
                        liquidity_adequate: false,
                        utilisation_rate: 1.0,
                        details: format!("health check error: {}", e),
                    });
                }
            }
        }

        // Evaluate risk
        let vault_config = self.vault_config.read().await;
        let action = engine::should_derisk(&vault_config, &results);

        match &action {
            DeriskAction::Hold => info!("sentinel: all clear"),
            DeriskAction::Migrate { from, to } => {
                warn!(from = from, to = ?to, "sentinel: migration recommended");
            }
            DeriskAction::EmergencyWithdraw { adapter } => {
                error!(adapter = adapter, "sentinel: EMERGENCY WITHDRAW recommended");
            }
        }

        // Update status
        let mut status = self.status.write().await;
        status.last_check = Some(chrono::Utc::now().to_rfc3339());
        status.checks_completed += 1;
        status.last_health = results.clone();
        status.last_action = Some(action);

        results
    }

    /// Run the monitoring loop
    pub async fn run(self: Arc<Self>, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        {
            let mut status = self.status.write().await;
            status.running = true;
        }
        info!(interval = self.config.poll_interval_secs, "sentinel started");

        let interval = tokio::time::Duration::from_secs(self.config.poll_interval_secs);
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    self.check_once().await;
                }
                _ = shutdown.changed() => {
                    info!("sentinel shutting down");
                    break;
                }
            }
        }

        let mut status = self.status.write().await;
        status.running = false;
    }
}
```

**Step 3: Run tests, commit**

```bash
cargo test --test sentinel_test
git add src/domain/sentinel.rs src/domain/mod.rs tests/sentinel_test.rs
git commit -m "feat: implement Sentinel monitoring loop with health checks and auto-derisk"
```

---

## Phase 5: MCP Tools & REST API

### Task 14: Update MCP Server with Vault Tools

**Files:**
- Modify: `src/gateway/server.rs` — add vault/adapter/sentinel/risk tools
- Modify: `src/gateway/router.rs` — add new prefixes

**Step 1: Add new tool definitions to server.rs**

Add MCP tools:
- `vault_status` — returns current portfolio and health
- `vault_risk` — runs risk evaluation
- `adapter_list` — lists all adapters with health
- `adapter_health` — single adapter health check
- `sentinel_status` — monitoring loop status
- `risk_evaluate` — on-demand risk assessment

Each tool follows the same pattern as existing enforcer/lineage tools: input struct → handler function → JSON response.

**Step 2: Update router.rs**

Add prefix routes: `vault_*`, `adapter_*`, `sentinel_*`, `risk_*`.

**Step 3: Verify compile and commit**

```bash
cargo build
git add src/gateway/
git commit -m "feat: add MCP tools for vault, adapter, sentinel, and risk operations"
```

---

### Task 15: REST API Endpoints

**Files:**
- Create: `src/gateway/api.rs`
- Modify: `src/gateway/mod.rs` — add `pub mod api;`
- Modify: `src/main.rs` — spawn API server alongside MCP

**Step 1: Implement Axum routes**

```rust
// src/gateway/api.rs
use axum::{
    Router,
    routing::get,
    extract::State,
    Json,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::domain::sentinel::SentinelStatus;

#[derive(Clone)]
pub struct ApiState {
    pub sentinel_status: Arc<RwLock<SentinelStatus>>,
}

pub fn api_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/sentinel/status", get(sentinel_status))
        .route("/adapters", get(list_adapters))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn sentinel_status(
    State(state): State<ApiState>,
) -> Json<SentinelStatus> {
    let status = state.sentinel_status.read().await;
    Json(status.clone())
}

async fn list_adapters() -> Json<serde_json::Value> {
    // Placeholder — will be wired to actual adapter registry
    Json(serde_json::json!({
        "adapters": ["sovereign_bond", "aave_savings"]
    }))
}
```

**Step 2: Wire into main.rs**

In main.rs `serve` command, spawn the API server on a configurable port (default 3000) alongside the MCP stdio server and sentinel loop.

**Step 3: Test manually, commit**

```bash
cargo build
git add src/gateway/api.rs src/gateway/mod.rs src/main.rs
git commit -m "feat: add REST API with health, sentinel status, and adapter endpoints"
```

---

## Phase 6: Enforcer Rules & Lineage Integration

### Task 16: Risk-Specific Enforcer Rules

**Files:**
- Modify: `config.toml.example` — add risk-specific rules
- Modify: `src/orchestration/enforcer.rs` — add new built-in risk rules

**Step 1: Add rules to enforcer**

Add built-in rules:
- `derisk_on_health_breach` — MissingInWindow: if `sentinel_check` fires and `health_ok` missing in window of 3
- `oracle_staleness` — MissingInWindow: if `adapter_query` without `oracle_fresh` in window of 1
- `concentration_limit` — RepeatWithout: if `deposit` category repeated 3x without `rebalance`

**Step 2: Update config.toml.example**

```toml
[general]
data_dir = "~/.impactvault"

[sentinel]
poll_interval_secs = 60
auto_derisk_enabled = true

[api]
port = 3000

[vault]
approved_sources = ["Sovereign", "StablecoinSavings"]
max_exposure_per_source = 100
concentration_limit = 80
derisking_health_threshold = 0.5

[[adapters]]
name = "sovereign_bond"
type = "sovereign_bond"
contract_address = "0x..."
chain_id = 11155111
rpc_url = "https://rpc.sepolia.org"

[[adapters]]
name = "aave_savings"
type = "aave_savings"
pool_address = "0x..."
asset_address = "0x..."
chain_id = 11155111
rpc_url = "https://rpc.sepolia.org"

[[enforcer.rules]]
name = "derisk_on_health_breach"
description = "Trigger derisk if health checks fail repeatedly"
action = "block"
enabled = true

[enforcer.rules.condition]
type = "MissingInWindow"
trigger = "sentinel_check"
required = "health_ok"
window = 3
```

**Step 3: Commit**

```bash
git add src/orchestration/enforcer.rs config.toml.example
git commit -m "feat: add risk-specific enforcer rules and vault configuration"
```

---

## Phase 7: Schema, Config, and Wiring

### Task 17: SQLite Schema for Vault State

**Files:**
- Modify: `src/store/state.rs` — add vault tables to migration

**Step 1: Add DDL**

Add tables: `vaults`, `adapter_health_log`, `disbursements`, `risk_events`.

```sql
CREATE TABLE IF NOT EXISTS vaults (
    id TEXT PRIMARY KEY,
    config_json TEXT NOT NULL,
    portfolio_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS adapter_health_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    adapter_name TEXT NOT NULL,
    score REAL NOT NULL,
    oracle_fresh INTEGER NOT NULL,
    liquidity_adequate INTEGER NOT NULL,
    utilisation_rate REAL NOT NULL,
    details TEXT,
    checked_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS disbursements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tx_hash TEXT,
    recipient TEXT NOT NULL,
    amount TEXT NOT NULL,
    token TEXT NOT NULL,
    block_number INTEGER,
    recorded_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS risk_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    adapter_name TEXT,
    action_taken TEXT NOT NULL,
    details_json TEXT,
    occurred_at TEXT NOT NULL
);
```

**Step 2: Commit**

```bash
git add src/store/state.rs
git commit -m "feat: add SQLite schema for vaults, health log, disbursements, risk events"
```

---

### Task 18: Config Integration

**Files:**
- Modify: `src/config.rs` — add vault, adapter, sentinel, API config sections

**Step 1: Update config structs**

Add `VaultTomlConfig`, `AdapterTomlConfig`, `SentinelTomlConfig`, `ApiConfig` to the config hierarchy. Wire deserialization from `config.toml`.

**Step 2: Wire config into main.rs**

In the `serve` command, read adapter configs → instantiate adapters → create sentinel → spawn all services.

**Step 3: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat: wire vault, adapter, sentinel config from TOML to runtime"
```

---

## Phase 8: Adapter Template & Documentation

### Task 19: Adapter Development Template

**Files:**
- Create: `adapter-template/README.md`
- Create: `adapter-template/src/lib.rs`
- Create: `adapter-template/Cargo.toml`

**Step 1: Create template**

A minimal Rust project that implements `YieldAdapter` with TODOs for each method. Includes example test, Cargo.toml pointing to impactvault as a dependency, and a README explaining how to build, test, and register a new adapter.

**Step 2: Commit**

```bash
git add adapter-template/
git commit -m "feat: add adapter development template with trait scaffold and guide"
```

---

### Task 20: Project Documentation

**Files:**
- Create: `README.md`
- Create: `docs/architecture.md`
- Create: `docs/adapters.md`
- Update: `.gitignore`
- Create: `LICENSE`

**Step 1: Write README**

Cover: what ImpactVault is, how to build, how to configure, how to run (MCP + API + Sentinel), how to write adapters.

**Step 2: Write architecture doc**

Explain the module structure, data flow (deposit → adapter → yield → splitter → recipients), and how enforcer/lineage/patterns work for risk management.

**Step 3: Add MIT LICENSE, .gitignore**

**Step 4: Commit**

```bash
git add README.md docs/ LICENSE .gitignore
git commit -m "docs: add README, architecture guide, adapter guide, and MIT license"
```

---

## Phase 9: Integration Test

### Task 21: End-to-End Integration Test

**Files:**
- Create: `tests/integration_test.rs`

**Step 1: Write integration test**

Test the full pipeline: create vault config → instantiate both adapters → run sentinel check → evaluate risk → verify allocation → verify derisking logic.

```rust
#[tokio::test]
async fn test_full_pipeline() {
    // 1. Create config
    // 2. Create adapters
    // 3. Create portfolio with allocations
    // 4. Run health checks via adapters
    // 5. Evaluate risk
    // 6. Verify assessment is Hold (healthy state)
    // 7. Simulate degraded health
    // 8. Verify derisking triggers
}
```

**Step 2: Run, commit**

```bash
cargo test --test integration_test
git add tests/integration_test.rs
git commit -m "test: add end-to-end integration test for full vault pipeline"
```

---

## Phase 10: Contract Integration Tests

### Task 22: Full Solidity Integration Test

**Files:**
- Create: `contracts/test/Integration.t.sol`

**Step 1: Write test**

Test: deploy MockRWAVault → deploy ImpactVault → deploy YieldSplitter → whitelist user → deposit → accrue yield → distribute to recipients → verify balances.

**Step 2: Run, commit**

```bash
cd contracts && forge test --match-contract IntegrationTest -vvv
cd /Users/fabio/Projects/impactvault
git add contracts/test/Integration.t.sol
git commit -m "test: add Solidity integration test for full deposit→yield→distribute pipeline"
```

---

## Summary

| Phase | Tasks | What it delivers |
|-------|-------|-----------------|
| 1: Setup | 1-2 | Fork skeleton + Foundry project |
| 2: Engine | 3-6 | Risk types, evaluation, allocation, derisking |
| 3: Adapters | 7-12 | Trait + sovereign bond + Aave + contracts |
| 4: Sentinel | 13 | Monitoring loop with auto-derisk |
| 5: API | 14-15 | MCP tools + REST endpoints |
| 6: Rules | 16 | Risk-specific enforcer rules |
| 7: Wiring | 17-18 | SQLite schema + config integration |
| 8: Template | 19 | Community adapter scaffold |
| 9-10: Tests | 20-22 | Docs + integration tests |
