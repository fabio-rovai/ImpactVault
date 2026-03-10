// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ERC20} from "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";
import {ERC4626} from "openzeppelin-contracts/contracts/token/ERC20/extensions/ERC4626.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import {Ownable} from "openzeppelin-contracts/contracts/access/Ownable.sol";

/// @title ImpactVault — ERC-4626 vault with whitelisting, timelock, and emergency derisk
contract ImpactVault is ERC4626, Ownable {
    // ── Whitelisting ──────────────────────────────────────────────────
    mapping(address => bool) public whitelisted;

    // ── Yield source management ───────────────────────────────────────
    address public activeYieldSource;
    address public pendingYieldSource;
    uint256 public yieldSourceChangeTimestamp;

    uint256 public constant TIMELOCK_DURATION = 2 days;

    // ── Events ────────────────────────────────────────────────────────
    event WhitelistUpdated(address indexed account, bool status);
    event YieldSourceProposed(address indexed newSource, uint256 executeAfter);
    event YieldSourceChanged(address indexed newSource);
    event EmergencyDerisk();

    // ── Errors ────────────────────────────────────────────────────────
    error NotWhitelisted(address account);
    error NoYieldSourceProposed();
    error TimelockNotElapsed(uint256 earliest, uint256 current);

    constructor(
        IERC20 asset_,
        string memory name_,
        string memory symbol_,
        address admin
    ) ERC20(name_, symbol_) ERC4626(asset_) Ownable(admin) {}

    // ── Whitelist management ──────────────────────────────────────────
    function setWhitelisted(address account, bool status) external onlyOwner {
        whitelisted[account] = status;
        emit WhitelistUpdated(account, status);
    }

    // ── Deposit guard ─────────────────────────────────────────────────
    function _deposit(
        address caller,
        address receiver,
        uint256 assets,
        uint256 shares
    ) internal override {
        if (!whitelisted[caller]) {
            revert NotWhitelisted(caller);
        }
        super._deposit(caller, receiver, assets, shares);
    }

    // ── Yield source timelock ─────────────────────────────────────────
    function proposeYieldSource(address newSource) external onlyOwner {
        pendingYieldSource = newSource;
        yieldSourceChangeTimestamp = block.timestamp + TIMELOCK_DURATION;
        emit YieldSourceProposed(newSource, yieldSourceChangeTimestamp);
    }

    function executeYieldSourceChange() external onlyOwner {
        if (pendingYieldSource == address(0)) {
            revert NoYieldSourceProposed();
        }
        if (block.timestamp < yieldSourceChangeTimestamp) {
            revert TimelockNotElapsed(yieldSourceChangeTimestamp, block.timestamp);
        }
        activeYieldSource = pendingYieldSource;
        pendingYieldSource = address(0);
        yieldSourceChangeTimestamp = 0;
        emit YieldSourceChanged(activeYieldSource);
    }

    // ── Emergency derisk ──────────────────────────────────────────────
    function emergencyDerisk() external onlyOwner {
        activeYieldSource = address(0);
        pendingYieldSource = address(0);
        yieldSourceChangeTimestamp = 0;
        emit EmergencyDerisk();
    }
}
