// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use kanari_move_runtime::changeset::{ChangeSet, CreatedObject};
use kanari_move_runtime::state::StateManager;
use move_core_types::account_address::AccountAddress;

#[test]
fn test_multiple_mints_consolidate_into_single_balance() -> Result<()> {
    // Test that multiple mint operations on the same token type and owner
    // are consolidated into a single balance entry, not fragmented.

    let mut state = StateManager::new_in_memory();
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let token_type = "0x2::kanari::KANARI";

    // First mint: 100 tokens
    println!("=== First Mint (100 tokens) ===");
    let mut cs1 = ChangeSet::new();
    cs1.add_token_balance_set(alice, token_type.to_string(), 100);

    println!(
        "ChangeSet 1 token_balance_sets: {:?}",
        cs1.token_balance_sets.len()
    );
    state.apply_changeset(&cs1)?;

    // Second mint: 50 tokens (should consolidate to 150)
    println!("=== Second Mint (50 tokens) ===");
    let mut cs2 = ChangeSet::new();
    cs2.add_token_balance_set(alice, token_type.to_string(), 50);

    println!(
        "ChangeSet 2 token_balance_sets: {:?}",
        cs2.token_balance_sets.len()
    );
    state.apply_changeset(&cs2)?;

    // Third mint: 75 tokens (should consolidate to 225)
    println!("=== Third Mint (75 tokens) ===");
    let mut cs3 = ChangeSet::new();
    cs3.add_token_balance_set(alice, token_type.to_string(), 75);

    println!(
        "ChangeSet 3 token_balance_sets: {:?}",
        cs3.token_balance_sets.len()
    );
    state.apply_changeset(&cs3)?;

    // Verify final balance
    let alice_account = state
        .get_account(&alice)
        .expect("Alice account should exist");
    let final_balance = alice_account.get_token_balance(token_type);

    println!("Final balance for Alice: {}", final_balance);
    assert_eq!(
        final_balance, 225,
        "Final balance should be 225 (100 + 50 + 75)"
    );

    Ok(())
}

#[test]
fn test_changeset_merge_consolidates_token_balances() -> Result<()> {
    // Test that when two ChangeSets are merged, token balance sets are consolidated
    // rather than duplicated.

    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let token_type = "0x2::kanari::KANARI";

    // Create first ChangeSet with 100 tokens
    let mut cs1 = ChangeSet::new();
    cs1.add_token_balance_set(alice, token_type.to_string(), 100);

    // Create second ChangeSet with 50 tokens for the same (owner, token_type)
    let mut cs2 = ChangeSet::new();
    cs2.add_token_balance_set(alice, token_type.to_string(), 50);

    println!("Before merge:");
    println!("  cs1 token_balance_sets: {}", cs1.token_balance_sets.len());
    println!("  cs2 token_balance_sets: {}", cs2.token_balance_sets.len());

    // Merge cs2 into cs1
    cs1.merge(cs2);

    println!("After merge:");
    println!("  cs1 token_balance_sets: {}", cs1.token_balance_sets.len());
    assert_eq!(
        cs1.token_balance_sets.len(),
        1,
        "Should have only 1 consolidated entry, not 2"
    );

    let (_, _, balance) = &cs1.token_balance_sets[0];
    let combined_amount = balance.value();
    println!("  Combined amount: {}", combined_amount);
    assert_eq!(
        combined_amount, 150,
        "Merged balance should be 150 (100 + 50)"
    );

    Ok(())
}

#[test]
fn test_changeset_consolidation_with_treasury() -> Result<()> {
    // Test that token consolidation works correctly even when treasuries are created
    // (multiple mints typically happen after treasury creation)

    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;
    let token_type = "0x2::kanari::KANARI";

    let mut state = StateManager::new_in_memory();

    // Step 1: Create treasury (Alice is publisher)
    println!("=== Step 1: Create Treasury ===");
    let mut cs_treasury = ChangeSet::new();
    cs_treasury.add_treasury(alice, token_type.to_string(), 1000);
    state.apply_changeset(&cs_treasury)?;

    // Step 2: First mint to Bob
    println!("=== Step 2: First Mint to Bob (100 tokens) ===");
    let mut cs_mint1 = ChangeSet::new();
    cs_mint1.add_token_balance_set(bob, token_type.to_string(), 100);
    state.apply_changeset(&cs_mint1)?;

    // Step 3: Second mint to Bob
    println!("=== Step 3: Second Mint to Bob (75 tokens) ===");
    let mut cs_mint2 = ChangeSet::new();
    cs_mint2.add_token_balance_set(bob, token_type.to_string(), 75);
    state.apply_changeset(&cs_mint2)?;

    // Verify Bob's balance
    let bob_account = state.get_account(&bob).expect("Bob account should exist");
    let bob_balance = bob_account.get_token_balance(token_type);

    println!("Bob's final balance: {}", bob_balance);
    assert_eq!(bob_balance, 175, "Bob should have 175 tokens (100 + 75)");

    Ok(())
}

#[test]
fn test_multiple_owners_and_token_types() -> Result<()> {
    // Test that consolidation works correctly with different owners and token types

    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;
    let token_kanari = "0x2::kanari::KANARI";
    let token_thb = "0x2::thb::THB";

    let mut state = StateManager::new_in_memory();

    // Mint KANARI to Alice twice: 100 + 50 = 150
    println!("=== Alice Mint KANARI (100) ===");
    let mut cs1 = ChangeSet::new();
    cs1.add_token_balance_set(alice, token_kanari.to_string(), 100);
    state.apply_changeset(&cs1)?;

    println!("=== Alice Mint KANARI (50) ===");
    let mut cs2 = ChangeSet::new();
    cs2.add_token_balance_set(alice, token_kanari.to_string(), 50);
    state.apply_changeset(&cs2)?;

    // Mint THB to Bob twice: 200 + 100 = 300
    println!("=== Bob Mint THB (200) ===");
    let mut cs3 = ChangeSet::new();
    cs3.add_token_balance_set(bob, token_thb.to_string(), 200);
    state.apply_changeset(&cs3)?;

    println!("=== Bob Mint THB (100) ===");
    let mut cs4 = ChangeSet::new();
    cs4.add_token_balance_set(bob, token_thb.to_string(), 100);
    state.apply_changeset(&cs4)?;

    // Verify all balances
    let alice_account = state.get_account(&alice).expect("Alice should exist");
    let alice_kanari = alice_account.get_token_balance(token_kanari);

    let bob_account = state.get_account(&bob).expect("Bob should exist");
    let bob_thb = bob_account.get_token_balance(token_thb);

    println!("Alice KANARI balance: {}", alice_kanari);
    println!("Bob THB balance: {}", bob_thb);

    assert_eq!(alice_kanari, 150, "Alice should have 150 KANARI");
    assert_eq!(bob_thb, 300, "Bob should have 300 THB");

    Ok(())
}

#[test]
fn test_balance_updates_immediately_after_second_mint_object() -> Result<()> {
    let mut state = StateManager::new_in_memory();
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let coin_type = "0x2::coin::Coin<0x2::james::JAMES>";
    let token_type = "0x2::james::JAMES";

    let mut first_coin_data = vec![0u8; 32];
    first_coin_data[0] = 0xAA;
    first_coin_data.extend_from_slice(&100u64.to_le_bytes());

    let mut cs1 = ChangeSet::new();
    cs1.add_created_object(
        alice,
        coin_type.to_string(),
        first_coin_data,
        1,
        None,
        Some("0xaaa1".to_string()),
    );
    cs1.add_token_balance_set(alice, token_type.to_string(), 100);
    state.apply_changeset(&cs1)?;

    let first = state
        .get_account(&alice)
        .expect("Alice account should exist after first mint")
        .get_token_balance(token_type);
    assert_eq!(first, 100, "first mint should be visible immediately");

    let mut second_coin_data = vec![0u8; 32];
    second_coin_data[0] = 0xBB;
    second_coin_data.extend_from_slice(&50u64.to_le_bytes());

    let mut cs2 = ChangeSet::new();
    cs2.created_objects.push((
        "0xbbb2".to_string(),
        CreatedObject {
            owner: alice,
            uid: None,
            type_: coin_type.to_string(),
            data: second_coin_data,
            version: 1,
        },
    ));
    cs2.add_token_balance_set(alice, token_type.to_string(), 50);
    state.apply_changeset(&cs2)?;

    let second = state
        .get_account(&alice)
        .expect("Alice account should exist after second mint")
        .get_token_balance(token_type);
    assert_eq!(
        second, 150,
        "second mint should update balance immediately without self-transfer"
    );

    Ok(())
}
