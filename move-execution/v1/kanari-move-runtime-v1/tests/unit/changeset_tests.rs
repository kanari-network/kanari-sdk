use kanari_types::address::Address as KanariAddress;
use kanari_types::error::KanariUnwrapExt;
use kanari_types::transaction::{
    GasPayment, ObjectChangeKind, ObjectGraphEdgeKind, ObjectInput, ObjectOwnerKind, ObjectRef,
};

use super::*;

#[test]
fn test_changeset_transfer() {
    let mut cs = ChangeSet::new();
    let from = AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS)
        .invariant("valid standard address");
    let to = AccountAddress::from_hex_literal(KanariAddress::KANARI_SYSTEM_ADDRESS)
        .invariant("valid system address");

    cs.transfer(from, to, 100);

    assert_eq!(cs.owner_deltas.len(), 2);
    assert_eq!(
        cs.owner_deltas
            .get(&from)
            .invariant("sender change missing")
            .balance_delta,
        -100
    );
    assert_eq!(
        cs.owner_deltas
            .get(&to)
            .invariant("recipient change missing")
            .balance_delta,
        100
    );
}

#[test]
fn test_changeset_mint() {
    let mut cs = ChangeSet::new();
    let to = AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS)
        .invariant("valid standard address");

    cs.mint(to, 1000);

    assert_eq!(cs.owner_deltas.len(), 1);
    assert_eq!(
        cs.owner_deltas
            .get(&to)
            .invariant("recipient change missing")
            .balance_delta,
        1000
    );
}

#[test]
fn test_changeset_burn() {
    let mut cs = ChangeSet::new();
    let from = AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS)
        .invariant("valid standard address");

    cs.burn(from, 500);

    assert_eq!(cs.owner_deltas.len(), 1);
    assert_eq!(
        cs.owner_deltas
            .get(&from)
            .invariant("sender change missing")
            .balance_delta,
        -500
    );
}

#[test]
fn test_changeset_module_publish() {
    let mut cs = ChangeSet::new();
    let publisher = AccountAddress::from_hex_literal(KanariAddress::KANARI_SYSTEM_ADDRESS)
        .invariant("valid system address");

    cs.publish_module(publisher, "kanari".to_string());

    let change = cs
        .owner_deltas
        .get(&publisher)
        .invariant("publisher change missing");
    assert_eq!(change.modules_added.len(), 1);
    assert!(change.modules_added.contains("kanari"));
}

#[test]
fn effects_bucket_object_changes_and_preserve_input_refs() {
    let owner = AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS)
        .invariant("valid standard address");
    let mut cs = ChangeSet::new();
    cs.set_transaction_context(
        vec![
            ObjectInput {
                object_ref: ObjectRef::new("0x1", Some(7), Some("0xaaa".to_string())),
                owner: Some(ObjectOwnerKind::AddressOwner(owner.to_hex_literal())),
                mutable: true,
            },
            ObjectInput {
                object_ref: ObjectRef::new("0x2", Some(3), Some("0xbbb".to_string())),
                owner: Some(ObjectOwnerKind::Shared),
                mutable: false,
            },
            ObjectInput {
                object_ref: ObjectRef::new("0x3", Some(9), Some("0xccc".to_string())),
                owner: Some(ObjectOwnerKind::Immutable),
                mutable: false,
            },
        ],
        Some(GasPayment {
            payment_objects: vec![ObjectRef::new("0xgas", Some(1), Some("0xddd".to_string()))],
            owner: owner.to_hex_literal(),
            budget: 100,
            price: 1,
        }),
    );
    cs.set_explicit_object_changes(vec![
        ObjectChange {
            change_type: ObjectChangeKind::Created,
            object_ref: ObjectRef::new("0xc", Some(1), Some("0x1".to_string())),
            previous_object_ref: None,
            type_: Some("0x2::test::Created".to_string()),
            owner: Some(ObjectOwnerKind::AddressOwner(owner.to_hex_literal())),
            previous_owner: None,
            previous_version: None,
        },
        ObjectChange {
            change_type: ObjectChangeKind::Transferred,
            object_ref: ObjectRef::new("0xt", Some(2), Some("0x2".to_string())),
            previous_object_ref: Some(ObjectRef::new("0xt", Some(1), Some("0xold".to_string()))),
            type_: Some("0x2::test::Transferred".to_string()),
            owner: Some(ObjectOwnerKind::AddressOwner(owner.to_hex_literal())),
            previous_owner: Some(ObjectOwnerKind::Shared),
            previous_version: Some(1),
        },
    ]);

    let effects = cs.effects(None);
    assert_eq!(effects.input_objects.len(), 3);
    assert_eq!(effects.shared_inputs.len(), 1);
    assert_eq!(effects.immutable_inputs.len(), 1);
    assert_eq!(effects.gas_object_refs.len(), 1);
    assert_eq!(effects.created.len(), 1);
    assert_eq!(effects.transferred.len(), 1);
    assert!(
        effects
            .causal_edges
            .iter()
            .any(|edge| matches!(edge.relation, ObjectGraphEdgeKind::VersionSuccessor))
    );
    assert!(
        effects
            .causal_edges
            .iter()
            .any(|edge| matches!(edge.relation, ObjectGraphEdgeKind::GasCreate))
    );
    assert!(
        effects
            .causal_edges
            .iter()
            .any(|edge| matches!(edge.relation, ObjectGraphEdgeKind::OwnershipTransfer))
    );
    assert_eq!(
        effects.transferred[0]
            .previous_object_ref
            .as_ref()
            .invariant("previous ref should exist")
            .version,
        Some(1)
    );
}
