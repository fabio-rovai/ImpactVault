// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {ImpactMultisig} from "../src/ImpactMultisig.sol";

contract ImpactMultisigTest is Test {
    ImpactMultisig multisig;

    address signer1 = address(0x1);
    address signer2 = address(0x2);
    address signer3 = address(0x3);
    address nonSigner = address(0x4);

    function setUp() public {
        address[] memory signers_ = new address[](3);
        signers_[0] = signer1;
        signers_[1] = signer2;
        signers_[2] = signer3;

        multisig = new ImpactMultisig(signers_, 2);
    }

    function test_initial_state() public view {
        assertEq(multisig.threshold(), 2, "Threshold should be 2");
        assertEq(multisig.signerCount(), 3, "Should have 3 signers");
        assertTrue(multisig.isSigner(signer1), "Signer1 should be a signer");
        assertTrue(multisig.isSigner(signer2), "Signer2 should be a signer");
        assertTrue(multisig.isSigner(signer3), "Signer3 should be a signer");
        assertFalse(multisig.isSigner(nonSigner), "NonSigner should not be a signer");
    }

    function test_propose_action() public {
        bytes memory callData = abi.encodeWithSignature("emergencyDerisk()");

        vm.prank(signer1);
        uint256 proposalId = multisig.proposeAction(callData);

        assertEq(proposalId, 0, "First proposal should have id 0");
        assertEq(multisig.approvalCount(proposalId), 1, "Proposer auto-approves");
    }

    function test_non_signer_cannot_propose() public {
        bytes memory callData = abi.encodeWithSignature("emergencyDerisk()");

        vm.prank(nonSigner);
        vm.expectRevert(abi.encodeWithSelector(ImpactMultisig.NotASigner.selector, nonSigner));
        multisig.proposeAction(callData);
    }

    function test_approve_action() public {
        bytes memory callData = abi.encodeWithSignature("emergencyDerisk()");

        vm.prank(signer1);
        uint256 proposalId = multisig.proposeAction(callData);

        vm.prank(signer2);
        multisig.approveAction(proposalId);

        assertEq(multisig.approvalCount(proposalId), 2, "Should have 2 approvals");
    }

    function test_cannot_approve_twice() public {
        bytes memory callData = abi.encodeWithSignature("emergencyDerisk()");

        vm.prank(signer1);
        uint256 proposalId = multisig.proposeAction(callData);

        // signer1 already auto-approved, try again
        vm.prank(signer1);
        vm.expectRevert(abi.encodeWithSelector(ImpactMultisig.AlreadyApproved.selector, proposalId, signer1));
        multisig.approveAction(proposalId);
    }

    function test_execute_after_threshold_and_timelock() public {
        bytes memory callData = abi.encodeWithSignature("emergencyDerisk()");

        vm.prank(signer1);
        uint256 proposalId = multisig.proposeAction(callData);

        vm.prank(signer2);
        multisig.approveAction(proposalId);

        // Warp past timelock
        vm.warp(block.timestamp + 2 days + 1);

        vm.prank(signer1);
        multisig.executeAction(proposalId);

        assertTrue(multisig.isExecuted(proposalId), "Proposal should be executed");
    }

    function test_cannot_execute_before_timelock() public {
        bytes memory callData = abi.encodeWithSignature("emergencyDerisk()");

        vm.prank(signer1);
        uint256 proposalId = multisig.proposeAction(callData);

        vm.prank(signer2);
        multisig.approveAction(proposalId);

        // Do NOT warp — timelock not elapsed
        uint256 earliest = block.timestamp + 2 days;
        vm.prank(signer1);
        vm.expectRevert(abi.encodeWithSelector(ImpactMultisig.TimelockNotElapsed.selector, earliest, block.timestamp));
        multisig.executeAction(proposalId);
    }

    function test_cannot_execute_without_threshold() public {
        bytes memory callData = abi.encodeWithSignature("emergencyDerisk()");

        // Only 1 approval (auto from proposer), threshold is 2
        vm.prank(signer1);
        uint256 proposalId = multisig.proposeAction(callData);

        // Warp past timelock
        vm.warp(block.timestamp + 2 days + 1);

        vm.prank(signer1);
        vm.expectRevert(abi.encodeWithSelector(ImpactMultisig.ThresholdNotMet.selector, 1, 2));
        multisig.executeAction(proposalId);
    }

    function test_emergency_derisk_single_signer() public {
        // Any signer can call emergencyDerisk without threshold
        vm.prank(signer1);
        multisig.emergencyDerisk();

        vm.prank(signer2);
        multisig.emergencyDerisk();

        vm.prank(signer3);
        multisig.emergencyDerisk();
    }

    function test_non_signer_cannot_emergency_derisk() public {
        vm.prank(nonSigner);
        vm.expectRevert(abi.encodeWithSelector(ImpactMultisig.NotASigner.selector, nonSigner));
        multisig.emergencyDerisk();
    }
}
