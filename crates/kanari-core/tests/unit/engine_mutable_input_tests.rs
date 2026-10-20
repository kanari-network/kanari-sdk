// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Regression tests for the sender-only rule on mutable declared object
//! inputs: `validate_transaction_object_access` must reject a transaction
//! whose mutable object input is owned by anyone other than the sender,
//! even when the declared owner honestly matches the stored owner.

use super::BlockchainEngine;
use kanari_crypto::hash_data_blake3;
use kanari_crypto::keys::{CurveType, generate_keypair};
use kanari_move_runtime_v2::changeset::{ChangeSet, CreatedObject};
use kanari_types::coin::CoinModule;
use kanari_types::gas_coin::GAS_COIN;
use kanari_types::transaction::{
    GasPayment, ObjectInput, ObjectOwnerKind, ObjectRef, SignedTransaction, Transaction,
};
use move_core_types::account_address::AccountAddress;

fn coin_data(object_id: &str, balance: u64) -> Vec<u8> {
    let mut data = vec![0u8; 40];
    if let Ok(addr) = AccountAddress::from_hex_literal(object_id) {
        data[..32].copy_from_slice(addr.as_ref());
    }
    data[32..40].copy_from_slice(&balance.to_le_bytes());
    data
}

fn coin_ref(object_id: &str, balance: u64) -> ObjectRef {
    ObjectRef::new(
        object_id.to_string(),
        Some(1),
        Some(format!("0x{}", hex::encode(hash_data_blake3(&coin_data(object_id, balance))))),
    )
}

fn fund_coin_object(
    engine: &BlockchainEngine,
    owner_literal: &str,
    coin_object_id: &str,
    balance: u64,
) {
    let addr = AccountAddress::from_hex_literal(owner_literal).unwrap();
    let data = coin_data(coin_object_id, balance);
    let mut state = engine.state.write().unwrap_or_else(|e| e.into_inner());

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
            type_: CoinModule::coin_type(GAS_COIN),
            data,
            version: 1,
        },
    ));
    create_coin.mint(addr, balance);
    state
        .apply_changeset_without_supply_validation(&create_coin)
        .unwrap();
}

fn signed_execute(
    sender: &kanari_crypto::keys::KeyPair,
    declared_owner: &str,
    target_ref: ObjectRef,
    nonce: u64,
) -> SignedTransaction {
    let tx = Transaction::ExecuteFunction {
        sender: sender.tagged_address(),
        module: CoinModule::module_path(),
        function: CoinModule::function_names().join_entry.to_string(),
        type_args: vec![GAS_COIN.to_string()],
        args: vec![],
        object_inputs: vec![ObjectInput {
            object_ref: target_ref,
            owner: Some(ObjectOwnerKind::AddressOwner(declared_owner.to_string())),
            mutable: true,
        }],
        gas_payment: Some(GasPayment {
            payment_objects: vec![],
            owner: sender.address.clone(),
            budget: 1_000_000,
            price: 1,
            coin_type: None,
            price_version: None,
        }),
        gas_limit: 1_000_000,
        gas_price: 1,
        nonce,
    };
    let mut signed_tx = SignedTransaction::new(tx);
    signed_tx
        .sign(&sender.private_key, sender.curve_type)
        .unwrap();
    signed_tx
}

#[test]
fn mutable_object_input_for_foreign_owned_object_is_rejected() {
    let engine = BlockchainEngine::new_in_memory().unwrap();
    let victim = generate_keypair(CurveType::Ed25519).unwrap();
    let attacker = generate_keypair(CurveType::Ed25519).unwrap();

    // 0xbabe is honestly owned by the victim.
    fund_coin_object(&engine, &victim.address, "0xbabe", 1_000_000);

    // The attacker declares the input truthfully (stored owner matches the
    // declared owner, metadata complete) but asks for a mutable alias to the
    // victim's object.
    let signed_tx = signed_execute(
        &attacker,
        &victim.address,
        coin_ref("0xbabe", 1_000_000),
        0,
    );

    let err = engine
        .execute_transaction_immediate(signed_tx)
        .expect_err("cross-owner mutable input must be rejected");
    assert!(
        err.to_string().contains("not the sender"),
        "unexpected error: {err}"
    );
}

#[test]
fn mutable_object_input_for_sender_owned_object_passes_sender_rule() {
    let engine = BlockchainEngine::new_in_memory().unwrap();
    let sender = generate_keypair(CurveType::Ed25519).unwrap();

    fund_coin_object(&engine, &sender.address, "0xaaaa", 1_000_000);

    let signed_tx = signed_execute(
        &sender,
        &sender.address,
        coin_ref("0xaaaa", 1_000_000),
        0,
    );

    // Sender-owned mutable inputs remain legal; any later failure must come
    // from entry semantics, never from the sender rule.
    match engine.execute_transaction_immediate(signed_tx) {
        Ok(_) => {}
        Err(err) => assert!(
            !err.to_string().contains("not the sender"),
            "sender-owned mutable input must not trip the sender rule: {err}"
        ),
    }
}
