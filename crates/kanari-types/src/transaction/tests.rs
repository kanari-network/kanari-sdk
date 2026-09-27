// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::gas_coin::GasModule;
use kanari_crypto::keys::CurveType;
use move_core_types::account_address::AccountAddress;

#[test]
fn transfer_helper_builds_native_execute_function() {
    let coin_object_id = format!("0x{:064x}", 0xaaaau64);
    let canonical_coin_object_id = AccountAddress::from_hex_literal(&coin_object_id)
        .unwrap()
        .to_hex_literal();
    let tx = Transaction::new_transfer_with_object_ref(
        "0x1".to_string(),
        ObjectRef::new(
            coin_object_id.clone(),
            Some(1),
            Some("0xtestdigest".to_string()),
        ),
        "0x2".to_string(),
        42,
        7,
    );

    match &tx {
        Transaction::ExecuteFunction {
            module,
            function,
            nonce,
            ..
        } => {
            assert_eq!(module, &GasModule::module_path());
            assert_eq!(function, GasModule::function_names().transfer);
            assert_eq!(*nonce, 7);
        }
        Transaction::PublishModule { .. }
        | Transaction::PublishPackage { .. }
        | Transaction::UpgradeModule { .. }
        | Transaction::UpgradePackage { .. } => {
            panic!("transfer helper must build a call")
        }
    }

    assert_eq!(
        tx.native_call(),
        Some(NativeCall::Transfer {
            coin_object_id: canonical_coin_object_id,
            recipient: AccountAddress::from_hex_literal("0x2")
                .unwrap()
                .to_hex_literal(),
            amount: 42,
        })
    );
    assert_eq!(tx.tx_type_label(), "transfer");
}

#[test]
fn transfer_helper_builds_execute_function_with_coin_input() {
    let coin_object_id = format!("0x{:064x}", 0xaaaau64);
    let canonical_coin_object_id = AccountAddress::from_hex_literal(&coin_object_id)
        .unwrap()
        .to_hex_literal();
    let tx = Transaction::new_transfer_with_object_ref(
        "0x1".to_string(),
        ObjectRef::new(
            coin_object_id.clone(),
            Some(1),
            Some("0xtestdigest".to_string()),
        ),
        "0x2".to_string(),
        42,
        7,
    );

    match &tx {
        Transaction::ExecuteFunction {
            module,
            function,
            nonce,
            args,
            ..
        } => {
            assert_eq!(module, &GasModule::module_path());
            assert_eq!(function, GasModule::function_names().transfer);
            assert_eq!(*nonce, 7);
            assert_eq!(args.len(), 3);
        }
        Transaction::PublishModule { .. }
        | Transaction::PublishPackage { .. }
        | Transaction::UpgradeModule { .. }
        | Transaction::UpgradePackage { .. } => {
            panic!("object transfer helper must build a call")
        }
    }

    assert_eq!(
        tx.native_call(),
        Some(NativeCall::Transfer {
            coin_object_id: canonical_coin_object_id,
            recipient: AccountAddress::from_hex_literal("0x2")
                .unwrap()
                .to_hex_literal(),
            amount: 42,
        })
    );
}

#[test]
fn native_transfer_uses_abi_coin_id_not_object_input_order() {
    let coin_object_id = format!("0x{:064x}", 0xaaaau64);
    let canonical_coin_object_id = AccountAddress::from_hex_literal(&coin_object_id)
        .unwrap()
        .to_hex_literal();
    let mut tx = Transaction::new_transfer_with_object_ref(
        "0x1".to_string(),
        ObjectRef::new(
            coin_object_id.clone(),
            Some(1),
            Some("0xtestdigest".to_string()),
        ),
        "0x2".to_string(),
        42,
        7,
    );
    let Transaction::ExecuteFunction { object_inputs, .. } = &mut tx else {
        panic!("transfer helper must build a call");
    };
    object_inputs.insert(
        0,
        ObjectInput {
            object_ref: ObjectRef::new(
                format!("0x{:064x}", 0xbbbbu64),
                Some(1),
                Some("0xotherdigest".to_string()),
            ),
            owner: None,
            mutable: false,
        },
    );

    assert_eq!(
        tx.native_call(),
        Some(NativeCall::Transfer {
            coin_object_id: canonical_coin_object_id,
            recipient: AccountAddress::from_hex_literal("0x2")
                .unwrap()
                .to_hex_literal(),
            amount: 42,
        })
    );
}

#[test]
fn transfer_helper_preserves_full_object_ref_metadata() {
    let object_ref = ObjectRef::new("0xaaaa".to_string(), Some(7), Some("0xdigest".to_string()));
    let tx = Transaction::new_transfer_with_object_ref(
        "0x1".to_string(),
        object_ref.clone(),
        "0x2".to_string(),
        42,
        7,
    );

    let object_inputs = tx.object_inputs();
    assert_eq!(object_inputs.len(), 1);
    assert_eq!(object_inputs[0].object_ref, object_ref);

    let gas_payment = tx.gas_payment().expect("transfer should carry gas payment");
    assert_eq!(gas_payment.payment_objects, vec![object_ref]);
}

#[test]
fn non_native_execute_function_does_not_infer_conflicts_from_args_only() {
    let tx = Transaction::ExecuteFunction {
        sender: "0x1".to_string(),
        module: "0x99::demo".to_string(),
        function: "touch".to_string(),
        type_args: vec![],
        args: vec![AccountAddress::from_hex_literal("0xaaaa").unwrap().to_vec()],
        object_inputs: Vec::new(),
        gas_payment: None,
        gas_limit: 100_000,
        gas_price: 1,
        nonce: 0,
    };

    let keys = tx.get_conflict_keys();
    assert_eq!(keys, vec!["owner:0x1".to_string()]);
}

#[test]
fn burn_helper_builds_native_execute_function() {
    let tx = Transaction::new_burn("0x1".to_string(), 9, 3);

    assert_eq!(tx.native_call(), Some(NativeCall::Burn { amount: 9 }));
    assert_eq!(tx.tx_type_label(), "burn");
}

#[test]
fn upgrade_transactions_have_upgrade_type_labels() {
    let gas_payment = GasPayment {
        payment_objects: vec![ObjectRef::new(
            "0xgas",
            Some(7),
            Some("0xdigest".to_string()),
        )],
        owner: "0x1".to_string(),
        budget: 100_000,
        price: 1,
    };
    let module_tx = Transaction::UpgradeModule {
        sender: "0x1".to_string(),
        module_bytes: vec![1, 2, 3],
        module_name: "demo".to_string(),
        gas_payment: Some(gas_payment.clone()),
        gas_limit: 100_000,
        gas_price: 1,
        nonce: 0,
    };
    let package_tx = Transaction::UpgradePackage {
        sender: "0x1".to_string(),
        modules: vec![PublishedModule {
            module_name: "demo".to_string(),
            module_bytes: vec![1, 2, 3],
        }],
        gas_payment: Some(gas_payment.clone()),
        gas_limit: 100_000,
        gas_price: 1,
        nonce: 1,
    };

    assert_eq!(module_tx.tx_type_label(), "upgrade_module");
    assert_eq!(package_tx.tx_type_label(), "upgrade_package");
    assert!(module_tx.requires_strict_object_metadata());
    assert!(package_tx.requires_strict_object_metadata());
    assert!(module_tx.object_inputs().is_empty());
    assert!(package_tx.object_inputs().is_empty());
    assert_eq!(module_tx.gas_payment(), Some(gas_payment.clone()));
    assert_eq!(package_tx.gas_payment(), Some(gas_payment));
    assert_eq!(
        module_tx.get_conflict_keys(),
        vec!["module:demo".to_string(), "object:0xgas".to_string()]
    );
    assert_eq!(
        package_tx.get_conflict_keys(),
        vec!["module:demo".to_string(), "object:0xgas".to_string()]
    );
}

#[test]
fn upgrade_package_conflict_keys_are_deterministic_and_deduplicated() {
    let tx = Transaction::UpgradePackage {
        sender: "0x1".to_string(),
        modules: vec![
            PublishedModule {
                module_name: "zeta".to_string(),
                module_bytes: vec![],
            },
            PublishedModule {
                module_name: "alpha".to_string(),
                module_bytes: vec![],
            },
            PublishedModule {
                module_name: "alpha".to_string(),
                module_bytes: vec![],
            },
        ],
        gas_payment: None,
        gas_limit: 100_000,
        gas_price: 1,
        nonce: 1,
    };

    assert_eq!(
        tx.get_conflict_keys(),
        vec![
            "module:alpha".to_string(),
            "module:zeta".to_string(),
            "owner:0x1".to_string()
        ]
    );
}

#[test]
fn upgrade_transactions_without_gas_fall_back_to_owner_conflict_key() {
    let tx = Transaction::UpgradeModule {
        sender: "0x1".to_string(),
        module_bytes: vec![1, 2, 3],
        module_name: "demo".to_string(),
        gas_payment: None,
        gas_limit: 100_000,
        gas_price: 1,
        nonce: 0,
    };

    assert_eq!(
        tx.get_conflict_keys(),
        vec!["module:demo".to_string(), "owner:0x1".to_string()]
    );
}

#[test]
fn tampering_transaction_after_signing_invalidates_signature_cache() {
    let keypair = kanari_crypto::keys::generate_keypair(CurveType::Ed25519).unwrap();
    let mut signed_tx = SignedTransaction::new(Transaction::new_burn_with_gas(
        keypair.tagged_address(),
        0,
        0,
        100_000,
        0,
    ));
    signed_tx
        .sign(&keypair.private_key, keypair.curve_type)
        .unwrap();

    assert!(signed_tx.verify_signature().unwrap());

    match &mut signed_tx.transaction {
        Transaction::ExecuteFunction { nonce, .. } => *nonce += 1,
        Transaction::PublishModule { .. }
        | Transaction::PublishPackage { .. }
        | Transaction::UpgradeModule { .. }
        | Transaction::UpgradePackage { .. } => {
            unreachable!()
        }
    }

    assert!(!signed_tx.verify_signature().unwrap());
}

#[test]
fn deserialized_transaction_verifies_without_in_memory_signature_cache() {
    let keypair = kanari_crypto::keys::generate_keypair(CurveType::Ed25519).unwrap();
    let mut signed_tx = SignedTransaction::new(Transaction::new_burn_with_gas(
        keypair.tagged_address(),
        0,
        0,
        100_000,
        0,
    ));
    signed_tx
        .sign(&keypair.private_key, keypair.curve_type)
        .unwrap();

    let bytes = bcs::to_bytes(&signed_tx).unwrap();
    let decoded: SignedTransaction = bcs::from_bytes(&bytes).unwrap();

    assert!(decoded.verify_signature().unwrap());
}

#[test]
fn verified_transaction_rejects_tampered_signed_payload() {
    let keypair = kanari_crypto::keys::generate_keypair(CurveType::Ed25519).unwrap();
    let mut signed_tx = SignedTransaction::new(Transaction::new_burn_with_gas(
        keypair.tagged_address(),
        0,
        0,
        100_000,
        0,
    ));
    signed_tx
        .sign(&keypair.private_key, keypair.curve_type)
        .unwrap();

    let verified = signed_tx.clone().into_verified().unwrap();
    assert_eq!(verified.hash(), signed_tx.transaction_hash());

    match &mut signed_tx.transaction {
        Transaction::ExecuteFunction { nonce, .. } => *nonce += 1,
        Transaction::PublishModule { .. }
        | Transaction::PublishPackage { .. }
        | Transaction::UpgradeModule { .. }
        | Transaction::UpgradePackage { .. } => {
            unreachable!()
        }
    }

    assert!(signed_tx.into_verified().is_err());
}

#[test]
fn object_access_identity_unifies_read_write_and_gas_roles() {
    let object_ref = ObjectRef::new("0x42", Some(1), Some("digest".to_string()));
    let tx = Transaction::ExecuteFunction {
        sender: "0x1".to_string(),
        module: "example".to_string(),
        function: "call".to_string(),
        type_args: Vec::new(),
        args: Vec::new(),
        object_inputs: vec![ObjectInput {
            object_ref: object_ref.clone(),
            owner: Some(ObjectOwnerKind::AddressOwner("0x1".to_string())),
            mutable: false,
        }],
        gas_payment: Some(GasPayment {
            payment_objects: vec![object_ref],
            owner: "0x1".to_string(),
            budget: 100,
            price: 1,
        }),
        gas_limit: 100,
        gas_price: 1,
        nonce: 0,
    };

    assert_eq!(tx.object_access_keys(), vec!["object:0x42".to_string()]);
}

#[test]
fn bcs_round_trips_optional_object_metadata() {
    let object_ref = ObjectRef::new("0x42", None, None);
    let bytes = bcs::to_bytes(&object_ref).unwrap();
    assert_eq!(bcs::from_bytes::<ObjectRef>(&bytes).unwrap(), object_ref);
}

#[test]
fn bcs_round_trips_sparse_transaction_effects() {
    let effects = TransactionEffects {
        status: "success".to_string(),
        gas_used: 0,
        gas_payment: None,
        input_objects: Vec::new(),
        shared_inputs: Vec::new(),
        immutable_inputs: Vec::new(),
        gas_object_refs: Vec::new(),
        object_changes: Vec::new(),
        created: Vec::new(),
        mutated: Vec::new(),
        deleted: Vec::new(),
        transferred: Vec::new(),
        causal_edges: Vec::new(),
        error_message: None,
    };
    let bytes = bcs::to_bytes(&effects).unwrap();
    assert_eq!(
        bcs::from_bytes::<TransactionEffects>(&bytes).unwrap(),
        effects
    );
}
