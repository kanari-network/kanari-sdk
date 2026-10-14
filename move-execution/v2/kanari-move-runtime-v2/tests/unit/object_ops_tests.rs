use kanari_types::error::KanariUnwrapExt;
use kanari_types::transaction::ObjectOwnerKind;
use move_core_types::account_address::AccountAddress;
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

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
        is_shared: false,
    };

    let baseline_count = runtime.object_storage.count();

    let mut speculative = ChangeSet::new();
    runtime
        .add_transferred_objects_to_changeset(
            &mut speculative,
            vec![object.clone()],
            false,
            None,
            &[],
            None,
        )
        .unwrap();

    assert_eq!(runtime.object_storage.count(), baseline_count);
    assert_eq!(speculative.created_objects.len(), 1);

    let mut canonical = ChangeSet::new();
    runtime
        .add_transferred_objects_to_changeset(&mut canonical, vec![object], true, None, &[], None)
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
        is_shared: false,
    };

    let mut changeset = ChangeSet::new();
    runtime
        .add_transferred_objects_to_changeset(
            &mut changeset,
            vec![transferred],
            false,
            Some(&overlay),
            &[],
            None,
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

#[test]
fn preload_resolves_zero_stripped_object_ids() {
    use kanari_system_natives::object::LoadedObjectsExt;

    let runtime = MoveRuntime::new_with_natives_in_memory(vec![]).invariant("runtime init");
    let owner = AccountAddress::from_hex_literal("0x1234").invariant("valid owner address");
    // Canonical id with leading zero bytes; its display form strips them.
    let canonical_id = "0x0070b2e77c3117590f6c18cc58a31bead3f3990742a5007f4c63cd4ae8ca90ed";
    runtime
        .preload_object_snapshot(canonical_id, owner, "0x2::test::Object", vec![0u8; 40], 1)
        .invariant("snapshot stored");

    // The stripped display form decodes to 31 bytes, not 32.
    let stripped = hex::decode("70b2e77c3117590f6c18cc58a31bead3f3990742a5007f4c63cd4ae8ca90ed")
        .invariant("decode stripped id");
    assert_eq!(stripped.len(), 31);

    let vm_guard = runtime.read_vm();
    let mut session = runtime.create_session_with_storage_ext(&vm_guard);
    runtime
        .preload_object_ids_from_args(&mut session, &[stripped], Some(owner), None)
        .invariant("preload succeeds");

    let loaded = session
        .get_native_extensions()
        .get_mut::<LoadedObjectsExt>();
    assert!(
        loaded.get(canonical_id).is_some(),
        "stripped id must resolve to the stored object"
    );
}

#[test]
fn freezing_existing_object_marks_it_immutable() {
    let runtime = MoveRuntime::new_with_natives_in_memory(vec![]).invariant("runtime init");
    let owner = AccountAddress::from_hex_literal("0x1234").invariant("valid owner address");
    let object_id = "0xabcd".to_string();
    let canonical_id = canonical_object_id(&object_id).invariant("canonical object id");

    // The object already exists in storage with an address owner.
    runtime
        .object_storage
        .store_object(StoredObject {
            id: canonical_id.clone(),
            owner,
            owner_kind: ObjectOwnerKind::AddressOwner(owner.to_hex_literal()),
            type_name: "0x2::test::Object".to_string(),
            data: vec![1, 2, 3],
            version: 1,
        })
        .invariant("seed existing object");

    // `transfer::public_freeze_object` re-emits the same object as frozen.
    let frozen = TransferredObject {
        object_id: object_id.clone(),
        object_type: "0x2::test::Object".to_string(),
        recipient: AccountAddress::ZERO,
        data: vec![1, 2, 3],
        should_persist: true,
        is_frozen: true,
        is_shared: false,
    };

    let mut changeset = ChangeSet::new();
    runtime
        .add_transferred_objects_to_changeset(&mut changeset, vec![frozen], true, None, &[], None)
        .invariant("freeze existing object");

    let (_, created) = changeset
        .created_objects
        .first()
        .invariant("created object");
    assert_eq!(
        created.owner_kind,
        ObjectOwnerKind::Immutable,
        "freezing an existing object must persist it as Immutable"
    );
}

#[test]
fn sharing_existing_object_persists_it_as_shared() {
    let runtime = MoveRuntime::new_with_natives_in_memory(vec![]).invariant("runtime init");
    let owner = AccountAddress::from_hex_literal("0x1234").invariant("valid owner address");
    let object_id = "0xdcba".to_string();
    let canonical_id = canonical_object_id(&object_id).invariant("canonical object id");

    runtime
        .object_storage
        .store_object(StoredObject {
            id: canonical_id.clone(),
            owner,
            owner_kind: ObjectOwnerKind::AddressOwner(owner.to_hex_literal()),
            type_name: "0x2::test::Object".to_string(),
            data: vec![1, 2, 3],
            version: 1,
        })
        .invariant("seed existing object");

    let shared = TransferredObject {
        object_id: object_id.clone(),
        object_type: "0x2::test::Object".to_string(),
        recipient: AccountAddress::ZERO,
        data: vec![1, 2, 3],
        should_persist: true,
        is_frozen: false,
        is_shared: true,
    };

    let mut changeset = ChangeSet::new();
    runtime
        .add_transferred_objects_to_changeset(&mut changeset, vec![shared], true, None, &[], None)
        .invariant("share existing object");

    let (_, created) = changeset
        .created_objects
        .first()
        .invariant("created object");
    assert_eq!(
        created.owner_kind,
        ObjectOwnerKind::Shared,
        "sharing an existing object must persist it as Shared"
    );
}

#[test]
fn saved_unknown_id_does_not_fabricate_zero_owned_ghost() {
    use kanari_system_natives::object::SavedObject;

    let runtime = MoveRuntime::new_with_natives_in_memory(vec![]).invariant("runtime init");
    let baseline_count = runtime.object_storage.count();

    // An embedded value's UID (e.g. a coin nested in a wallet): never a
    // declared input and never stored standalone. Saving it must record
    // nothing — previously this fabricated a ZERO-owned ghost duplicate.
    let ghost = SavedObject {
        object_id: format!("0x{:0>64}", "99"),
        object_type: "0x2::coin::Coin<0x2::test::MOCK>".to_string(),
        data: vec![0u8; 40],
    };
    let mut changeset = ChangeSet::new();
    let mut processed_ids = HashSet::new();
    runtime
        .upsert_saved_and_borrowed_objects(
            &mut changeset,
            &[],
            vec![ghost],
            vec![],
            "saved",
            "borrowed",
            &mut processed_ids,
        )
        .unwrap();

    assert!(
        changeset.created_objects.is_empty(),
        "unknown embedded id must not create a stored object"
    );
    assert_eq!(runtime.object_storage.count(), baseline_count);
}
