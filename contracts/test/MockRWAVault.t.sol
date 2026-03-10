// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {MockRWAVault, MockAsset} from "../src/mocks/MockRWAVault.sol";

contract MockRWAVaultTest is Test {
    MockRWAVault vault;
    MockAsset asset;
    address alice = address(0xA11CE);

    function setUp() public {
        vault = new MockRWAVault("RWA Vault", "vRWA", 450); // 4.5% APY
        asset = vault.underlyingAsset();
    }

    function test_deposit() public {
        uint256 depositAmount = 1_000e18;

        // Mint underlying to alice and approve
        asset.mint(alice, depositAmount);
        vm.startPrank(alice);
        asset.approve(address(vault), depositAmount);

        // Deposit and verify shares
        uint256 shares = vault.deposit(depositAmount, alice);
        vm.stopPrank();

        assertEq(shares, depositAmount, "First depositor should get 1:1 shares");
        assertEq(vault.balanceOf(alice), depositAmount, "Alice share balance wrong");
        assertEq(asset.balanceOf(address(vault)), depositAmount, "Vault asset balance wrong");
    }

    function test_yield_accrual() public {
        uint256 depositAmount = 10_000e18;

        // Deposit
        asset.mint(alice, depositAmount);
        vm.startPrank(alice);
        asset.approve(address(vault), depositAmount);
        vault.deposit(depositAmount, alice);
        vm.stopPrank();

        // Warp 1 year
        vm.warp(block.timestamp + 365 days);

        // Accrue yield
        vault.accrueYield();

        // Verify ~4.5% yield: 10_000e18 * 450 / 10000 = 450e18
        uint256 vaultBalance = asset.balanceOf(address(vault));
        uint256 expectedYield = 450e18;
        uint256 expectedTotal = depositAmount + expectedYield;

        // Allow 0.01% tolerance for rounding
        assertApproxEqRel(vaultBalance, expectedTotal, 1e14, "Vault balance should reflect ~4.5% yield");
    }

    function test_withdraw() public {
        uint256 depositAmount = 1_000e18;

        // Deposit
        asset.mint(alice, depositAmount);
        vm.startPrank(alice);
        asset.approve(address(vault), depositAmount);
        vault.deposit(depositAmount, alice);

        // Full withdrawal
        uint256 shares = vault.balanceOf(alice);
        uint256 assetsReceived = vault.redeem(shares, alice, alice);
        vm.stopPrank();

        assertEq(assetsReceived, depositAmount, "Should receive full deposit back");
        assertEq(vault.balanceOf(alice), 0, "Alice should have 0 shares");
        assertEq(asset.balanceOf(alice), depositAmount, "Alice should have assets back");
    }

    function test_configurable_apy() public {
        assertEq(vault.apyBps(), 450, "APY should be 450 bps");

        // Deploy another vault with different APY
        MockRWAVault vault2 = new MockRWAVault("RWA Vault 2", "vRWA2", 800);
        assertEq(vault2.apyBps(), 800, "Second vault APY should be 800 bps");
    }
}
