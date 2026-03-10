// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import {YieldSplitter} from "../src/YieldSplitter.sol";
import {MockAsset} from "../src/mocks/MockRWAVault.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";

contract YieldSplitterTest is Test {
    YieldSplitter splitter;
    MockAsset token;

    address admin = address(0xAD);
    address walletA = address(0xA);
    address walletB = address(0xB);
    address walletC = address(0xC);

    function setUp() public {
        token = new MockAsset("Test USDC", "tUSDC");

        address[] memory wallets = new address[](2);
        wallets[0] = walletA;
        wallets[1] = walletB;

        uint256[] memory bps = new uint256[](2);
        bps[0] = 6_000; // 60%
        bps[1] = 4_000; // 40%

        splitter = new YieldSplitter(IERC20(address(token)), wallets, bps, admin);
    }

    function test_distribute_splits_correctly() public {
        uint256 balance = 10_000e18;
        token.mint(address(splitter), balance);

        splitter.distribute();

        assertEq(token.balanceOf(walletA), 6_000e18, "Wallet A should get 60%");
        assertEq(token.balanceOf(walletB), 4_000e18, "Wallet B should get 40%");
    }

    function test_distribute_emits_events() public {
        uint256 balance = 10_000e18;
        token.mint(address(splitter), balance);

        vm.expectEmit(true, false, false, true);
        emit YieldSplitter.YieldDistributed(walletA, 6_000e18);

        vm.expectEmit(true, false, false, true);
        emit YieldSplitter.YieldDistributed(walletB, 4_000e18);

        splitter.distribute();
    }

    function test_total_bps_must_equal_10000() public {
        address[] memory wallets = new address[](2);
        wallets[0] = walletA;
        wallets[1] = walletB;

        uint256[] memory bps = new uint256[](2);
        bps[0] = 5_000;
        bps[1] = 3_000; // total = 8000, not 10000

        vm.expectRevert(abi.encodeWithSelector(YieldSplitter.TotalBpsMustEqual10000.selector, 8_000));
        new YieldSplitter(IERC20(address(token)), wallets, bps, admin);
    }

    function test_update_recipients_with_timelock() public {
        // Prepare new recipients
        address[] memory newWallets = new address[](3);
        newWallets[0] = walletA;
        newWallets[1] = walletB;
        newWallets[2] = walletC;

        uint256[] memory newBps = new uint256[](3);
        newBps[0] = 5_000; // 50%
        newBps[1] = 3_000; // 30%
        newBps[2] = 2_000; // 20%

        vm.startPrank(admin);

        // Propose update
        splitter.proposeRecipientUpdate(newWallets, newBps);
        assertTrue(splitter.updatePending());

        // Cannot execute immediately
        vm.expectRevert(
            abi.encodeWithSelector(
                YieldSplitter.TimelockNotElapsed.selector,
                block.timestamp + 2 days,
                block.timestamp
            )
        );
        splitter.executeRecipientUpdate();

        // Warp past timelock
        vm.warp(block.timestamp + 2 days);
        splitter.executeRecipientUpdate();

        vm.stopPrank();

        assertEq(splitter.recipientCount(), 3, "Should have 3 recipients");
        assertFalse(splitter.updatePending(), "Update should no longer be pending");

        // Verify distribution with new splits
        uint256 balance = 10_000e18;
        token.mint(address(splitter), balance);
        splitter.distribute();

        assertEq(token.balanceOf(walletA), 5_000e18, "Wallet A should get 50%");
        assertEq(token.balanceOf(walletB), 3_000e18, "Wallet B should get 30%");
        assertEq(token.balanceOf(walletC), 2_000e18, "Wallet C should get 20%");
    }
}
