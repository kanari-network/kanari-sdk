use super::{
    apply_committed_effect, base_transaction_details, classify_transaction_error_data,
    derive_transaction_state_flags, select_native_coin_consolidation_step,
    select_native_transfer_and_gas_payment, transaction_error_with_reason,
};
use kanari_move_runtime_v1::changeset::ChangeSet;
use kanari_rpc_api::TransactionErrorReason;
use kanari_types::coin::CoinModule;
use kanari_types::gas_coin::GAS_COIN;
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

    apply_committed_effect(&mut details, Some(&effect));

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
    )
    .unwrap_err();
    assert!(pending_error.to_string().contains("two distinct Coin<"));

    let (transfer, gas) =
        select_native_transfer_and_gas_payment(&owned_objects, "0xa", 10, 1, 1, &HashSet::new())
            .unwrap();
    assert_ne!(transfer.object_id, gas.payment_objects[0].object_id);
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
