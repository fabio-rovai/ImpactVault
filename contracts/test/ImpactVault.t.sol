// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {ImpactVault} from "../src/ImpactVault.sol";
import {MockAsset} from "../src/mocks/MockRWAVault.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";

contract ImpactVaultTest is Test {
    ImpactVault vault;
    MockAsset asset;

    address admin = address(0xAD);
    address alice = address(0xA11CE);
    address bob = address(0xB0B);
    address yieldSource1 = address(0xF1);
    address yieldSource2 = address(0xF2);

    function setUp() public {
        asset = new MockAsset("Test USDC", "tUSDC");
        vault = new ImpactVault(IERC20(address(asset)), "Impact Vault", "ivUSDC", admin);
    }

    function test_whitelisted_can_deposit() public {
        uint256 depositAmount = 1_000e18;

        // Admin whitelists alice
        vm.prank(admin);
        vault.setWhitelisted(alice, true);

        // Alice deposits
        asset.mint(alice, depositAmount);
        vm.startPrank(alice);
        asset.approve(address(vault), depositAmount);
        uint256 shares = vault.deposit(depositAmount, alice);
        vm.stopPrank();

        assertEq(shares, depositAmount, "Should get 1:1 shares");
        assertEq(vault.balanceOf(alice), depositAmount);
        assertTrue(vault.whitelisted(alice));
    }

    function test_non_whitelisted_cannot_deposit() public {
        uint256 depositAmount = 1_000e18;

        // Bob is NOT whitelisted
        asset.mint(bob, depositAmount);
        vm.startPrank(bob);
        asset.approve(address(vault), depositAmount);

        vm.expectRevert(abi.encodeWithSelector(ImpactVault.NotWhitelisted.selector, bob));
        vault.deposit(depositAmount, bob);
        vm.stopPrank();
    }

    function test_timelock_on_parameter_change() public {
        vm.startPrank(admin);

        // Propose yield source
        vault.proposeYieldSource(yieldSource1);
        assertEq(vault.pendingYieldSource(), yieldSource1);

        // Cannot execute immediately
        vm.expectRevert(
            abi.encodeWithSelector(
                ImpactVault.TimelockNotElapsed.selector,
                block.timestamp + 2 days,
                block.timestamp
            )
        );
        vault.executeYieldSourceChange();

        // Warp past timelock
        vm.warp(block.timestamp + 2 days);
        vault.executeYieldSourceChange();

        assertEq(vault.activeYieldSource(), yieldSource1, "Active yield source should be updated");
        assertEq(vault.pendingYieldSource(), address(0), "Pending should be cleared");

        vm.stopPrank();
    }

    function test_emergency_derisk_only_reduces_risk() public {
        vm.startPrank(admin);

        // Set up an active yield source first
        vault.proposeYieldSource(yieldSource1);
        vm.warp(block.timestamp + 2 days);
        vault.executeYieldSourceChange();
        assertEq(vault.activeYieldSource(), yieldSource1);

        // Propose another change
        vault.proposeYieldSource(yieldSource2);

        // Emergency derisk — clears everything
        vault.emergencyDerisk();
        assertEq(vault.activeYieldSource(), address(0), "Active should be zero after derisk");
        assertEq(vault.pendingYieldSource(), address(0), "Pending should be zero after derisk");
        assertEq(vault.yieldSourceChangeTimestamp(), 0, "Timestamp should be zero after derisk");

        vm.stopPrank();
    }
}
