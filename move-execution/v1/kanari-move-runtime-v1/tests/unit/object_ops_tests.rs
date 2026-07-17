use kanari_types::error::KanariUnwrapExt;
use kanari_types::transaction::ObjectOwnerKind;
use move_core_types::account_address::AccountAddress;
use std::{collections::BTreeMap, sync::Arc};

use super::*;
use crate::move_runtime::MoveRuntime;
use crate::storage::object_storage::StoredObject;

#[test]
fn speculative_transferred_objects_do_not_mutate_object_storage() {
    let runtime = MoveRuntime::new_with_natives_in_memory(vec![]).invariant("runtime init");
    let owner = AccountAddress::from_hex_literal("0x1234").invariant("valid owner address");
    let object_id = "0xabcd".to_string();
    let object_type = "0x2::test::Object".to_string();
    let object = TransferredObject {
        object_id: object_id.clone(),
        object_type: object_type.clone(),
        recipient: owner,
        data: vec![1, 2, 3],
        should_persist: true,
        is_frozen: false,
    };

    let baseline_count = runtime.object_storage.count();

    let mut speculative = ChangeSet::new();
    runtime
        .add_transferred_objects_to_changeset(&mut speculative, vec![object.clone()], false, None)
        .unwrap();

    assert_eq!(runtime.object_storage.count(), baseline_count);
    assert_eq!(speculative.created_objects.len(), 1);

    let mut canonical = ChangeSet::new();
    runtime
        .add_transferred_objects_to_changeset(&mut canonical, vec![object], true, None)
        .unwrap();

    assert_eq!(runtime.object_storage.count(), baseline_count + 1);
    let canonical_id = canonical_object_id(&object_id).invariant("canonical object id");
    assert!(
        runtime
            .object_storage
            .get_object(&canonical_id)
            .unwrap()
            .is_some()
    );
}

#[test]
fn transferred_object_version_and_owner_kind_are_read_from_speculative_overlay() {
    let runtime = MoveRuntime::new_with_natives_in_memory(vec![]).invariant("runtime init");
    let old_owner = AccountAddress::from_hex_literal("0x1111").invariant("old owner");
    let new_owner = AccountAddress::from_hex_literal("0x2222").invariant("new owner");
    let object_id = canonical_object_id("0xabcd").invariant("canonical object id");
    let existing = StoredObject {
        id: object_id.clone(),
        owner: old_owner,
        owner_kind: ObjectOwnerKind::Shared,
        type_name: "0x2::test::Object".to_string(),
        data: vec![1],
        version: 9,
    };
    let mut overlay = BTreeMap::new();
    overlay.insert(
        crate::common::keys::object_key(&object_id),
        Some(bcs::to_bytes(&existing).invariant("serialize overlay object")),
    );
    let overlay = Arc::new(overlay);
    let transferred = TransferredObject {
        object_id: object_id.clone(),
        object_type: existing.type_name,
        recipient: new_owner,
        data: vec![2],
        should_persist: true,
        is_frozen: false,
    };

    let mut changeset = ChangeSet::new();
    runtime
        .add_transferred_objects_to_changeset(
            &mut changeset,
            vec![transferred],
            false,
            Some(&overlay),
        )
        .unwrap();

    let (_, created) = changeset
        .created_objects
        .first()
        .invariant("created object");
    assert_eq!(created.version, 10);
    assert_eq!(created.owner_kind, ObjectOwnerKind::Shared);
    assert_eq!(created.owner, new_owner);
}
