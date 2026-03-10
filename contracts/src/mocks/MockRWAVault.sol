// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ERC20} from "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";
import {ERC4626} from "openzeppelin-contracts/contracts/token/ERC20/extensions/ERC4626.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";

/// @title MockAsset — simple mintable ERC20 for testing
contract MockAsset is ERC20 {
    constructor(string memory name_, string memory symbol_) ERC20(name_, symbol_) {}

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

/// @title MockRWAVault — ERC-4626 vault simulating tokenised sovereign bond yields
/// @notice Testnet-only. Mints synthetic yield based on a configurable APY.
contract MockRWAVault is ERC4626 {
    uint256 public apyBps; // e.g. 450 = 4.50%
    uint256 public lastAccrualTimestamp;
    MockAsset public underlyingAsset;

    constructor(
        string memory name_,
        string memory symbol_,
        uint256 apyBps_
    ) ERC20(name_, symbol_) ERC4626(IERC20(address(new MockAsset("Mock Bond", "mBOND")))) {
        apyBps = apyBps_;
        underlyingAsset = MockAsset(asset());
        lastAccrualTimestamp = block.timestamp;
    }

    /// @notice Mints synthetic yield to the vault based on APY and elapsed time
    function accrueYield() external {
        uint256 elapsed = block.timestamp - lastAccrualTimestamp;
        if (elapsed == 0) return;

        uint256 currentAssets = IERC20(asset()).balanceOf(address(this));
        // yield = currentAssets * apyBps * elapsed / (10000 * 365 days)
        uint256 yield_ = (currentAssets * apyBps * elapsed) / (10_000 * 365 days);

        if (yield_ > 0) {
            underlyingAsset.mint(address(this), yield_);
        }

        lastAccrualTimestamp = block.timestamp;
    }

    /// @notice Returns total assets including pending (un-accrued) yield
    function totalAssets() public view override returns (uint256) {
        uint256 currentBalance = IERC20(asset()).balanceOf(address(this));
        uint256 elapsed = block.timestamp - lastAccrualTimestamp;
        if (elapsed == 0) return currentBalance;

        uint256 pendingYield = (currentBalance * apyBps * elapsed) / (10_000 * 365 days);
        return currentBalance + pendingYield;
    }
}
