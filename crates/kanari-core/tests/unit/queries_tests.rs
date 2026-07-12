use super::*;
use crate::{CheckpointSyncData, consensus::Checkpoint};
use kanari_crypto::keys::{CurveType, generate_keypair};
use kanari_move_runtime_v1::changeset::{ChangeSet, CreatedObject};
use kanari_move_runtime_v1::state::OwnerState;
use kanari_types::address::Address as KanariAddress;
use kanari_types::balance::BalanceRecord;
use kanari_types::coin::{CoinModule, TreasuryCap};
use kanari_types::kanari::KANARI_TOKEN_TYPE;
use kanari_types::transaction::{
    ObjectChange, ObjectChangeKind, ObjectGraphEdge, ObjectGraphEdgeKind, ObjectRef,
    SignedTransaction, Transaction,
};

fn signed_transfer(nonce: u64) -> SignedTransaction {
    let sender = generate_keypair(CurveType::Ed25519).unwrap();
    let recipient = generate_keypair(CurveType::Ed25519).unwrap();
    let mut coin_data = vec![0u8; 40];
    coin_data[32..40].copy_from_slice(&1_000_000u64.to_le_bytes());
    let tx = Transaction::new_transfer_with_object_ref(
        sender.tagged_address(),
        ObjectRef::new(
            "0xaaaa",
            Some(1),
            Some(format!(
                "0x{}",
                hex::encode(kanari_crypto::hash_data_blake3(&coin_data))
            )),
        ),
        recipient.address,
        1,
        nonce,
    );
    let mut signed_tx = SignedTransaction::new(tx);
    signed_tx
        .sign(&sender.private_key, sender.curve_type)
        .unwrap();
    signed_tx
}

fn fund_sender_with_coin(
    engine: &BlockchainEngine,
    owner: move_core_types::account_address::AccountAddress,
    coin_object_id: &str,
    balance: u64,
) {
    let mut coin_data = vec![0u8; 40];
    coin_data[32..40].copy_from_slice(&balance.to_le_bytes());
    let mut state = engine.state.write().unwrap_or_else(|e| e.into_inner());
    let previous_total = state.total_supply;
    let previous_visible = state
        .global_token_supplies
        .get(KANARI_TOKEN_TYPE)
        .copied()
        .unwrap_or(previous_total);

    let mut create_coin = ChangeSet::new();
    create_coin.created_objects.push((
        coin_object_id.to_string(),
        CreatedObject {
            owner,
            owner_kind: kanari_types::transaction::ObjectOwnerKind::AddressOwner(
                owner.to_hex_literal(),
            ),
            uid: None,
            id: None,
            type_: format!("0x2::coin::Coin<{}>", KANARI_TOKEN_TYPE),
            data: coin_data,
            version: 1,
        },
    ));
    state
        .apply_changeset_without_supply_validation(&create_coin)
        .unwrap();

    let mut owner_state = state
        .get_owner_state(&owner)
        .unwrap_or_else(|| OwnerState::new(owner));
    owner_state.set_token_balance(KANARI_TOKEN_TYPE.to_string(), BalanceRecord::new(balance));
    state.save_owner_state(&owner_state).unwrap();

    let updated_total = previous_total.saturating_add(balance);
    let updated_visible = previous_visible.saturating_add(balance);
    state.total_supply = updated_total;
    state.store.save(b"total_supply", &updated_total).unwrap();
    state
        .store
        .save(
            format!("supply:{}", KANARI_TOKEN_TYPE).as_bytes(),
            &TreasuryCap {
                total_supply: updated_total,
            },
        )
        .unwrap();
    state
        .global_token_supplies
        .insert(KANARI_TOKEN_TYPE.to_string(), updated_visible);
    state
        .store
        .save(b"global_token_supplies", &state.global_token_supplies)
        .unwrap();
}

#[test]
fn fresh_engine_owner_query_exposes_separate_genesis_native_gas_coin() {
    let engine = BlockchainEngine::new_in_memory().unwrap();
    assert_fresh_engine_exposes_separate_genesis_native_gas_coin(&engine);
}

#[test]
fn fresh_persistent_engine_owner_query_exposes_separate_genesis_native_gas_coin() {
    let temp_dir = tempfile::tempdir().unwrap();
    let engine = BlockchainEngine::new_dir(temp_dir.path().to_str().unwrap()).unwrap();
    assert_fresh_engine_exposes_separate_genesis_native_gas_coin(&engine);
}

fn assert_fresh_engine_exposes_separate_genesis_native_gas_coin(engine: &BlockchainEngine) {
    let owner = KanariAddress::DEV_ADDRESS;
    let native_coin_type = CoinModule::coin_type(KANARI_TOKEN_TYPE);

    let owner_info = engine
        .get_owner_info(owner)
        .expect("dev owner should exist after genesis");
    let owned_objects = owner_info
        .owned_objects
        .expect("owner query should include object list");
    let native_coin_ids: Vec<_> = owned_objects
        .into_iter()
        .filter(|object| object.type_ == native_coin_type)
        .map(|object| object.id)
        .collect();

    assert!(
        native_coin_ids.len() >= 2,
        "fresh engine query must expose separate native transfer and gas coin objects, found {:?}",
        native_coin_ids
    );
}

#[test]
fn sync_checkpoint_from_data_rejects_empty_checkpoint() {
    let engine = BlockchainEngine::new_in_memory().unwrap();
    let prev_hash = {
        let chain = engine.blockchain.read().unwrap_or_else(|e| e.into_inner());
        chain.latest_checkpoint().hash().unwrap()
    };
    let state_root = engine
        .state
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .compute_state_root();
    let checkpoint = Checkpoint::new(1, vec![], vec![], state_root, 42, prev_hash);
    let sync_data = CheckpointSyncData { checkpoint };

    let error = engine.sync_checkpoint_from_data(&sync_data).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Refusing to sync empty checkpoint")
    );
    assert_eq!(engine.get_stats().height, 0);
}

#[test]
fn sync_checkpoint_from_data_rejects_root_mismatch() {
    let engine = BlockchainEngine::new_in_memory().unwrap();
    let prev_hash = {
        let chain = engine.blockchain.read().unwrap_or_else(|e| e.into_inner());
        chain.latest_checkpoint().hash().unwrap()
    };
    let signed_tx = signed_transfer(0);
    let sender =
        KanariAddress::parse_to_account_address(signed_tx.transaction.sender_address()).unwrap();
    fund_sender_with_coin(&engine, sender, "0xaaaa", 1_000_000);
    let checkpoint = Checkpoint::new(1, vec![], vec![signed_tx], vec![9u8; 32], 42, prev_hash);
    let sync_data = CheckpointSyncData { checkpoint };

    let error = engine.sync_checkpoint_from_data(&sync_data).unwrap_err();
    assert!(
        error.to_string().contains("state root mismatch")
            || error
                .to_string()
                .contains("cannot overlap with a mutable object input")
    );
    assert_eq!(engine.get_stats().height, 0);
}

#[test]
fn block_queries_include_checkpoint_object_changes() {
    let engine = BlockchainEngine::new_in_memory().unwrap();
    let prev_hash = {
        let chain = engine.blockchain.read().unwrap_or_else(|e| e.into_inner());
        chain.latest_checkpoint().hash().unwrap()
    };
    let checkpoint = Checkpoint::new(1, vec![], vec![], vec![9u8; 32], 42, prev_hash)
        .with_object_changes(vec![ObjectChange {
            change_type: ObjectChangeKind::Created,
            object_ref: ObjectRef::new("0x1", Some(1), Some("0xabc".to_string())),
            previous_object_ref: None,
            type_: Some("0x2::test::Thing".to_string()),
            owner: None,
            previous_owner: None,
            previous_version: None,
        }])
        .with_object_graph_edges(vec![ObjectGraphEdge {
            source_object_ref: ObjectRef::new("0xgas", Some(1), Some("0xdef".to_string())),
            target_object_ref: ObjectRef::new("0x1", Some(1), Some("0xabc".to_string())),
            relation: ObjectGraphEdgeKind::GasCreate,
        }]);

    {
        let mut chain = engine.blockchain.write().unwrap_or_else(|e| e.into_inner());
        chain
            .add_checkpoint_with_validation(checkpoint, false)
            .unwrap();
    }

    let block = engine.get_block(1).expect("block should exist");
    let full_block = engine.get_full_block(1).expect("full block should exist");
    assert_eq!(block.object_changes.len(), 1);
    assert_eq!(full_block.object_changes.len(), 1);
    assert_eq!(block.object_changes[0].object_ref.object_id, "0x1");
    assert_eq!(block.object_graph_edges.len(), 1);
    assert_eq!(full_block.object_graph_edges.len(), 1);
}
