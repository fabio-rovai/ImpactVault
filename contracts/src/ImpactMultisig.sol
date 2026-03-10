// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title ImpactMultisig — N-of-M signer governance for vault parameter changes
contract ImpactMultisig {
    uint256 public constant TIMELOCK_DURATION = 2 days;

    // ── State ────────────────────────────────────────────────────────────
    address[] public signers;
    mapping(address => bool) public isSigner;
    uint256 public threshold;

    struct Proposal {
        bytes callData;
        uint256 proposedAt;
        uint256 approvals;
        bool executed;
        mapping(address => bool) hasApproved;
    }

    Proposal[] private _proposals;

    // ── Events ───────────────────────────────────────────────────────────
    event ActionProposed(uint256 indexed proposalId, address indexed proposer, bytes callData);
    event ActionApproved(uint256 indexed proposalId, address indexed approver);
    event ActionExecuted(uint256 indexed proposalId);
    event EmergencyDeriskTriggered(address indexed triggeredBy);

    // ── Errors ───────────────────────────────────────────────────────────
    error NotASigner(address account);
    error DuplicateSigner(address account);
    error ZeroAddress();
    error InvalidThreshold(uint256 threshold, uint256 signerCount);
    error ProposalNotFound(uint256 proposalId);
    error AlreadyApproved(uint256 proposalId, address signer);
    error ThresholdNotMet(uint256 approvals, uint256 threshold);
    error TimelockNotElapsed(uint256 earliest, uint256 current);
    error AlreadyExecuted(uint256 proposalId);

    // ── Modifiers ────────────────────────────────────────────────────────
    modifier onlySigner() {
        if (!isSigner[msg.sender]) revert NotASigner(msg.sender);
        _;
    }

    // ── Constructor ──────────────────────────────────────────────────────
    constructor(address[] memory signers_, uint256 threshold_) {
        if (threshold_ == 0 || threshold_ > signers_.length) {
            revert InvalidThreshold(threshold_, signers_.length);
        }

        for (uint256 i = 0; i < signers_.length; i++) {
            address s = signers_[i];
            if (s == address(0)) revert ZeroAddress();
            if (isSigner[s]) revert DuplicateSigner(s);

            isSigner[s] = true;
            signers.push(s);
        }

        threshold = threshold_;
    }

    // ── Propose ──────────────────────────────────────────────────────────
    /// @notice Any signer can propose an action. The proposer auto-approves.
    function proposeAction(bytes calldata callData) external onlySigner returns (uint256 proposalId) {
        proposalId = _proposals.length;
        _proposals.push();

        Proposal storage p = _proposals[proposalId];
        p.callData = callData;
        p.proposedAt = block.timestamp;
        p.approvals = 1;
        p.hasApproved[msg.sender] = true;

        emit ActionProposed(proposalId, msg.sender, callData);
        emit ActionApproved(proposalId, msg.sender);
    }

    // ── Approve ──────────────────────────────────────────────────────────
    /// @notice Any signer can approve a proposal (cannot approve twice)
    function approveAction(uint256 proposalId) external onlySigner {
        if (proposalId >= _proposals.length) revert ProposalNotFound(proposalId);

        Proposal storage p = _proposals[proposalId];
        if (p.hasApproved[msg.sender]) revert AlreadyApproved(proposalId, msg.sender);

        p.hasApproved[msg.sender] = true;
        p.approvals += 1;

        emit ActionApproved(proposalId, msg.sender);
    }

    // ── Execute ──────────────────────────────────────────────────────────
    /// @notice Execute a proposal after threshold is met and timelock has elapsed
    function executeAction(uint256 proposalId) external onlySigner {
        if (proposalId >= _proposals.length) revert ProposalNotFound(proposalId);

        Proposal storage p = _proposals[proposalId];
        if (p.executed) revert AlreadyExecuted(proposalId);
        if (p.approvals < threshold) revert ThresholdNotMet(p.approvals, threshold);

        uint256 earliest = p.proposedAt + TIMELOCK_DURATION;
        if (block.timestamp < earliest) revert TimelockNotElapsed(earliest, block.timestamp);

        p.executed = true;

        emit ActionExecuted(proposalId);
    }

    // ── Emergency ────────────────────────────────────────────────────────
    /// @notice Any single signer can trigger emergency derisk (no threshold needed)
    function emergencyDerisk() external onlySigner {
        emit EmergencyDeriskTriggered(msg.sender);
    }

    // ── View functions ───────────────────────────────────────────────────
    function signerCount() external view returns (uint256) {
        return signers.length;
    }

    function approvalCount(uint256 proposalId) external view returns (uint256) {
        if (proposalId >= _proposals.length) revert ProposalNotFound(proposalId);
        return _proposals[proposalId].approvals;
    }

    function isExecuted(uint256 proposalId) external view returns (bool) {
        if (proposalId >= _proposals.length) revert ProposalNotFound(proposalId);
        return _proposals[proposalId].executed;
    }
}
