// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{
    apply_committed_effect, base_transaction_details, classify_transaction_error_data,
    derive_transaction_state_flags, enrich_transfer_from_effect, fresh_nonce,
    lookup_token_decimals, push_transfer_entry, select_native_coin_consolidation_step,
    select_native_transfer_and_gas_payment, transaction_error_with_reason, transfers_from_effect,
    validate_object_inputs_and_gas, validate_object_inputs_match_state,
};
use crate::RpcServerState;
use kanari_move_runtime_v1::changeset::ChangeSet;
use kanari_rpc_api::TransactionErrorReason;
use kanari_types::coin::CoinModule;
use kanari_types::gas_coin::GAS_COIN;
use proptest::prelude::*;
use std::collections::HashSet;

#[test]
fn transaction_state_flags_match_pending_status() {
    let (success, previewed, submitted, committed) = derive_transaction_state_flags("pending");
    assert!(success);
    assert!(!previewed);
    assert!(submitted);
    assert!(!committed);
}

#[test]
fn transaction_state_flags_match_committed_status() {
    let (success, previewed, submitted, committed) = derive_transaction_state_flags("committed");
    assert!(success);
    assert!(!previewed);
    assert!(submitted);
    assert!(committed);
}

#[test]
fn committed_failed_effect_is_not_reported_as_success() {
    let mut changeset = ChangeSet::new();
    changeset.mark_failed("Move execution failed".to_string());
    changeset.set_gas_used(210);
    let effect = changeset.effects(None);
    let mut details = base_transaction_details(
        "hash".to_string(),
        "committed".to_string(),
        Some(1),
        "transfer",
        "sender".to_string(),
        "0x1".to_string(),
        1,
        100_000,
        1,
    );

    apply_committed_effect(None, &mut details, Some(&effect));

    assert_eq!(details.status, "failed");
    assert!(!details.success);
    assert!(details.submitted);
    assert!(details.committed);
    assert_eq!(details.gas_used, Some(210));
    assert_eq!(
        details.effects.unwrap().error_message.as_deref(),
        Some("Move execution failed")
    );
}

#[test]
fn transaction_state_flags_match_simulated_pending_status() {
    let (success, previewed, submitted, committed) =
        derive_transaction_state_flags("simulated_pending");
    assert!(success);
    assert!(previewed);
    assert!(submitted);
    assert!(!committed);
}

#[test]
fn classifies_invalid_gas_payment_type_error() {
    let data = classify_transaction_error_data(
        "Immediate execution failed: Gas payment object 0xabc must be Coin<0x2::kanari::KANARI>, found 0x2::coin::Coin<0x2::foo::BAR>",
    )
    .expect("classification should exist");
    assert_eq!(data.reason, TransactionErrorReason::InvalidGasPaymentType);
}

#[test]
fn classifies_gas_payment_overlap_error() {
    let data = classify_transaction_error_data(
        "Submission failed: Gas payment object 0xabc cannot overlap with a mutable object input",
    )
    .expect("classification should exist");
    assert_eq!(data.reason, TransactionErrorReason::GasPaymentObjectOverlap);
}

#[test]
fn structured_transaction_error_sets_reason_data() {
    let error = transaction_error_with_reason(
        "Immediate execution failed: Gas payment object 0xabc cannot overlap with a mutable object input",
    );
    assert_eq!(error.code, -32002);
    assert_eq!(
        error.transaction_error_reason(),
        Some(TransactionErrorReason::GasPaymentObjectOverlap)
    );
}

#[test]
fn structured_transaction_error_attaches_native_transfer_policy() {
    let error = transaction_error_with_reason(
        "Transaction error: Native transfer requires two distinct Coin<0x2::kanari::KANARI> objects: one mutable transfer input and one separate gas payment object",
    );
    let details = error
        .transaction_error_details()
        .expect("structured transaction details should exist");
    assert_eq!(
        details.reason,
        TransactionErrorReason::NativeTransferPolicyNotSatisfied
    );
    assert!(details.native_transfer_policy.is_some());
}

#[test]
fn object_input_ref_metadata_must_be_complete() {
    let input = kanari_types::transaction::ObjectInput {
        object_ref: kanari_types::transaction::ObjectRef::new("0x1", Some(1), None),
        owner: Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
            "0xa".to_string(),
        )),
        mutable: true,
    };

    let err = validate_object_inputs_and_gas(42, &[input], None)
        .expect_err("partial object ref metadata must fail");

    assert_eq!(err.error.as_ref().expect("rpc error").code, -32602);
    assert!(
        err.error
            .as_ref()
            .expect("rpc error")
            .message
            .contains("must include both version and digest")
    );
}

#[test]
fn gas_payment_ref_metadata_must_be_complete() {
    let gas_payment = kanari_types::transaction::GasPayment {
        payment_objects: vec![kanari_types::transaction::ObjectRef::new(
            "0x1",
            None,
            Some("digest".to_string()),
        )],
        owner: "0xa".to_string(),
        budget: 1,
        price: 1,
    };

    let err = validate_object_inputs_and_gas(43, &[], Some(&gas_payment))
        .expect_err("partial gas object ref metadata must fail");

    assert_eq!(err.error.as_ref().expect("rpc error").code, -32602);
    assert!(
        err.error
            .as_ref()
            .expect("rpc error")
            .message
            .contains("must include both version and digest")
    );
}

#[test]
fn object_input_must_match_current_state_ref() {
    let engine = kanari_core::BlockchainEngine::new_in_memory().expect("in-memory engine");
    let state = RpcServerState::new(std::sync::Arc::new(engine));
    let input = kanari_types::transaction::ObjectInput {
        object_ref: kanari_types::transaction::ObjectRef::new(
            "0xdead",
            Some(1),
            Some("missing".to_string()),
        ),
        owner: Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
            "0xa".to_string(),
        )),
        mutable: true,
    };

    let err = validate_object_inputs_match_state(&state, 44, &[input])
        .expect_err("missing object input ref must fail");

    assert_eq!(err.error.as_ref().expect("rpc error").code, -32602);
    assert!(
        err.error
            .as_ref()
            .expect("rpc error")
            .message
            .contains("does not match current state ref")
    );
}

#[test]
fn duplicate_mutable_object_inputs_are_rejected_at_rpc_boundary() {
    let input = kanari_types::transaction::ObjectInput {
        object_ref: kanari_types::transaction::ObjectRef::new(
            "0x1",
            Some(1),
            Some("digest".to_string()),
        ),
        owner: Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
            "0xa".to_string(),
        )),
        mutable: true,
    };

    let err = validate_object_inputs_and_gas(45, &[input.clone(), input], None)
        .expect_err("duplicate mutable inputs must fail");

    assert_eq!(err.error.as_ref().expect("rpc error").code, -32602);
    assert!(
        err.error
            .as_ref()
            .expect("rpc error")
            .message
            .contains("Duplicate mutable object input")
    );
}

#[test]
fn equivalent_hex_mutable_object_inputs_are_rejected_at_rpc_boundary() {
    let owner = Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
        "0xa".to_string(),
    ));
    let short = kanari_types::transaction::ObjectInput {
        object_ref: kanari_types::transaction::ObjectRef::new(
            "0x1",
            Some(1),
            Some("digest".to_string()),
        ),
        owner: owner.clone(),
        mutable: true,
    };
    let padded = kanari_types::transaction::ObjectInput {
        object_ref: kanari_types::transaction::ObjectRef::new(
            "0x0000000000000000000000000000000000000000000000000000000000000001",
            Some(1),
            Some("digest".to_string()),
        ),
        owner,
        mutable: true,
    };

    let err = validate_object_inputs_and_gas(48, &[short, padded], None)
        .expect_err("equivalent object ids must fail duplicate check");

    assert_eq!(err.error.as_ref().expect("rpc error").code, -32602);
    assert!(
        err.error
            .as_ref()
            .expect("rpc error")
            .message
            .contains("Duplicate mutable object input")
    );
}

#[test]
fn gas_payment_cannot_overlap_mutable_object_input_at_rpc_boundary() {
    let input = kanari_types::transaction::ObjectInput {
        object_ref: kanari_types::transaction::ObjectRef::new(
            "0x1",
            Some(1),
            Some("digest".to_string()),
        ),
        owner: Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
            "0xa".to_string(),
        )),
        mutable: true,
    };
    let gas_payment = kanari_types::transaction::GasPayment {
        payment_objects: vec![kanari_types::transaction::ObjectRef::new(
            "0x1",
            Some(1),
            Some("digest".to_string()),
        )],
        owner: "0xa".to_string(),
        budget: 1,
        price: 1,
    };

    let err = validate_object_inputs_and_gas(46, &[input], Some(&gas_payment))
        .expect_err("gas must not overlap mutable input");

    assert_eq!(err.error.as_ref().expect("rpc error").code, -32602);
    assert!(
        err.error
            .as_ref()
            .expect("rpc error")
            .message
            .contains("cannot overlap with a mutable object input")
    );
}

#[test]
fn equivalent_hex_gas_overlap_is_rejected_at_rpc_boundary() {
    let input = kanari_types::transaction::ObjectInput {
        object_ref: kanari_types::transaction::ObjectRef::new(
            "0x1",
            Some(1),
            Some("digest".to_string()),
        ),
        owner: Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
            "0xa".to_string(),
        )),
        mutable: true,
    };
    let gas_payment = kanari_types::transaction::GasPayment {
        payment_objects: vec![kanari_types::transaction::ObjectRef::new(
            "0x0000000000000000000000000000000000000000000000000000000000000001",
            Some(1),
            Some("digest".to_string()),
        )],
        owner: "0xa".to_string(),
        budget: 1,
        price: 1,
    };

    let err = validate_object_inputs_and_gas(49, &[input], Some(&gas_payment))
        .expect_err("equivalent gas/mutable input ids must fail overlap check");

    assert_eq!(err.error.as_ref().expect("rpc error").code, -32602);
    assert!(
        err.error
            .as_ref()
            .expect("rpc error")
            .message
            .contains("cannot overlap with a mutable object input")
    );
}

#[test]
fn duplicate_gas_payment_objects_are_rejected_at_rpc_boundary() {
    let gas_ref =
        kanari_types::transaction::ObjectRef::new("0x1", Some(1), Some("digest".to_string()));
    let gas_payment = kanari_types::transaction::GasPayment {
        payment_objects: vec![gas_ref.clone(), gas_ref],
        owner: "0xa".to_string(),
        budget: 1,
        price: 1,
    };

    let err = validate_object_inputs_and_gas(47, &[], Some(&gas_payment))
        .expect_err("duplicate gas refs must fail");

    assert_eq!(err.error.as_ref().expect("rpc error").code, -32602);
    assert!(
        err.error
            .as_ref()
            .expect("rpc error")
            .message
            .contains("Duplicate gas payment object")
    );
}

#[test]
fn invalid_object_input_id_is_rejected_at_rpc_boundary() {
    let input = kanari_types::transaction::ObjectInput {
        object_ref: kanari_types::transaction::ObjectRef::new(
            "not-an-object-id",
            Some(1),
            Some("digest".to_string()),
        ),
        owner: Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
            "0xa".to_string(),
        )),
        mutable: true,
    };

    let err = validate_object_inputs_and_gas(50, &[input], None)
        .expect_err("invalid object id must fail");

    assert_eq!(err.error.as_ref().expect("rpc error").code, -32602);
    assert!(
        err.error
            .as_ref()
            .expect("rpc error")
            .message
            .contains("must be a valid object id")
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// Hex encodings of the same object must remain the same conflict domain.
    /// This prevents a client from bypassing mutable-input/gas overlap checks by
    /// changing only leading zeroes or letter case in an object id.
    #[test]
    fn canonical_object_id_overlap_cannot_bypass_rpc_validation(
        object_id in 1u64..u64::MAX,
        leading_zeroes in 0usize..48,
        uppercase in any::<bool>(),
    ) {
        let compact = format!("0x{object_id:x}");
        let digits = if uppercase {
            format!("{object_id:X}")
        } else {
            format!("{object_id:x}")
        };
        let padded = format!("0x{}{}", "0".repeat(leading_zeroes), digits);
        let input = kanari_types::transaction::ObjectInput {
            object_ref: kanari_types::transaction::ObjectRef::new(
                compact,
                Some(1),
                Some("input-digest".to_string()),
            ),
            owner: Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
                "0xa".to_string(),
            )),
            mutable: true,
        };
        let gas_payment = kanari_types::transaction::GasPayment {
            payment_objects: vec![kanari_types::transaction::ObjectRef::new(
                padded,
                Some(1),
                Some("gas-digest".to_string()),
            )],
            owner: "0xa".to_string(),
            budget: 1,
            price: 1,
        };

        let error = validate_object_inputs_and_gas(51, &[input], Some(&gas_payment))
            .expect_err("canonical-equivalent gas and mutable input must conflict");
        prop_assert!(error
            .error
            .as_ref()
            .expect("rpc error")
            .message
            .contains("cannot overlap with a mutable object input"));
    }

    /// A client cannot turn a mutable object/gas overlap into an allowed input
    /// by changing the object-id spelling.  Conversely, the same reference is
    /// valid as a read-only input: authorization is tied to mutability, not
    /// representation.
    #[test]
    fn overlap_policy_is_invariant_under_object_id_encodings(
        object_id in 1u64..u64::MAX,
        leading_zeroes in 0usize..48,
        uppercase in any::<bool>(),
        mutable in any::<bool>(),
    ) {
        let compact = format!("0x{object_id:x}");
        let digits = if uppercase {
            format!("{object_id:X}")
        } else {
            format!("{object_id:x}")
        };
        let padded = format!("0x{}{}", "0".repeat(leading_zeroes), digits);
        let input = kanari_types::transaction::ObjectInput {
            object_ref: kanari_types::transaction::ObjectRef::new(
                compact,
                Some(1),
                Some("input-digest".to_string()),
            ),
            owner: Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
                "0xa".to_string(),
            )),
            mutable,
        };
        let gas_payment = kanari_types::transaction::GasPayment {
            payment_objects: vec![kanari_types::transaction::ObjectRef::new(
                padded,
                Some(1),
                Some("gas-digest".to_string()),
            )],
            owner: "0xa".to_string(),
            budget: 1,
            price: 1,
        };

        let result = validate_object_inputs_and_gas(52, &[input], Some(&gas_payment));
        if mutable {
            let error = result.expect_err("mutable input must not overlap gas");
            prop_assert!(error
                .error
                .as_ref()
                .expect("rpc error")
                .message
                .contains("cannot overlap with a mutable object input"));
        } else {
            prop_assert!(result.is_ok());
        }
    }

    /// Adversarial object-input metadata must be handled as a normal RPC
    /// validation result, never as a panic.  This covers malformed IDs,
    /// owner declarations, versions, and digests before they reach Move.
    #[test]
    fn malformed_object_input_metadata_is_bounded_at_rpc_validation(
        object_id in ".{0,256}",
        owner in ".{0,256}",
        digest in ".{0,256}",
        version in any::<u64>(),
        mutable in any::<bool>(),
        owner_kind in 0u8..3,
    ) {
        let owner = match owner_kind {
            0 => kanari_types::transaction::ObjectOwnerKind::AddressOwner(owner),
            1 => kanari_types::transaction::ObjectOwnerKind::Shared,
            _ => kanari_types::transaction::ObjectOwnerKind::Immutable,
        };
        let input = kanari_types::transaction::ObjectInput {
            object_ref: kanari_types::transaction::ObjectRef::new(
                object_id,
                Some(version),
                Some(digest),
            ),
            owner: Some(owner),
            mutable,
        };

        let _ = validate_object_inputs_and_gas(51, &[input], None);
    }

    /// RPC clients may submit transaction bytes that are truncated, noncanonical,
    /// or deliberately shaped like a large BCS container.  Decoding must remain
    /// an ordinary fallible operation at this boundary; malformed bytes must not
    /// unwind the RPC worker.
    #[test]
    fn malformed_signed_transaction_bcs_is_fallible(
        payload in prop::collection::vec(any::<u8>(), 0..4096),
    ) {
        let _ = bcs::from_bytes::<kanari_types::transaction::SignedTransaction>(&payload);
    }

    /// Larger adversarial BCS payloads must stay within the normal error path
    /// too.  This guards the RPC decoder against panics when a request mixes
    /// arbitrary bytes with container-sized prefixes.
    #[test]
    fn large_malformed_signed_transaction_bcs_is_fallible(
        payload in prop::collection::vec(any::<u8>(), 4096..16384),
    ) {
        let _ = bcs::from_bytes::<kanari_types::transaction::SignedTransaction>(&payload);
    }
}

#[test]
fn selects_distinct_native_transfer_and_gas_objects() {
    use kanari_rpc_api::ObjectInfo;
    use kanari_types::transaction::ObjectOwnerKind;

    let owned_objects = vec![
        ObjectInfo {
            id: "0x1".to_string(),
            owner: "0xa".to_string(),
            owner_kind: ObjectOwnerKind::AddressOwner("0xa".to_string()),
            type_: CoinModule::coin_type(GAS_COIN),
            data: {
                let mut bytes = vec![0u8; 40];
                bytes[32..40].copy_from_slice(&100u64.to_le_bytes());
                bytes
            },
            version: 1,
            digest: Some("d1".to_string()),
        },
        ObjectInfo {
            id: "0x2".to_string(),
            owner: "0xa".to_string(),
            owner_kind: ObjectOwnerKind::AddressOwner("0xa".to_string()),
            type_: CoinModule::coin_type(GAS_COIN),
            data: {
                let mut bytes = vec![0u8; 40];
                bytes[32..40].copy_from_slice(&50u64.to_le_bytes());
                bytes
            },
            version: 1,
            digest: Some("d2".to_string()),
        },
    ];

    let pending_access_keys = HashSet::new();
    let (coin, gas) = select_native_transfer_and_gas_payment(
        &owned_objects,
        "0xa",
        60,
        10,
        1,
        &pending_access_keys,
        &HashSet::new(),
    )
    .unwrap();
    assert_eq!(coin.object_id, "0x1");
    assert_ne!(coin.object_id, gas.payment_objects[0].object_id);
    assert_eq!(gas.payment_objects[0].object_id, "0x2");
}

#[test]
fn native_transfer_preserves_small_coin_as_gas_reserve() {
    use kanari_rpc_api::ObjectInfo;
    use kanari_types::transaction::ObjectOwnerKind;

    let coin = |id: &str, balance: u64| ObjectInfo {
        id: id.to_string(),
        owner: "0xa".to_string(),
        owner_kind: ObjectOwnerKind::AddressOwner("0xa".to_string()),
        type_: CoinModule::coin_type(GAS_COIN),
        data: {
            let mut bytes = vec![0u8; 40];
            bytes[32..40].copy_from_slice(&balance.to_le_bytes());
            bytes
        },
        version: 1,
        digest: Some(format!("{id}:digest")),
    };
    let owned_objects = vec![
        coin("0xsmall", 1_000_000_000),
        coin("0xlarge", 11_000_000_000),
    ];

    let (transfer, gas) = select_native_transfer_and_gas_payment(
        &owned_objects,
        "0xa",
        1_000_000_000,
        100_000,
        1,
        &HashSet::new(),
        &HashSet::new(),
    )
    .unwrap();

    assert_eq!(transfer.object_id, "0xlarge");
    assert_eq!(gas.payment_objects[0].object_id, "0xsmall");
}

#[test]
fn native_transfer_selection_skips_pending_object_refs() {
    use kanari_rpc_api::ObjectInfo;
    use kanari_types::transaction::ObjectOwnerKind;

    let owned_objects = ["0x1", "0x2", "0x3", "0x4"]
        .into_iter()
        .map(|id| ObjectInfo {
            id: id.to_string(),
            owner: "0xa".to_string(),
            owner_kind: ObjectOwnerKind::AddressOwner("0xa".to_string()),
            type_: CoinModule::coin_type(GAS_COIN),
            data: {
                let mut bytes = vec![0u8; 40];
                bytes[32..40].copy_from_slice(&100u64.to_le_bytes());
                bytes
            },
            version: 1,
            digest: Some(format!("{id}:digest")),
        })
        .collect::<Vec<_>>();

    let pending_access_keys =
        HashSet::from(["mut:object:0x1".to_string(), "mut:gas:0x2".to_string()]);
    let (coin, gas) = select_native_transfer_and_gas_payment(
        &owned_objects,
        "0xa",
        60,
        10,
        1,
        &pending_access_keys,
        &HashSet::new(),
    )
    .unwrap();

    assert_ne!(coin.object_id, "0x1");
    assert_ne!(gas.payment_objects[0].object_id, "0x2");
    assert_ne!(coin.object_id, gas.payment_objects[0].object_id);
}

#[test]
fn rejects_native_transfer_when_only_one_coin_would_overlap_gas() {
    use kanari_rpc_api::ObjectInfo;
    use kanari_types::transaction::ObjectOwnerKind;

    let owned_objects = vec![ObjectInfo {
        id: "0x1".to_string(),
        owner: "0xa".to_string(),
        owner_kind: ObjectOwnerKind::AddressOwner("0xa".to_string()),
        type_: CoinModule::coin_type(GAS_COIN),
        data: {
            let mut bytes = vec![0u8; 40];
            bytes[32..40].copy_from_slice(&100u64.to_le_bytes());
            bytes
        },
        version: 1,
        digest: Some("d1".to_string()),
    }];

    let pending_access_keys = HashSet::new();
    let err = select_native_transfer_and_gas_payment(
        &owned_objects,
        "0xa",
        60,
        10,
        1,
        &pending_access_keys,
        &HashSet::new(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("two distinct Coin<"));
}

#[test]
fn native_transfer_pair_becomes_available_after_pending_refs_commit() {
    use kanari_rpc_api::ObjectInfo;
    use kanari_types::transaction::ObjectOwnerKind;

    let owned_objects = [("0x1", 1_000u64), ("0x2", 100u64), ("0x3", 50u64)]
        .into_iter()
        .map(|(id, balance)| ObjectInfo {
            id: id.to_string(),
            owner: "0xa".to_string(),
            owner_kind: ObjectOwnerKind::AddressOwner("0xa".to_string()),
            type_: CoinModule::coin_type(GAS_COIN),
            data: {
                let mut bytes = vec![0u8; 40];
                bytes[32..40].copy_from_slice(&balance.to_le_bytes());
                bytes
            },
            version: 1,
            digest: Some(format!("{id}:digest")),
        })
        .collect::<Vec<_>>();

    let pending_access_keys =
        HashSet::from(["mut:object:0x1".to_string(), "mut:gas:0x2".to_string()]);
    let pending_error = select_native_transfer_and_gas_payment(
        &owned_objects,
        "0xa",
        10,
        1,
        1,
        &pending_access_keys,
        &HashSet::new(),
    )
    .unwrap_err();
    assert!(pending_error.to_string().contains("two distinct Coin<"));

    let (transfer, gas) = select_native_transfer_and_gas_payment(
        &owned_objects,
        "0xa",
        10,
        1,
        1,
        &HashSet::new(),
        &HashSet::new(),
    )
    .unwrap();
    assert_ne!(transfer.object_id, gas.payment_objects[0].object_id);
}

#[test]
fn native_transfer_selection_skips_client_excluded_object_refs() {
    use kanari_rpc_api::ObjectInfo;
    use kanari_types::transaction::ObjectOwnerKind;

    let owned_objects = ["0x1", "0x2", "0x3", "0x4"]
        .into_iter()
        .map(|id| ObjectInfo {
            id: id.to_string(),
            owner: "0xa".to_string(),
            owner_kind: ObjectOwnerKind::AddressOwner("0xa".to_string()),
            type_: CoinModule::coin_type(GAS_COIN),
            data: {
                let mut bytes = vec![0u8; 40];
                bytes[32..40].copy_from_slice(&100u64.to_le_bytes());
                bytes
            },
            version: 1,
            digest: Some(format!("{id}:digest")),
        })
        .collect::<Vec<_>>();

    let excluded_object_ids = HashSet::from(["0x1".to_string(), "0x2".to_string()]);
    let (coin, gas) = select_native_transfer_and_gas_payment(
        &owned_objects,
        "0xa",
        60,
        10,
        1,
        &HashSet::new(),
        &excluded_object_ids,
    )
    .unwrap();

    assert_ne!(coin.object_id, "0x1");
    assert_ne!(coin.object_id, "0x2");
    assert_ne!(gas.payment_objects[0].object_id, "0x1");
    assert_ne!(gas.payment_objects[0].object_id, "0x2");
    assert_ne!(coin.object_id, gas.payment_objects[0].object_id);
}

#[test]
fn selects_native_coin_consolidation_step_with_reserved_gas_coin() {
    use kanari_rpc_api::ObjectInfo;
    use kanari_types::transaction::ObjectOwnerKind;

    let owned_objects = vec![
        ObjectInfo {
            id: "0x1".to_string(),
            owner: "0xa".to_string(),
            owner_kind: ObjectOwnerKind::AddressOwner("0xa".to_string()),
            type_: CoinModule::coin_type(GAS_COIN),
            data: {
                let mut bytes = vec![0u8; 40];
                bytes[32..40].copy_from_slice(&120u64.to_le_bytes());
                bytes
            },
            version: 1,
            digest: Some("d1".to_string()),
        },
        ObjectInfo {
            id: "0x2".to_string(),
            owner: "0xa".to_string(),
            owner_kind: ObjectOwnerKind::AddressOwner("0xa".to_string()),
            type_: CoinModule::coin_type(GAS_COIN),
            data: {
                let mut bytes = vec![0u8; 40];
                bytes[32..40].copy_from_slice(&90u64.to_le_bytes());
                bytes
            },
            version: 1,
            digest: Some("d2".to_string()),
        },
        ObjectInfo {
            id: "0x3".to_string(),
            owner: "0xa".to_string(),
            owner_kind: ObjectOwnerKind::AddressOwner("0xa".to_string()),
            type_: CoinModule::coin_type(GAS_COIN),
            data: {
                let mut bytes = vec![0u8; 40];
                bytes[32..40].copy_from_slice(&20u64.to_le_bytes());
                bytes
            },
            version: 1,
            digest: Some("d3".to_string()),
        },
    ];

    let (primary, merge, gas) =
        select_native_coin_consolidation_step(&owned_objects, "0xa", 180, 10, 1).unwrap();
    assert_eq!(gas.payment_objects[0].object_id, "0x3");
    assert_eq!(primary.id, "0x1");
    assert_eq!(merge.id, "0x2");
}

#[test]
fn fresh_nonce_honors_client_value_and_watermark_floor() {
    // Explicit client nonces pass through (zero is rejected).
    assert_eq!(fresh_nonce(Some(42), None).unwrap(), 42);
    assert_eq!(fresh_nonce(Some(42), Some(5000)).unwrap(), 42);
    assert!(fresh_nonce(Some(0), None).is_err());

    // Without a watermark (fresh sender) fall back to floor 1, which the
    // engine accepts since the per-sender watermark defaults to 0.
    assert!(
        fresh_nonce(None, None).unwrap() >= 1,
        "must generate nonce from default floor without watermark"
    );

    // Simulated post-restart state: the global counter starts at 1 but the
    // sender's watermark floor is high. Generated nonces must clear the floor
    // immediately instead of producing thousands of stale rejections, and
    // stay strictly increasing afterwards.
    let first = fresh_nonce(None, Some(5000)).unwrap();
    assert!(first >= 5000, "floor not honored: {first}");
    let second = fresh_nonce(None, Some(5000)).unwrap();
    assert!(second > first, "nonces must be strictly increasing");
}

fn coin_change(
    object_id: &str,
    owner: &str,
    token_type: &str,
) -> kanari_types::transaction::ObjectChange {
    kanari_types::transaction::ObjectChange {
        change_type: kanari_types::transaction::ObjectChangeKind::Transferred,
        object_ref: kanari_types::transaction::ObjectRef::new(
            object_id.to_string(),
            Some(2),
            Some("0xdigest".to_string()),
        ),
        previous_object_ref: None,
        type_: Some(format!("0x2::coin::Coin<{token_type}>")),
        owner: Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
            owner.to_string(),
        )),
        previous_owner: None,
        previous_version: Some(1),
        amount: None,
    }
}

fn empty_success_effect() -> kanari_types::transaction::TransactionEffects {
    kanari_types::transaction::TransactionEffects {
        status: "success".to_string(),
        gas_used: 10,
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
    }
}

#[test]
fn transfers_from_effect_collects_every_coin_movement() {
    let mut effect = empty_success_effect();
    effect.transferred = vec![
        coin_change("0xaaa", "0xbbb", "0x1::james::JAMES"),
        coin_change("0xccc", "0xddd", "0x2::kanari::KANARI"),
    ];
    // Same coin repeated in another bucket must not duplicate.
    effect.object_changes = vec![coin_change("0xaaa", "0xbbb", "0x1::james::JAMES")];

    let transfers = transfers_from_effect(None, &effect);
    assert_eq!(transfers.len(), 2);
    assert_eq!(
        transfers[0].transfer_token_type.as_deref(),
        Some("0x1::james::JAMES")
    );
    assert_eq!(
        transfers[1].transfer_token_type.as_deref(),
        Some("0x2::kanari::KANARI")
    );
}

#[test]
fn effect_carried_amount_flows_into_transfer() {
    let mut effect = empty_success_effect();
    let mut change = coin_change("0xaaa", "0xbbb", "0x1::james::JAMES");
    // 100 THB in base units at decimals 6, recorded post-move on the coin.
    change.amount = Some(100_000_000);
    effect.transferred = vec![change];
    let transfers = transfers_from_effect(None, &effect);
    assert_eq!(transfers.len(), 1);
    assert_eq!(transfers[0].transfer_amount, Some(100_000_000));
    assert_eq!(
        transfers[0].transfer_token_type.as_deref(),
        Some("0x1::james::JAMES")
    );
}

#[test]
fn enrich_merges_arg_amount_with_effect_owner_by_coin_id() {
    let mut details = base_transaction_details(
        "0xhash".to_string(),
        "pending".to_string(),
        None,
        "transfer",
        "0x1".to_string(),
        "0x1".to_string(),
        1,
        100_000,
        1,
    );
    push_transfer_entry(
        &mut details,
        kanari_rpc_api::TransferEntry {
            recipient: Some("0xbbb".to_string()),
            transfer_amount: Some(77),
            transfer_token_type: None,
            transfer_decimals: None,
            previous_owner: None,
            coin_object_id: Some("0xaaa".to_string()),
        },
    );

    let mut effect = empty_success_effect();
    effect.transferred = vec![coin_change("0xaaa", "0xbbb", "0x1::james::JAMES")];
    enrich_transfer_from_effect(None, &mut details, &effect);

    // Merged into one entry, not appended as a second.
    let transfers = details.transfers.as_ref().unwrap();
    assert_eq!(transfers.len(), 1);
    assert_eq!(transfers[0].recipient.as_deref(), Some("0xbbb"));
    assert_eq!(transfers[0].transfer_amount, Some(77));
    assert_eq!(
        transfers[0].transfer_token_type.as_deref(),
        Some("0x1::james::JAMES")
    );
}

#[test]
fn effect_only_sender_change_back_is_filtered_as_noise() {
    let mut details = base_transaction_details(
        "0xhash".to_string(),
        "pending".to_string(),
        None,
        "transfer",
        "0xsender".to_string(),
        "0xsender".to_string(),
        1,
        100_000,
        1,
    );
    let mut effect = empty_success_effect();
    let mut noise = coin_change("0xaaa", "0xsender", "0x1::james::JAMES");
    noise.previous_owner = Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
        "0xsender".to_string(),
    ));
    effect.mutated = vec![noise];
    enrich_transfer_from_effect(None, &mut details, &effect);
    assert!(
        details.transfers.is_none() || details.transfers.as_ref().unwrap().is_empty(),
        "sender change-back without amount must not appear as a transfer"
    );
}

#[test]
fn arg_backed_self_transfer_survives_noise_filter() {
    let mut details = base_transaction_details(
        "0xhash".to_string(),
        "pending".to_string(),
        None,
        "transfer",
        "0xsender".to_string(),
        "0xsender".to_string(),
        1,
        100_000,
        1,
    );
    push_transfer_entry(
        &mut details,
        kanari_rpc_api::TransferEntry {
            recipient: Some("0xsender".to_string()),
            transfer_amount: Some(77),
            transfer_token_type: Some("0x1::james::JAMES".to_string()),
            transfer_decimals: None,
            previous_owner: None,
            coin_object_id: Some("0xaaa".to_string()),
        },
    );
    let mut effect = empty_success_effect();
    let mut noise = coin_change("0xaaa", "0xsender", "0x1::james::JAMES");
    noise.previous_owner = Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
        "0xsender".to_string(),
    ));
    effect.mutated = vec![noise];
    enrich_transfer_from_effect(None, &mut details, &effect);
    let transfers = details.transfers.as_ref().unwrap();
    assert_eq!(transfers.len(), 1);
    assert_eq!(transfers[0].transfer_amount, Some(77));
}

/// No-fallback contract: without state, only the KANARI protocol constant
/// resolves; every other token is unknown (None), never an invented 9/6/0.
#[test]
fn lookup_token_decimals_never_invents_fallback() {
    assert_eq!(lookup_token_decimals(None, Some(GAS_COIN)), Some(9));
    assert_eq!(lookup_token_decimals(None, Some("0xabc::thb::THB")), None);
    assert_eq!(lookup_token_decimals(None, None), None);
}

#[test]
fn typeless_arg_entry_backfills_from_matching_effect_entry() {
    // Committed entry-function calls (e.g. token::transfer_amount) carry the
    // coin in object inputs: the source object is consumed, so the arg entry
    // has (recipient, amount) but no token type, while the split coin in
    // effects has the type under a different object id.
    let mut details = base_transaction_details(
        "0xhash".to_string(),
        "pending".to_string(),
        None,
        "transfer",
        "0xsender".to_string(),
        "0xsender".to_string(),
        1,
        100_000,
        1,
    );
    push_transfer_entry(
        &mut details,
        kanari_rpc_api::TransferEntry {
            recipient: Some("0xbbb".to_string()),
            transfer_amount: Some(500_000),
            transfer_token_type: None,
            transfer_decimals: None,
            previous_owner: None,
            coin_object_id: Some("0xsource".to_string()),
        },
    );
    let mut effect = empty_success_effect();
    let mut split = coin_change("0xsplit", "0xbbb", "0xabc::thb::THB");
    split.amount = Some(500_000);
    effect.created = vec![split];
    enrich_transfer_from_effect(None, &mut details, &effect);

    let transfers = details.transfers.as_ref().unwrap();
    let arg_entry = transfers
        .iter()
        .find(|t| t.coin_object_id.as_deref() == Some("0xsource"))
        .expect("arg entry survives");
    assert_eq!(
        arg_entry.transfer_token_type.as_deref(),
        Some("0xabc::thb::THB")
    );
}

#[test]
fn ambiguous_backfill_never_mislabels_funds() {
    let mut details = base_transaction_details(
        "0xhash".to_string(),
        "pending".to_string(),
        None,
        "transfer",
        "0xsender".to_string(),
        "0xsender".to_string(),
        1,
        100_000,
        1,
    );
    push_transfer_entry(
        &mut details,
        kanari_rpc_api::TransferEntry {
            recipient: Some("0xbbb".to_string()),
            transfer_amount: Some(500_000),
            transfer_token_type: None,
            transfer_decimals: None,
            previous_owner: None,
            coin_object_id: Some("0xsource".to_string()),
        },
    );
    let mut effect = empty_success_effect();
    let mut first = coin_change("0xsplit1", "0xbbb", "0xabc::thb::THB");
    first.amount = Some(500_000);
    let mut second = coin_change("0xsplit2", "0xbbb", "0xabc::other::OTH");
    second.amount = Some(500_000);
    effect.created = vec![first, second];
    enrich_transfer_from_effect(None, &mut details, &effect);

    let transfers = details.transfers.as_ref().unwrap();
    let arg_entry = transfers
        .iter()
        .find(|t| t.coin_object_id.as_deref() == Some("0xsource"))
        .expect("arg entry survives");
    assert_eq!(arg_entry.transfer_token_type, None);
}

#[test]
fn multiple_transfers_are_all_preserved() {
    let mut details = base_transaction_details(
        "0xhash".to_string(),
        "pending".to_string(),
        None,
        "transfer",
        "0x1".to_string(),
        "0x1".to_string(),
        1,
        100_000,
        1,
    );
    push_transfer_entry(
        &mut details,
        kanari_rpc_api::TransferEntry {
            recipient: Some("0xbbb".to_string()),
            transfer_amount: Some(5),
            transfer_token_type: Some("0x1::james::JAMES".to_string()),
            transfer_decimals: None,
            previous_owner: None,
            coin_object_id: None,
        },
    );
    push_transfer_entry(
        &mut details,
        kanari_rpc_api::TransferEntry {
            recipient: Some("0xddd".to_string()),
            transfer_amount: Some(6),
            transfer_token_type: Some("0x2::kanari::KANARI".to_string()),
            transfer_decimals: Some(9),
            previous_owner: None,
            coin_object_id: None,
        },
    );
    let transfers = details.transfers.as_ref().unwrap();
    assert_eq!(transfers.len(), 2);
    assert_eq!(transfers[0].recipient.as_deref(), Some("0xbbb"));
    assert_eq!(transfers[0].transfer_amount, Some(5));
    assert_eq!(transfers[1].recipient.as_deref(), Some("0xddd"));
    assert_eq!(transfers[1].transfer_amount, Some(6));
}

#[test]
fn concurrent_nonce_generation_stays_unique_above_floor() {
    use std::collections::HashSet;
    let floor = 9000u64;
    let handles: Vec<_> = (0..8)
        .map(|_| std::thread::spawn(move || fresh_nonce(None, Some(floor)).unwrap()))
        .collect();
    let mut seen = HashSet::new();
    for h in handles {
        let n = h.join().expect("nonce thread panicked");
        assert!(n >= floor, "floor not honored under concurrency: {n}");
        assert!(
            seen.insert(n),
            "duplicate nonce generated concurrently: {n}"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn transfers_preserve_every_entry_in_order(
        n in 1usize..5,
        amount in 1u64..1_000_000,
    ) {
        let mut details = base_transaction_details(
            "0xhash".to_string(),
            "pending".to_string(),
            None,
            "transfer",
            "0x1".to_string(),
            "0x1".to_string(),
            1,
            100_000,
            1,
        );
        for i in 0..n {
            push_transfer_entry(
                &mut details,
                kanari_rpc_api::TransferEntry {
                    recipient: Some(format!("0x{i}")),
                    transfer_amount: Some(amount + i as u64),
                    transfer_token_type: Some("0x1::james::JAMES".to_string()),
                    transfer_decimals: None,
                    previous_owner: None,
                    coin_object_id: Some(format!("0xcoin{i}")),
                },
            );
        }
        let transfers = details.transfers.as_ref().unwrap();
        prop_assert_eq!(transfers.len(), n);
        prop_assert_eq!(transfers[0].recipient.as_deref(), Some("0x0"));
        prop_assert_eq!(transfers[0].transfer_amount, Some(amount));
    }
}
