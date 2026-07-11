use super::BlockchainEngine;
use crate::consensus::Checkpoint;
use kanari_crypto::keys::{CurveType, generate_keypair};
use kanari_move_runtime_v1::changeset::{ChangeSet, CreatedObject};
use kanari_move_runtime_v1::state::OwnerState;
use kanari_types::address::Address as KanariAddress;
use kanari_types::balance::BalanceRecord;
use kanari_types::coin::{CoinModule, TreasuryCap};
use kanari_types::kanari::{KANARI_TOKEN_TYPE, KanariModule};
use kanari_types::transaction::{
    GasPayment, ObjectInput, ObjectOwnerKind, ObjectRef, SignedTransaction, Transaction,
};
use move_core_types::account_address::AccountAddress;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn native_coin_object_ref(object_id: &str, balance: u64) -> ObjectRef {
    let mut coin_data = vec![0u8; 40];
    coin_data[32..40].copy_from_slice(&balance.to_le_bytes());
    ObjectRef::new(
        object_id.to_string(),
        Some(1),
        Some(format!(
            "0x{}",
            hex::encode(kanari_crypto::hash_data_blake3(&coin_data))
        )),
    )
}

fn signed_transfer_from(sender: &kanari_crypto::keys::KeyPair, nonce: u64) -> SignedTransaction {
    let recipient = generate_keypair(CurveType::Ed25519).unwrap();
    let tx = Transaction::new_transfer_with_object_ref(
        sender.tagged_address(),
        native_coin_object_ref("0xaaaa", 1_000_000),
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

fn transaction_coin_balance(data: &[u8]) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[32..40]);
    u64::from_le_bytes(bytes)
}

fn fund_sender_with_coin(
    engine: &BlockchainEngine,
    address: &str,
    coin_object_id: &str,
    balance: u64,
) {
    fund_sender_with_coin_type(engine, address, coin_object_id, balance, KANARI_TOKEN_TYPE);
}

fn fund_sender_with_coin_type(
    engine: &BlockchainEngine,
    address: &str,
    coin_object_id: &str,
    balance: u64,
    token_type: &str,
) {
    let addr = AccountAddress::from_hex_literal(address).unwrap();
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
            owner: addr,
            owner_kind: kanari_types::transaction::ObjectOwnerKind::AddressOwner(
                addr.to_hex_literal(),
            ),
            uid: None,
            id: None,
            type_: CoinModule::coin_type(token_type),
            data: coin_data,
            version: 1,
        },
    ));
    state
        .apply_changeset_without_supply_validation(&create_coin)
        .unwrap();

    let mut owner_state = state
        .get_owner_state(&addr)
        .unwrap_or_else(|| OwnerState::new(addr));
    owner_state.set_token_balance(token_type.to_string(), BalanceRecord::new(balance));
    state.save_owner_state(&owner_state).unwrap();

    if token_type == KANARI_TOKEN_TYPE {
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
}

#[test]
fn account_info_uses_ledger_native_balance_over_coin_object_amount() {
    let engine = BlockchainEngine::new_in_memory().unwrap();
    let owner = AccountAddress::from_hex_literal("0x1111").unwrap();
    let stale_object_balance = 1_100_000u64;
    let ledger_balance_after_fee = stale_object_balance - 210;
    let mut coin_data = vec![0u8; 40];
    coin_data[32..40].copy_from_slice(&stale_object_balance.to_le_bytes());

    let mut cs = ChangeSet::new();
    cs.created_objects.push((
        "0xcoin".to_string(),
        CreatedObject {
            owner,
            owner_kind: kanari_types::transaction::ObjectOwnerKind::AddressOwner(
                owner.to_hex_literal(),
            ),
            uid: None,
            id: None,
            type_: CoinModule::coin_type(KANARI_TOKEN_TYPE),
            data: coin_data,
            version: 1,
        },
    ));
    {
        let mut state = engine.state.write().unwrap_or_else(|e| e.into_inner());
        state
            .apply_changeset_without_supply_validation(&cs)
            .unwrap();
        let mut owner_state = OwnerState::with_native_balance(owner, ledger_balance_after_fee);
        owner_state.set_token_balance(
            KANARI_TOKEN_TYPE.to_string(),
            BalanceRecord::new(ledger_balance_after_fee),
        );
        state.save_owner_state(&owner_state).unwrap();
    }

    let account = engine.get_owner_info("0x1111").unwrap();
    assert_eq!(
        account.balances.get(KANARI_TOKEN_TYPE).copied(),
        Some(ledger_balance_after_fee)
    );
}

#[test]
fn backend_native_burn_uses_prepared_gas_coin_and_reduces_supply() {
    let engine = BlockchainEngine::new_in_memory().unwrap();
    let owner = "0x1111";
    let object_id = "0xcoin";
    let starting_balance = 1_000_000u64;
    let burn_amount = 100_000u64;
    fund_sender_with_coin(&engine, owner, object_id, starting_balance);

    let initial_supply = {
        let state = engine.state.read().unwrap_or_else(|e| e.into_inner());
        state.total_supply
    };

    let mut tx = Transaction::new_burn_with_gas(owner.to_string(), burn_amount, 1, 100_000, 1);
    if let Transaction::ExecuteFunction { gas_payment, .. } = &mut tx {
        *gas_payment = Some(GasPayment {
            payment_objects: vec![native_coin_object_ref(object_id, starting_balance)],
            owner: owner.to_string(),
            budget: 100_000,
            price: 1,
        });
    }

    let changeset = engine
        .execute_transaction_with_runtime_internal(
            &tx,
            &engine.runtime_pool[0],
            &engine.state,
            false,
            None,
            false,
        )
        .unwrap();
    assert!(changeset.success, "{:?}", changeset.error_message);

    {
        let mut state = engine.state.write().unwrap_or_else(|e| e.into_inner());
        state.apply_changeset(&changeset).unwrap();

        let coin = state
            .get_object(object_id)
            .unwrap()
            .expect("coin must exist");
        let expected_balance = starting_balance - burn_amount - changeset.gas_used;
        assert_eq!(transaction_coin_balance(&coin.data), expected_balance);
        assert_eq!(
            state
                .resolve_owner_native_balance(AccountAddress::from_hex_literal(owner).unwrap())
                .unwrap(),
            expected_balance
        );
        assert_eq!(state.total_supply, initial_supply - burn_amount);
    }
}

fn secure_consensus_keys(
    authorities: &[String],
    local_authority: &str,
) -> (ed25519_dalek::SigningKey, BTreeMap<String, Vec<u8>>) {
    let mut public_keys = BTreeMap::new();
    let mut local_signing_key = None;

    for (index, authority) in authorities.iter().enumerate() {
        let seed = [index as u8 + 11; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
        if authority == local_authority {
            local_signing_key = Some(signing_key.clone());
        }
        public_keys.insert(
            authority.clone(),
            signing_key.verifying_key().to_bytes().to_vec(),
        );
    }

    (
        local_signing_key.expect("local authority must be in authority set"),
        public_keys,
    )
}

#[test]
fn mainnet_defaults_enable_strict_runtime_guards() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    unsafe {
        std::env::set_var("KANARI_NETWORK", "mainnet");
        std::env::remove_var("KANARI_REQUIRE_PERSISTENT_STORAGE");
        std::env::remove_var("KANARI_STRICT_CHECKPOINT_ROOTS");
    }

    assert!(BlockchainEngine::strict_persistence_required());
    assert!(BlockchainEngine::strict_checkpoint_roots_required());

    unsafe {
        std::env::remove_var("KANARI_NETWORK");
    }
}

#[test]
fn devnet_defaults_enable_strict_checkpoint_roots() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    unsafe {
        std::env::set_var("KANARI_NETWORK", "devnet");
        std::env::remove_var("KANARI_STRICT_CHECKPOINT_ROOTS");
    }

    assert!(BlockchainEngine::strict_checkpoint_roots_required());

    unsafe {
        std::env::remove_var("KANARI_NETWORK");
    }
}

#[test]
fn local_network_defaults_allow_relaxed_checkpoint_roots() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    unsafe {
        std::env::set_var("KANARI_NETWORK", "local");
        std::env::remove_var("KANARI_STRICT_CHECKPOINT_ROOTS");
    }

    assert!(!BlockchainEngine::strict_checkpoint_roots_required());

    unsafe {
        std::env::remove_var("KANARI_NETWORK");
    }
}

#[test]
fn explicit_env_overrides_strict_runtime_guards() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    unsafe {
        std::env::set_var("KANARI_NETWORK", "mainnet");
        std::env::set_var("KANARI_REQUIRE_PERSISTENT_STORAGE", "false");
        std::env::set_var("KANARI_STRICT_CHECKPOINT_ROOTS", "0");
    }

    assert!(!BlockchainEngine::strict_persistence_required());
    assert!(!BlockchainEngine::strict_checkpoint_roots_required());

    unsafe {
        std::env::remove_var("KANARI_NETWORK");
        std::env::remove_var("KANARI_REQUIRE_PERSISTENT_STORAGE");
        std::env::remove_var("KANARI_STRICT_CHECKPOINT_ROOTS");
    }
}

#[test]
fn dag_engine_requires_explicit_consensus_signing_key() {
    let engine = BlockchainEngine::new_in_memory().unwrap();

    let err = engine.produce_checkpoint().unwrap_err();

    assert!(err.to_string().contains("requires an explicit signing key"));
}

#[test]
fn configured_dag_engine_rejects_empty_checkpoint() {
    let mut engine = BlockchainEngine::new_in_memory().unwrap();
    let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];
    engine.set_authorities("0x1".to_string(), authorities.clone());
    let (local_key, public_keys) = secure_consensus_keys(&authorities, "0x1");
    engine
        .set_consensus_signing_key(local_key, public_keys)
        .unwrap();

    let err = engine.produce_checkpoint().unwrap_err();

    assert!(err.to_string().contains("No new transactions"));
    assert_eq!(engine.get_stats().height, 0);
}

#[test]
fn restarted_engine_does_not_create_empty_dag_progress() {
    let temp_dir = tempfile::tempdir().unwrap();
    let data_dir = temp_dir.path().to_str().unwrap();
    let authorities = vec!["0x1".to_string(), "0x2".to_string(), "0x3".to_string()];

    {
        let mut engine = BlockchainEngine::new_dir(data_dir).unwrap();
        if engine.persistent_store.is_none() {
            return;
        }
        engine.set_authorities("0x1".to_string(), authorities.clone());
        let (local_key, public_keys) = secure_consensus_keys(&authorities, "0x1");
        engine
            .set_consensus_signing_key(local_key, public_keys)
            .unwrap();

        let err = engine.produce_checkpoint().unwrap_err();
        assert!(err.to_string().contains("No new transactions"));
        assert_eq!(engine.get_stats().height, 0);
    }

    let mut restarted = BlockchainEngine::new_dir(data_dir).unwrap();
    if restarted.persistent_store.is_none() {
        return;
    }
    restarted.set_authorities("0x1".to_string(), authorities.clone());
    let (local_key, public_keys) = secure_consensus_keys(&authorities, "0x1");
    restarted
        .set_consensus_signing_key(local_key, public_keys)
        .unwrap();

    assert_eq!(restarted.get_stats().pending_transactions, 0);
    assert_eq!(restarted.get_stats().height, 0);
    let err = restarted.produce_checkpoint().unwrap_err();
    assert!(err.to_string().contains("No new transactions"));
}

#[test]
fn committed_transaction_history_survives_metadata_stripping() {
    let temp_dir = tempfile::tempdir().unwrap();
    let data_dir = temp_dir.path().to_str().unwrap();
    let engine = BlockchainEngine::new_dir(data_dir).unwrap();
    if engine.persistent_store.is_none() {
        return;
    }

    let sender = generate_keypair(CurveType::Ed25519).unwrap();
    let tx = signed_transfer_from(&sender, 0);
    let tx_hash = tx.transaction_hash().to_vec();
    let genesis_hash = {
        let chain = engine.blockchain.read().unwrap_or_else(|e| e.into_inner());
        chain.latest_checkpoint().hash().unwrap()
    };
    let checkpoint = Checkpoint::new(
        1,
        vec![[7u8; 32]],
        vec![tx],
        vec![9u8; 32],
        42,
        genesis_hash,
    );

    {
        let mut chain = engine.blockchain.write().unwrap_or_else(|e| e.into_inner());
        chain
            .add_checkpoint_with_validation(checkpoint, false)
            .unwrap();
        engine.persist_blockchain_snapshot(&chain).unwrap();
    }

    let latest = engine.list_committed_transactions_from_history(10, |_| true);
    assert_eq!(latest.len(), 1);
    assert_eq!(latest[0].1, 1);
    assert_eq!(latest[0].0.transaction_hash(), tx_hash.as_slice());

    let found = engine
        .get_committed_transaction_from_history(&tx_hash)
        .expect("transaction must be found in persistent history");
    assert_eq!(found.1, 1);
    assert_eq!(found.0.transaction_hash(), tx_hash.as_slice());
}

#[test]
fn batch_submit_accepts_contiguous_sequences_for_same_sender() {
    let engine = BlockchainEngine::new().unwrap();
    let sender = generate_keypair(CurveType::Ed25519).unwrap();
    let tx0 = signed_transfer_from(&sender, 0);
    let tx1 = signed_transfer_from(&sender, 1);

    let hashes = engine.submit_transactions_batch(vec![tx0, tx1]).unwrap();

    assert_eq!(hashes.len(), 2);
    assert_eq!(engine.pending_transaction_len(), 2);
}

#[test]
fn batch_submit_accepts_shuffled_contiguous_sequences_for_same_sender() {
    let engine = BlockchainEngine::new().unwrap();
    let sender = generate_keypair(CurveType::Ed25519).unwrap();
    let tx0 = signed_transfer_from(&sender, 0);
    let tx1 = signed_transfer_from(&sender, 1);
    let tx2 = signed_transfer_from(&sender, 2);

    let hashes = engine
        .submit_transactions_batch(vec![tx2.clone(), tx0.clone(), tx1.clone()])
        .unwrap();

    assert_eq!(hashes.len(), 3);
    let pending = engine.pending_transactions_snapshot();
    let pending_nonces = pending
        .iter()
        .map(|tx| tx.transaction.nonce())
        .collect::<Vec<_>>();
    assert_eq!(pending_nonces, vec![0, 1, 2]);
}

#[test]
fn gas_application_creates_a_spendable_dao_fee_coin() {
    let sender = AccountAddress::random();
    let mut changeset = ChangeSet::new();

    BlockchainEngine::apply_gas_and_sequence(&mut changeset, sender, 10, 10, &[7; 32]).unwrap();

    let sender_owner_delta = changeset.owner_deltas.get(&sender).unwrap();
    assert_eq!(sender_owner_delta.balance_delta, -10);
    let dao = AccountAddress::from_hex_literal(KanariAddress::DAO_ADDRESS).unwrap();
    assert_eq!(changeset.owner_deltas.get(&dao).unwrap().balance_delta, 10);
    assert_eq!(changeset.created_objects.len(), 1);
    let (object_id, fee_coin) = &changeset.created_objects[0];
    assert_eq!(fee_coin.owner, dao);
    assert_eq!(fee_coin.type_, CoinModule::coin_type(KANARI_TOKEN_TYPE));
    assert_eq!(&fee_coin.data[..32], &hex::decode(&object_id[2..]).unwrap());
    assert_eq!(
        u64::from_le_bytes(fee_coin.data[32..40].try_into().unwrap()),
        10
    );

    let mut state = kanari_move_runtime_v1::state::StateManager::new_in_memory();
    state
        .save_owner_state(&OwnerState::with_native_balance(sender, 10))
        .unwrap();
    state
        .apply_changeset_without_supply_validation(&changeset)
        .unwrap();
    assert_eq!(state.resolve_owner_native_balance(dao).unwrap(), 10);
    assert_eq!(
        state.get_owned_objects(&dao).unwrap(),
        vec![object_id.clone()]
    );
    assert_eq!(
        state.get_object(object_id).unwrap().unwrap().data,
        fee_coin.data
    );
}

#[test]
fn batch_submit_rejects_duplicate_transactions() {
    let engine = BlockchainEngine::new().unwrap();
    let sender = generate_keypair(CurveType::Ed25519).unwrap();
    let tx = signed_transfer_from(&sender, 0);

    let err = engine
        .submit_transactions_batch(vec![tx.clone(), tx])
        .unwrap_err();

    assert!(err.to_string().contains("already in pending pool"));
}

#[test]
fn batch_submit_rejects_transaction_already_indexed_in_pending_pool() {
    let engine = BlockchainEngine::new().unwrap();
    let sender = generate_keypair(CurveType::Ed25519).unwrap();
    let tx = signed_transfer_from(&sender, 0);

    engine.submit_transactions_batch(vec![tx.clone()]).unwrap();
    let err = engine.submit_transactions_batch(vec![tx]).unwrap_err();

    assert!(err.to_string().contains("already in pending pool"));
}

#[test]
fn batch_submit_accepts_sequence_gaps() {
    let engine = BlockchainEngine::new().unwrap();
    let sender = generate_keypair(CurveType::Ed25519).unwrap();
    let tx = signed_transfer_from(&sender, 1);

    let hashes = engine.submit_transactions_batch(vec![tx]).unwrap();

    assert_eq!(hashes.len(), 1);
}

#[test]
fn deterministic_parallel_execution_matches_strict_serial_root() {
    let engine = BlockchainEngine::new_in_memory().unwrap();
    let mut txs = Vec::new();

    for i in 0..16 {
        let sender = generate_keypair(CurveType::Ed25519).unwrap();
        let recipient = generate_keypair(CurveType::Ed25519).unwrap();
        let coin_object_id = format!("0x{:0>64x}", i + 1);
        let gas_object_id = format!("0x{:0>64x}", i + 101);
        fund_sender_with_coin(&engine, &sender.address, &coin_object_id, 1_000_000);
        fund_sender_with_coin(&engine, &sender.address, &gas_object_id, 1_000_000);

        let mut tx = Transaction::new_transfer_with_object_ref_and_gas(
            sender.tagged_address(),
            native_coin_object_ref(&coin_object_id, 1_000_000),
            recipient.address.clone(),
            1,
            0,
            100_000,
            1_000,
        );
        if let Transaction::ExecuteFunction {
            gas_payment: Some(gas_payment),
            ..
        } = &mut tx
        {
            gas_payment.payment_objects = vec![native_coin_object_ref(&gas_object_id, 1_000_000)];
        }
        let mut signed_tx = SignedTransaction::new(tx);
        signed_tx
            .sign(&sender.private_key, sender.curve_type)
            .unwrap();
        txs.push(signed_tx);
    }

    let base_state = engine
        .state
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let strict_state = Arc::new(RwLock::new(base_state.clone()));
    let parallel_state = Arc::new(RwLock::new(base_state));

    let strict_counts = engine
        .execute_tx_waves_parallel(txs.clone(), &strict_state, Some(123), false, true)
        .unwrap();
    let parallel_counts = engine
        .execute_tx_waves_deterministic_parallel(txs, &parallel_state, Some(123), false)
        .unwrap();

    let strict_root = strict_state
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .compute_state_root();
    let parallel_root = parallel_state
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .compute_state_root();

    assert_eq!(strict_counts, parallel_counts);
    assert_eq!(strict_root, parallel_root);
}

#[test]
fn non_native_execute_function_requires_full_object_ref_metadata() {
    let engine = BlockchainEngine::new_in_memory().unwrap();
    let sender = generate_keypair(CurveType::Ed25519).unwrap();
    fund_sender_with_coin(&engine, &sender.address, "0xaaaa", 1_000_000);

    let tx = Transaction::ExecuteFunction {
        sender: sender.tagged_address(),
        module: CoinModule::module_path(),
        function: CoinModule::function_names().join_entry.to_string(),
        type_args: vec![KANARI_TOKEN_TYPE.to_string()],
        args: vec![],
        object_inputs: vec![ObjectInput {
            object_ref: ObjectRef::new("0xaaaa", None, None),
            owner: Some(ObjectOwnerKind::AddressOwner(sender.address.clone())),
            mutable: true,
        }],
        gas_payment: Some(GasPayment {
            payment_objects: vec![ObjectRef::new("0xaaaa", None, None)],
            owner: sender.address.clone(),
            budget: 100_000,
            price: 1,
        }),
        gas_limit: 100_000,
        gas_price: 1,
        nonce: 0,
    };
    let mut signed_tx = SignedTransaction::new(tx);
    signed_tx
        .sign(&sender.private_key, sender.curve_type)
        .unwrap();

    let err = engine.execute_transaction_immediate(signed_tx).unwrap_err();
    assert!(
        err.to_string()
            .contains("must include (object_id, version, digest)")
    );
}

#[test]
fn gas_payment_object_must_be_native_kanari_coin() {
    let engine = BlockchainEngine::new_in_memory().unwrap();
    let sender = generate_keypair(CurveType::Ed25519).unwrap();
    fund_sender_with_coin_type(
        &engine,
        &sender.address,
        "0xaaaa",
        1_000_000,
        "0x2::james::JAMES",
    );

    let tx = Transaction::ExecuteFunction {
        sender: sender.tagged_address(),
        module: KanariModule::module_path(),
        function: "burn_amount".to_string(),
        type_args: vec![],
        args: vec![bcs::to_bytes(&1u64).unwrap()],
        object_inputs: vec![],
        gas_payment: Some(GasPayment {
            payment_objects: vec![native_coin_object_ref("0xaaaa", 1_000_000)],
            owner: sender.address.clone(),
            budget: 100_000,
            price: 1,
        }),
        gas_limit: 100_000,
        gas_price: 1,
        nonce: 0,
    };
    let mut signed_tx = SignedTransaction::new(tx);
    signed_tx
        .sign(&sender.private_key, sender.curve_type)
        .unwrap();

    let err = engine.execute_transaction_immediate(signed_tx).unwrap_err();
    assert!(err.to_string().contains("must be Coin<"));
}

#[test]
fn move_transfer_rejects_gas_payment_overlap_with_transfer_coin() {
    let engine = BlockchainEngine::new_in_memory().unwrap();
    let sender = generate_keypair(CurveType::Ed25519).unwrap();
    fund_sender_with_coin(&engine, &sender.address, "0xaaaa", 1_000_000);

    let tx = Transaction::new_transfer_with_object_ref(
        sender.tagged_address(),
        native_coin_object_ref("0xaaaa", 1_000_000),
        generate_keypair(CurveType::Ed25519).unwrap().address,
        1,
        0,
    );
    let mut signed_tx = SignedTransaction::new(tx);
    signed_tx
        .sign(&sender.private_key, sender.curve_type)
        .unwrap();

    assert!(
        engine
            .execute_transaction_immediate(signed_tx)
            .unwrap_err()
            .to_string()
            .contains("cannot overlap with a mutable object input")
    );
}

#[test]
fn non_native_execute_function_still_rejects_gas_overlap_with_mutable_input() {
    let engine = BlockchainEngine::new_in_memory().unwrap();
    let sender = generate_keypair(CurveType::Ed25519).unwrap();
    fund_sender_with_coin(&engine, &sender.address, "0xaaaa", 1_000_000);

    let tx = Transaction::ExecuteFunction {
        sender: sender.tagged_address(),
        module: CoinModule::module_path(),
        function: CoinModule::function_names().join_entry.to_string(),
        type_args: vec![KANARI_TOKEN_TYPE.to_string()],
        args: vec![],
        object_inputs: vec![ObjectInput {
            object_ref: native_coin_object_ref("0xaaaa", 1_000_000),
            owner: Some(ObjectOwnerKind::AddressOwner(sender.address.clone())),
            mutable: true,
        }],
        gas_payment: Some(GasPayment {
            payment_objects: vec![native_coin_object_ref("0xaaaa", 1_000_000)],
            owner: sender.address.clone(),
            budget: 100_000,
            price: 1,
        }),
        gas_limit: 100_000,
        gas_price: 1,
        nonce: 0,
    };
    let mut signed_tx = SignedTransaction::new(tx);
    signed_tx
        .sign(&sender.private_key, sender.curve_type)
        .unwrap();

    let err = engine.execute_transaction_immediate(signed_tx).unwrap_err();
    assert!(
        err.to_string()
            .contains("cannot overlap with a mutable object input")
    );
}
