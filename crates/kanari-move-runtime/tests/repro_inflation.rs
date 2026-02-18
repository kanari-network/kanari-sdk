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
fn test_coin_split_inflation() -> Result<()> {
    // 1. Initialize StateManager
    let mut state = StateManager::new_in_memory();
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;

    // 2. Create Coin A (1000) owned by Alice
    let coin_a_id = "0xAAAA";
    let _coin_a_data_1000 = vec![100, 0, 0, 0, 0, 0, 0, 0]; // Unused, fixed with underscore
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
