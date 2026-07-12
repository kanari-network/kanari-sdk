use anyhow::Result;
use kanari_move_runtime_v1::changeset::CreatedObject;
use kanari_move_runtime_v1::storage::persistent_store::PersistentStore;
use kanari_move_runtime_v1::{
    ChangeSet,
    state::{OwnerState, StateManager},
};
use kanari_types::error::KanariUnwrapExt;
use kanari_types::kanari::KANARI_TOKEN_TYPE;
use kanari_types::transaction::ObjectOwnerKind;
use move_core_types::account_address::AccountAddress;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::tempdir;

const GENESIS_SUPPLY: u64 = 11_000_000_000_000_000;

fn owned_objects_key(owner: &AccountAddress) -> Vec<u8> {
    let mut key = b"owned_objects:".to_vec();
    key.extend_from_slice(owner.as_ref());
    key
}

fn open_state(db_path: &Path) -> Result<StateManager> {
    let store = Arc::new(PersistentStore::open_with_path(Some(
        db_path.to_path_buf(),
    ))?);
    Ok(StateManager::new(store))
}

fn seed_legacy_supply(db_path: &Path, total_supply: u64) -> Result<()> {
    let store = PersistentStore::open_with_path(Some(db_path.to_path_buf()))?;
    let mut global_supplies = BTreeMap::new();
    global_supplies.insert(KANARI_TOKEN_TYPE.to_string(), total_supply);

    let mut supply_key = b"supply:".to_vec();
    supply_key.extend_from_slice(KANARI_TOKEN_TYPE.as_bytes());

    // Simulate older databases that persisted the native supply but not `total_supply`.
    store.save(&supply_key, &total_supply)?;
    store.save(b"global_token_supplies", &global_supplies)?;
    Ok(())
}

#[test]
fn restart_recovery_preserves_native_supply_invariants() -> Result<()> {
    let temp_dir = tempdir()?;
    let db_path: PathBuf = temp_dir.path().join("state-db");
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;
    let transfer_amount = 1_250_000_000u64;

    seed_legacy_supply(&db_path, GENESIS_SUPPLY)?;

    let mut initial_state = open_state(&db_path)?;
    assert_eq!(initial_state.total_supply, GENESIS_SUPPLY);
    assert_eq!(
        initial_state
            .store
            .load::<u64>(b"total_supply")?
            .invariant("backfilled total_supply must be persisted"),
        GENESIS_SUPPLY
    );
    initial_state.validate_supply_invariants()?;

    initial_state.save_owner_state(&OwnerState::with_native_balance(alice, GENESIS_SUPPLY))?;
    initial_state.commit()?;
    initial_state.validate_supply_invariants()?;
    drop(initial_state);

    let mut reopened_state = open_state(&db_path)?;
    assert_eq!(reopened_state.total_supply, GENESIS_SUPPLY);
    assert_eq!(reopened_state.owner_count(), 1);
    assert_eq!(
        reopened_state
            .load_owner_state(&alice)?
            .invariant("alice owner state must persist")
            .native_balance(),
        GENESIS_SUPPLY
    );
    reopened_state.validate_supply_invariants()?;

    let mut transfer = ChangeSet::new();
    transfer.transfer(alice, bob, transfer_amount);
    reopened_state.apply_changeset(&transfer)?;
    reopened_state.commit()?;
    reopened_state.validate_supply_invariants()?;
    assert_eq!(reopened_state.total_supply, GENESIS_SUPPLY);
    assert_eq!(
        reopened_state
            .load_owner_state(&alice)?
            .invariant("alice owner state must remain present")
            .native_balance(),
        GENESIS_SUPPLY - transfer_amount
    );
    assert_eq!(
        reopened_state
            .load_owner_state(&bob)?
            .invariant("bob owner state must be created by transfer")
            .native_balance(),
        transfer_amount
    );
    drop(reopened_state);

    let final_state = open_state(&db_path)?;
    assert_eq!(final_state.total_supply, GENESIS_SUPPLY);
    assert_eq!(final_state.owner_count(), 2);
    assert_eq!(
        final_state
            .load_owner_state(&alice)?
            .invariant("alice owner state must survive restart")
            .native_balance()
            + final_state
                .load_owner_state(&bob)?
                .invariant("bob owner state must survive restart")
                .native_balance(),
        GENESIS_SUPPLY
    );
    final_state.validate_supply_invariants()?;

    Ok(())
}

#[test]
fn restart_rebuilds_derived_indexes_from_canonical_records() -> Result<()> {
    let temp_dir = tempdir()?;
    let db_path: PathBuf = temp_dir.path().join("state-db");
    let alice = AccountAddress::from_hex_literal("0x1111")?;
    let bob = AccountAddress::from_hex_literal("0x2222")?;

    let mut state = open_state(&db_path)?;
    state.save_owner_state(&OwnerState::with_native_balance(alice, 500))?;
    state.save_owner_state(&OwnerState::with_native_balance(bob, 700))?;

    let mut changeset = ChangeSet::new();
    changeset.created_objects.push((
        "0xaaa1".to_string(),
        CreatedObject {
            owner: alice,
            owner_kind: ObjectOwnerKind::AddressOwner(alice.to_hex_literal()),
            uid: None,
            id: None,
            type_: "0x2::coin::Coin<0x2::kanari::KANARI>".to_string(),
            data: vec![0u8; 40],
            version: 1,
        },
    ));
    changeset.created_objects.push((
        "0xbbb2".to_string(),
        CreatedObject {
            owner: bob,
            owner_kind: ObjectOwnerKind::AddressOwner(bob.to_hex_literal()),
            uid: None,
            id: None,
            type_: "0x2::coin::Coin<0x2::kanari::KANARI>".to_string(),
            data: vec![0u8; 40],
            version: 1,
        },
    ));
    state.apply_changeset_without_supply_validation(&changeset)?;
    state.commit()?;
    let canonical_root_before = state.compute_state_root();
    drop(state);

    let store = PersistentStore::open_with_path(Some(db_path.clone()))?;
    store.save(b"owner_index", &vec![alice.to_hex_literal()])?;
    store.save(b"account_index", &vec!["0xdead".to_string()])?;
    store.save(b"object_index", &vec!["0xdead".to_string()])?;
    store.save(&owned_objects_key(&alice), &vec!["0xdead".to_string()])?;
    store.save(&owned_objects_key(&bob), &Vec::<String>::new())?;

    let reopened = open_state(&db_path)?;
    let owner_ids = reopened
        .owner_addresses()?
        .into_iter()
        .map(|address| address.to_hex_literal())
        .collect::<Vec<_>>();
    assert!(
        owner_ids.contains(&alice.to_hex_literal()),
        "rebuilt owner index should include alice"
    );
    assert!(
        owner_ids.contains(&bob.to_hex_literal()),
        "rebuilt owner index should include bob"
    );
    assert_eq!(
        reopened.get_owned_objects(&alice)?,
        vec!["0xaaa1".to_string()]
    );
    assert_eq!(
        reopened.get_owned_objects(&bob)?,
        vec!["0xbbb2".to_string()]
    );
    let object_index = reopened
        .load_internal::<Vec<String>>(b"object_index")?
        .invariant("object_index should be rebuilt");
    assert!(object_index.contains(&"0xaaa1".to_string()));
    assert!(object_index.contains(&"0xbbb2".to_string()));
    assert!(
        reopened
            .load_internal::<Vec<String>>(b"account_index")?
            .is_none(),
        "legacy account_index should be deleted during startup repair"
    );
    assert_eq!(reopened.compute_state_root(), canonical_root_before);

    Ok(())
}
