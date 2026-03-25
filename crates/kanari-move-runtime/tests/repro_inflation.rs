use anyhow::Result;
use kanari_move_runtime::changeset::{ChangeSet, CreatedObject};
use kanari_move_runtime::state::StateManager;
use move_core_types::account_address::AccountAddress;

#[test]
fn test_ownership_transfer() -> Result<()> {
    // 1. Initialize StateManager (in-memory)
    let mut state = StateManager::new_in_memory();

    // 2. Define Alice and Bob
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;

    // 3. Create an object (Coin)
    let object_id = "0xAAAA";
    let object_data = vec![1, 2, 3]; // Dummy data
    let object_type = "0x2::coin::Coin<0x2::kanari::KANARI>";

    let created_obj = CreatedObject {
        owner: alice,
        uid: None,
        type_: object_type.to_string(),
        data: object_data.clone(),
        version: 1,
    };

    // 4. Create ChangeSet to add object for Alice
    let mut cs1 = ChangeSet::new();
    cs1.created_objects
        .push((object_id.to_string(), created_obj));

    // 5. Apply ChangeSet 1
    state.apply_changeset(&cs1)?;

    // Verify Alice owns it
    let alice_owned = state.get_owned_objects(&alice)?;
    assert!(
        alice_owned.contains(&object_id.to_string()),
        "Alice should own the object"
    );

    // Verify Bob does not own it
    let bob_owned = state.get_owned_objects(&bob)?;
    assert!(
        !bob_owned.contains(&object_id.to_string()),
        "Bob should not own the object"
    );

    // 6. Transfer object to Bob (Update object with new owner)
    let updated_obj = CreatedObject {
        owner: bob, // New owner
        uid: None,
        type_: object_type.to_string(),
        data: object_data.clone(),
        version: 2, // Version incremented
    };

    // 7. Create ChangeSet 2 to update object
    let mut cs2 = ChangeSet::new();
    cs2.created_objects
        .push((object_id.to_string(), updated_obj));

    // 8. Apply ChangeSet 2
    state.apply_changeset(&cs2)?;

    // Verify Bob owns it
    let bob_owned_after = state.get_owned_objects(&bob)?;
    assert!(
        bob_owned_after.contains(&object_id.to_string()),
        "Bob should own the object after transfer"
    );

    // Verify Alice NO LONGER owns it (The Fix)
    let alice_owned_after = state.get_owned_objects(&alice)?;
    assert!(
        !alice_owned_after.contains(&object_id.to_string()),
        "Alice should NOT own the object after transfer"
    );

    Ok(())
}

#[test]
fn test_object_id_single_use_no_reuse() -> Result<()> {
    // Test that object_id can only be used once and subsequent uses get new IDs
    let mut state = StateManager::new_in_memory();
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;

    // Create object with explicit ID
    let object_id = "0xAAAA";
    let object_data = vec![1, 2, 3];
    let object_type = "0x2::coin::coin::Coin<0x2::kanari::KANARI>";

    let created_obj = CreatedObject {
        owner: alice,
        uid: None,
        type_: object_type.to_string(),
        data: object_data.clone(),
        version: 1,
    };

    let mut cs1 = ChangeSet::new();
    cs1.created_objects
        .push((object_id.to_string(), created_obj));
    state.apply_changeset(&cs1)?;

    // Verify Alice owns it
    let alice_owned = state.get_owned_objects(&alice)?;
    assert!(alice_owned.contains(&object_id.to_string()));

    // Try to create another object with SAME ID - should get a different ID generated
    let created_obj2 = CreatedObject {
        owner: bob,
        uid: None,
        type_: object_type.to_string(),
        data: vec![4, 5, 6], // Different data
        version: 1,
    };

    let mut cs2 = ChangeSet::new();
    // This should NOT overwrite the first object, but generate a new unique ID
    cs2.created_objects
        .push((object_id.to_string(), created_obj2));
    state.apply_changeset(&cs2)?;

    // Verify Alice still owns the original object
    let alice_owned_after = state.get_owned_objects(&alice)?;
    assert!(alice_owned_after.contains(&object_id.to_string()));
    assert_eq!(
        alice_owned_after.len(),
        1,
        "Alice should still own exactly 1 object"
    );

    // Verify Bob owns an object (should have gotten a new unique ID)
    let bob_owned = state.get_owned_objects(&bob)?;
    assert_eq!(bob_owned.len(), 1, "Bob should own 1 object");
    // Bob's object should NOT have the same ID as Alice's
    assert!(
        !bob_owned.contains(&object_id.to_string()),
        "Bob's object should have a different ID"
    );

    Ok(())
}

#[test]
fn test_object_transfer_removes_from_old_owner() -> Result<()> {
    // Test that transferring an object removes it from old owner's list
    let mut state = StateManager::new_in_memory();
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;
    let charlie = AccountAddress::from_hex_literal("0x3333")?;

    // Create object owned by Alice
    let object_id = "0xAAAA";
    let object_data = vec![1, 2, 3];
    let object_type = "0x2::coin::coin::Coin<0x2::kanari::KANARI>";

    let created_obj = CreatedObject {
        owner: alice,
        uid: None,
        type_: object_type.to_string(),
        data: object_data.clone(),
        version: 1,
    };

    let mut cs1 = ChangeSet::new();
    cs1.created_objects
        .push((object_id.to_string(), created_obj));
    state.apply_changeset(&cs1)?;

    // Verify initial state
    let alice_owned_1 = state.get_owned_objects(&alice)?;
    assert!(alice_owned_1.contains(&object_id.to_string()));
    assert_eq!(alice_owned_1.len(), 1);

    let bob_owned_1 = state.get_owned_objects(&bob)?;
    assert_eq!(bob_owned_1.len(), 0, "Bob should own nothing initially");

    // Transfer to Bob (update with new owner)
    let transferred_obj = CreatedObject {
        owner: bob,
        uid: None,
        type_: object_type.to_string(),
        data: object_data.clone(),
        version: 2,
    };

    let mut cs2 = ChangeSet::new();
    cs2.created_objects
        .push((object_id.to_string(), transferred_obj.clone()));
    state.apply_changeset(&cs2)?;

    // Verify transfer: Alice should NOT own it anymore
    let alice_owned_2 = state.get_owned_objects(&alice)?;
    assert!(
        !alice_owned_2.contains(&object_id.to_string()),
        "Alice should not own the object after transfer"
    );
    assert_eq!(alice_owned_2.len(), 0, "Alice should own 0 objects");

    // Verify Bob owns it
    let bob_owned_2 = state.get_owned_objects(&bob)?;
    assert!(bob_owned_2.contains(&object_id.to_string()));
    assert_eq!(bob_owned_2.len(), 1);

    // Transfer again to Charlie
    let transferred_obj2 = CreatedObject {
        owner: charlie,
        uid: None,
        type_: object_type.to_string(),
        data: object_data.clone(),
        version: 3,
    };

    let mut cs3 = ChangeSet::new();
    cs3.created_objects
        .push((object_id.to_string(), transferred_obj2));
    state.apply_changeset(&cs3)?;

    // Verify second transfer: Bob should NOT own it anymore
    let bob_owned_3 = state.get_owned_objects(&bob)?;
    assert!(
        !bob_owned_3.contains(&object_id.to_string()),
        "Bob should not own the object after second transfer"
    );
    assert_eq!(bob_owned_3.len(), 0, "Bob should own 0 objects");

    // Verify Charlie owns it
    let charlie_owned = state.get_owned_objects(&charlie)?;
    assert!(charlie_owned.contains(&object_id.to_string()));
    assert_eq!(charlie_owned.len(), 1);

    Ok(())
}

#[test]
fn test_coin_split_inflation() -> Result<()> {
    // 1. Initialize StateManager
    let mut state = StateManager::new_in_memory();
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;

    // 2. Create Coin A (1000) owned by Alice
    let coin_a_id = "0xAAAA";
    // Actually, RPC parses last 8 bytes. Let's make data 32 bytes (UID) + 8 bytes (Balance).
    let mut coin_a_data = vec![0u8; 32]; // UID
    coin_a_data.extend_from_slice(&1000u64.to_le_bytes()); // Balance 1000

    let object_type = "0x2::coin::Coin<0x2::kanari::KANARI>";

    let created_a = CreatedObject {
        owner: alice,
        uid: None,
        type_: object_type.to_string(),
        data: coin_a_data.clone(),
        version: 1,
    };

    let mut cs_init = ChangeSet::new();
    cs_init
        .created_objects
        .push((coin_a_id.to_string(), created_a));
    state.apply_changeset(&cs_init)?;

    // Verify Initial State
    let alice_owned = state.get_owned_objects(&alice)?;
    assert!(alice_owned.contains(&coin_a_id.to_string()));
    // We can't easily check balance sum here without RPC logic, but we can check objects.

    // 3. Simulate Transfer Amount (Split 500)
    // Coin A: 1000 -> 500 (Updated, Owner=Alice)
    // Coin B: 500 (Created, Owner=Bob)

    let mut coin_a_data_500 = vec![0u8; 32];
    coin_a_data_500.extend_from_slice(&500u64.to_le_bytes());

    let updated_a = CreatedObject {
        owner: alice,
        uid: None,
        type_: object_type.to_string(),
        data: coin_a_data_500,
        version: 2,
    };

    let coin_b_id = "0xBBBB";
    let mut coin_b_data_500 = vec![0u8; 32];
    coin_b_data_500[0] = 0xBB; // Distinct UID
    coin_b_data_500.extend_from_slice(&500u64.to_le_bytes());

    let created_b = CreatedObject {
        owner: bob,
        uid: None,
        type_: object_type.to_string(),
        data: coin_b_data_500,
        version: 1,
    };

    let mut cs_split = ChangeSet::new();
    cs_split
        .created_objects
        .push((coin_a_id.to_string(), updated_a));
    cs_split
        .created_objects
        .push((coin_b_id.to_string(), created_b));

    state.apply_changeset(&cs_split)?;

    // 4. Verify Final State
    let alice_owned_final = state.get_owned_objects(&alice)?;
    assert!(alice_owned_final.contains(&coin_a_id.to_string()));
    assert!(!alice_owned_final.contains(&coin_b_id.to_string()));
    assert_eq!(alice_owned_final.len(), 1, "Alice should only own Coin A");

    let bob_owned_final = state.get_owned_objects(&bob)?;
    assert!(bob_owned_final.contains(&coin_b_id.to_string()));
    assert_eq!(bob_owned_final.len(), 1, "Bob should only own Coin B");

    // Verify Data of Coin A in DB
    let stored_a = state.get_object(coin_a_id)?.expect("Coin A must exist");
    // Check last 8 bytes
    let stored_balance_bytes: [u8; 8] = stored_a.data[32..40].try_into().unwrap();
    let stored_balance = u64::from_le_bytes(stored_balance_bytes);
    assert_eq!(stored_balance, 500, "Coin A balance should be 500");

    Ok(())
}

#[test]
fn test_transfer_to_owner_with_existing_coin_recomputes_total_balance() -> Result<()> {
    let mut state = StateManager::new_in_memory();
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;
    let coin_type = "0x2::coin::Coin<0x2::kanari::KANARI>";
    let token_type = "0x2::kanari::KANARI";

    // Initial: Alice has 1000, Bob has 700 in a separate coin object.
    let mut a_data = vec![0u8; 32];
    a_data.extend_from_slice(&1000u64.to_le_bytes());
    let mut b_data = vec![1u8; 32];
    b_data.extend_from_slice(&700u64.to_le_bytes());

    let mut cs_init = ChangeSet::new();
    cs_init.created_objects.push((
        "0xAAAA".to_string(),
        CreatedObject {
            owner: alice,
            uid: None,
            type_: coin_type.to_string(),
            data: a_data,
            version: 1,
        },
    ));
    cs_init.created_objects.push((
        "0xBBBB".to_string(),
        CreatedObject {
            owner: bob,
            uid: None,
            type_: coin_type.to_string(),
            data: b_data,
            version: 1,
        },
    ));
    state.apply_changeset(&cs_init)?;

    assert_eq!(
        state
            .get_account(&alice)
            .expect("alice account")
            .get_token_balance(token_type),
        1000
    );
    assert_eq!(
        state
            .get_account(&bob)
            .expect("bob account")
            .get_token_balance(token_type),
        700
    );

    // Transfer-like update: Alice coin reduced to 800 and a new 200 coin is created for Bob.
    let mut a_data_800 = vec![0u8; 32];
    a_data_800.extend_from_slice(&800u64.to_le_bytes());
    let mut c_data_200 = vec![2u8; 32];
    c_data_200.extend_from_slice(&200u64.to_le_bytes());

    let mut cs_transfer = ChangeSet::new();
    cs_transfer.created_objects.push((
        "0xAAAA".to_string(),
        CreatedObject {
            owner: alice,
            uid: None,
            type_: coin_type.to_string(),
            data: a_data_800,
            version: 2,
        },
    ));
    cs_transfer.created_objects.push((
        "0xCCCC".to_string(),
        CreatedObject {
            owner: bob,
            uid: None,
            type_: coin_type.to_string(),
            data: c_data_200,
            version: 1,
        },
    ));
    state.apply_changeset(&cs_transfer)?;

    // Bob must be 700 + 200 = 900, not overwritten to 200.
    assert_eq!(
        state
            .get_account(&alice)
            .expect("alice account")
            .get_token_balance(token_type),
        800
    );
    assert_eq!(
        state
            .get_account(&bob)
            .expect("bob account")
            .get_token_balance(token_type),
        900
    );

    Ok(())
}

#[test]
fn test_recompute_supports_non_canonical_coin_type_string() -> Result<()> {
    let mut state = StateManager::new_in_memory();
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;

    // Observed in some flows: duplicated module path in type string.
    let non_canonical_coin_type = "0x2::coin::coin::Coin<0x2::kanari::KANARI>";
    let token_type = "0x2::kanari::KANARI";

    let mut a_data = vec![0u8; 32];
    a_data.extend_from_slice(&1000u64.to_le_bytes());
    let mut b_data = vec![1u8; 32];
    b_data.extend_from_slice(&300u64.to_le_bytes());

    let mut init_cs = ChangeSet::new();
    init_cs.created_objects.push((
        "0xA001".to_string(),
        CreatedObject {
            owner: alice,
            uid: None,
            type_: non_canonical_coin_type.to_string(),
            data: a_data,
            version: 1,
        },
    ));
    init_cs.created_objects.push((
        "0xB001".to_string(),
        CreatedObject {
            owner: bob,
            uid: None,
            type_: non_canonical_coin_type.to_string(),
            data: b_data,
            version: 1,
        },
    ));
    state.apply_changeset(&init_cs)?;

    // Transfer-like split: Alice 1000 -> 900 and Bob receives +100 coin.
    let mut a_data_900 = vec![0u8; 32];
    a_data_900.extend_from_slice(&900u64.to_le_bytes());
    let mut c_data_100 = vec![2u8; 32];
    c_data_100.extend_from_slice(&100u64.to_le_bytes());

    let mut transfer_cs = ChangeSet::new();
    transfer_cs.created_objects.push((
        "0xA001".to_string(),
        CreatedObject {
            owner: alice,
            uid: None,
            type_: non_canonical_coin_type.to_string(),
            data: a_data_900,
            version: 2,
        },
    ));
    transfer_cs.created_objects.push((
        "0xC001".to_string(),
        CreatedObject {
            owner: bob,
            uid: None,
            type_: non_canonical_coin_type.to_string(),
            data: c_data_100,
            version: 1,
        },
    ));
    state.apply_changeset(&transfer_cs)?;

    assert_eq!(
        state
            .get_account(&alice)
            .expect("alice account")
            .get_token_balance(token_type),
        900
    );
    assert_eq!(
        state
            .get_account(&bob)
            .expect("bob account")
            .get_token_balance(token_type),
        400
    );

    Ok(())
}

#[test]
fn test_transfer_with_same_version_is_not_treated_as_collision() -> Result<()> {
    let mut state = StateManager::new_in_memory();
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;
    let object_id = "0xABCD";
    let coin_type = "0x2::coin::Coin<0x2::kanari::KANARI>";

    let mut coin_data = AccountAddress::from_hex_literal(object_id)?.to_vec();
    coin_data.extend_from_slice(&500u64.to_le_bytes());

    let mut init_cs = ChangeSet::new();
    init_cs.created_objects.push((
        object_id.to_string(),
        CreatedObject {
            owner: alice,
            uid: None,
            type_: coin_type.to_string(),
            data: coin_data.clone(),
            version: 2,
        },
    ));
    state.apply_changeset(&init_cs)?;

    // Same object id/type/size moved to Bob but version is equal (not incremented).
    let mut transfer_cs = ChangeSet::new();
    transfer_cs.created_objects.push((
        object_id.to_string(),
        CreatedObject {
            owner: bob,
            uid: None,
            type_: coin_type.to_string(),
            data: coin_data,
            version: 2,
        },
    ));
    state.apply_changeset(&transfer_cs)?;

    let alice_owned = state.get_owned_objects(&alice)?;
    let bob_owned = state.get_owned_objects(&bob)?;

    assert!(
        !alice_owned.contains(&object_id.to_string()),
        "Alice should no longer own object after transfer"
    );
    assert!(
        bob_owned.contains(&object_id.to_string()),
        "Bob should own the transferred object id"
    );
    assert_eq!(
        bob_owned.len(),
        1,
        "Transfer should not create collision id"
    );

    Ok(())
}

#[test]
fn test_duplicate_created_object_id_keeps_latest_owner() -> Result<()> {
    let mut state = StateManager::new_in_memory();
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;
    let object_id = "0xDEAD";
    let coin_type = "0x2::coin::Coin<0x2::kanari::KANARI>";

    let mut init_data = AccountAddress::from_hex_literal(object_id)?.to_vec();
    init_data.extend_from_slice(&1000u64.to_le_bytes());
    let mut cs_init = ChangeSet::new();
    cs_init.created_objects.push((
        object_id.to_string(),
        CreatedObject {
            owner: alice,
            uid: None,
            type_: coin_type.to_string(),
            data: init_data.clone(),
            version: 1,
        },
    ));
    state.apply_changeset(&cs_init)?;

    // Simulate a malformed/duplicated changeset entry for same object id in one tx:
    // first an old-owner writeback, then final transferred owner.
    let mut new_data = AccountAddress::from_hex_literal(object_id)?.to_vec();
    new_data.extend_from_slice(&1000u64.to_le_bytes());
    let mut cs = ChangeSet::new();
    cs.created_objects.push((
        object_id.to_string(),
        CreatedObject {
            owner: alice,
            uid: None,
            type_: coin_type.to_string(),
            data: init_data,
            version: 2,
        },
    ));
    cs.created_objects.push((
        object_id.to_string(),
        CreatedObject {
            owner: bob,
            uid: None,
            type_: coin_type.to_string(),
            data: new_data,
            version: 2,
        },
    ));
    state.apply_changeset(&cs)?;

    let alice_owned = state.get_owned_objects(&alice)?;
    let bob_owned = state.get_owned_objects(&bob)?;
    assert!(
        !alice_owned.contains(&object_id.to_string()),
        "Alice should not keep duplicate id after final transfer"
    );
    assert!(
        bob_owned.contains(&object_id.to_string()),
        "Latest owner should keep the object id"
    );

    Ok(())
}

#[test]
fn test_collision_with_higher_version_and_mismatched_uid_is_not_transfer() -> Result<()> {
    let mut state = StateManager::new_in_memory();
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;
    let object_id = "0xAAAA";
    let coin_type = "0x2::coin::Coin<0x2::kanari::KANARI>";

    // Existing object id = 0xAAAA (uid prefix matches id)
    let mut existing = [0u8; 32];
    existing[30] = 0xAA;
    existing[31] = 0xAA;
    let mut existing_data = existing.to_vec();
    existing_data.extend_from_slice(&500u64.to_le_bytes());

    let mut init_cs = ChangeSet::new();
    init_cs.created_objects.push((
        object_id.to_string(),
        CreatedObject {
            owner: alice,
            uid: None,
            type_: coin_type.to_string(),
            data: existing_data,
            version: 1,
        },
    ));
    state.apply_changeset(&init_cs)?;

    // New object reuses same object_id string but has different uid prefix, higher version.
    let mut mismatched_uid = [0u8; 32];
    mismatched_uid[0] = 0xBB;
    let mut bad_data = mismatched_uid.to_vec();
    bad_data.extend_from_slice(&500u64.to_le_bytes());

    let mut cs = ChangeSet::new();
    cs.created_objects.push((
        object_id.to_string(),
        CreatedObject {
            owner: bob,
            uid: None,
            type_: coin_type.to_string(),
            data: bad_data,
            version: 2,
        },
    ));
    state.apply_changeset(&cs)?;

    // Should be treated as collision, not transfer:
    // Alice keeps original object id, Bob receives a generated id.
    let alice_owned = state.get_owned_objects(&alice)?;
    let bob_owned = state.get_owned_objects(&bob)?;
    assert!(alice_owned.contains(&object_id.to_string()));
    assert!(!bob_owned.contains(&object_id.to_string()));
    assert_eq!(bob_owned.len(), 1);

    Ok(())
}

#[test]
fn test_multi_token_keeps_system_token_with_mixed_type_format() -> Result<()> {
    let mut state = StateManager::new_in_memory();
    let alice = AccountAddress::from_hex_literal("0x1111")?;

    let system_short = "0x2::kanari::KANARI";
    let system_padded =
        "0x0000000000000000000000000000000000000000000000000000000000000002::kanari::KANARI";
    let custom_token = "0x3::usdt::USDT";

    // First tx: create spendable Coin objects for two token types.
    let mut system_coin_data = vec![0u8; 32];
    system_coin_data.extend_from_slice(&1_000u64.to_le_bytes());
    let mut custom_coin_data = vec![1u8; 32];
    custom_coin_data.extend_from_slice(&500u64.to_le_bytes());

    let mut init_cs = ChangeSet::new();
    init_cs.created_objects.push((
        "0xS001".to_string(),
        CreatedObject {
            owner: alice,
            uid: None,
            type_: format!("0x2::coin::Coin<{}>", system_short),
            data: system_coin_data,
            version: 1,
        },
    ));
    init_cs.created_objects.push((
        "0xU001".to_string(),
        CreatedObject {
            owner: alice,
            uid: None,
            type_: format!("0x2::coin::Coin<{}>", custom_token),
            data: custom_coin_data,
            version: 1,
        },
    ));
    // Also set system token with padded format to ensure normalization consistency.
    init_cs.add_token_balance_set(alice, system_padded.to_string(), 1_000);
    state.apply_changeset(&init_cs)?;

    let account_after_init = state.get_account(&alice).expect("alice account");
    assert_eq!(account_after_init.get_token_balance(system_short), 1_000);
    assert_eq!(account_after_init.get_token_balance(custom_token), 500);
    assert!(account_after_init.token_balances.contains_key(system_short));
    assert!(
        !account_after_init
            .token_balances
            .contains_key(system_padded)
    );

    // Second tx touches only custom token via coin object recompute.
    // System token must remain unchanged.
    let mut custom_coin_data_200 = vec![1u8; 32];
    custom_coin_data_200.extend_from_slice(&200u64.to_le_bytes());
    let mut custom_cs = ChangeSet::new();
    custom_cs.created_objects.push((
        "0xU001".to_string(),
        CreatedObject {
            owner: alice,
            uid: None,
            type_: "0x2::coin::Coin<0x3::usdt::USDT>".to_string(),
            data: custom_coin_data_200,
            version: 2,
        },
    ));
    state.apply_changeset(&custom_cs)?;

    let account_after_custom = state.get_account(&alice).expect("alice account");
    assert_eq!(account_after_custom.get_token_balance(system_short), 1_000);
    assert_eq!(account_after_custom.get_token_balance(custom_token), 200);

    Ok(())
}

#[test]
fn test_token_balance_set_without_coin_object_is_recomputed_to_zero() -> Result<()> {
    let mut state = StateManager::new_in_memory();
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let usdc = "0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146::usdc::USDC";

    let mut cs = ChangeSet::new();
    cs.add_token_balance_set(alice, usdc.to_string(), 1_000_000);
    state.apply_changeset(&cs)?;

    let acc = state.get_account(&alice).expect("alice account");
    assert_eq!(
        acc.get_token_balance(usdc),
        0,
        "balance must be recomputed from owned Coin objects"
    );

    Ok(())
}
