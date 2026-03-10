// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";
import {Ownable} from "openzeppelin-contracts/contracts/access/Ownable.sol";

/// @title YieldSplitter — distributes yield tokens to impact programme recipients
contract YieldSplitter is Ownable {
    using SafeERC20 for IERC20;

    struct Recipient {
        address wallet;
        uint256 bps;
    }

    IERC20 public immutable token;
    Recipient[] public recipients;

    // ── Timelock for recipient updates ────────────────────────────────
    uint256 public constant TIMELOCK_DURATION = 2 days;
    address[] private _pendingWallets;
    uint256[] private _pendingBps;
    uint256 public updateTimestamp;
    bool public updatePending;

    // ── Events ────────────────────────────────────────────────────────
    event YieldDistributed(address indexed recipient, uint256 amount);
    event RecipientUpdateProposed(uint256 executeAfter);
    event RecipientsUpdated();

    // ── Errors ────────────────────────────────────────────────────────
    error TotalBpsMustEqual10000(uint256 actual);
    error ArrayLengthMismatch();
    error NoUpdatePending();
    error TimelockNotElapsed(uint256 earliest, uint256 current);

    constructor(
        IERC20 token_,
        address[] memory wallets,
        uint256[] memory bps,
        address admin
    ) Ownable(admin) {
        token = token_;
        _setRecipients(wallets, bps);
    }

    /// @notice Distributes the contract's entire token balance to recipients
    function distribute() external {
        uint256 balance = token.balanceOf(address(this));
        uint256 len = recipients.length;

        for (uint256 i = 0; i < len; i++) {
            uint256 amount = (balance * recipients[i].bps) / 10_000;
            if (amount > 0) {
                token.safeTransfer(recipients[i].wallet, amount);
                emit YieldDistributed(recipients[i].wallet, amount);
            }
        }
    }

    /// @notice Proposes a new set of recipients (timelocked)
    function proposeRecipientUpdate(
        address[] calldata wallets,
        uint256[] calldata bps
    ) external onlyOwner {
        if (wallets.length != bps.length) revert ArrayLengthMismatch();

        // Validate total bps upfront
        uint256 total;
        for (uint256 i = 0; i < bps.length; i++) {
            total += bps[i];
        }
        if (total != 10_000) revert TotalBpsMustEqual10000(total);

        delete _pendingWallets;
        delete _pendingBps;
        for (uint256 i = 0; i < wallets.length; i++) {
            _pendingWallets.push(wallets[i]);
            _pendingBps.push(bps[i]);
        }

        updateTimestamp = block.timestamp + TIMELOCK_DURATION;
        updatePending = true;

        emit RecipientUpdateProposed(updateTimestamp);
    }

    /// @notice Executes a previously proposed recipient update after timelock
    function executeRecipientUpdate() external onlyOwner {
        if (!updatePending) revert NoUpdatePending();
        if (block.timestamp < updateTimestamp) {
            revert TimelockNotElapsed(updateTimestamp, block.timestamp);
        }

        _setRecipients(_pendingWallets, _pendingBps);

        delete _pendingWallets;
        delete _pendingBps;
        updatePending = false;
        updateTimestamp = 0;

        emit RecipientsUpdated();
    }

    /// @notice Returns the number of current recipients
    function recipientCount() external view returns (uint256) {
        return recipients.length;
    }

    // ── Internal ──────────────────────────────────────────────────────
    function _setRecipients(address[] memory wallets, uint256[] memory bps) internal {
        if (wallets.length != bps.length) revert ArrayLengthMismatch();

        uint256 total;
        for (uint256 i = 0; i < bps.length; i++) {
            total += bps[i];
        }
        if (total != 10_000) revert TotalBpsMustEqual10000(total);

        delete recipients;
        for (uint256 i = 0; i < wallets.length; i++) {
            recipients.push(Recipient({wallet: wallets[i], bps: bps[i]}));
        }
    }
}
