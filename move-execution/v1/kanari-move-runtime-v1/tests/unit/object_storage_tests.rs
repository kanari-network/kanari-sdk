use super::*;
use kanari_types::transaction::ObjectOwnerKind;

fn address_owner(owner: AccountAddress) -> ObjectOwnerKind {
    ObjectOwnerKind::AddressOwner(owner.to_hex_literal())
}

#[test]
fn persistent_owner_lookup_prefers_canonical_owned_objects_index() -> Result<()> {
    let store = Arc::new(PersistentStore::open_in_memory()?);
    let owner = AccountAddress::from_hex_literal("0x1")?;
    let stale_id = "0xaaaa".to_string();
    let canonical_id = "0xbbbb".to_string();

    store.save(
        format!("object:{}", stale_id).as_bytes(),
        &StoredObject {
            id: stale_id.clone(),
            owner,
            owner_kind: address_owner(owner),
            type_name: "0x2::coin::Coin<0x2::kanari::KANARI>".to_string(),
            data: vec![1],
            version: 1,
        },
    )?;
    store.save(
        format!("object:{}", canonical_id).as_bytes(),
        &StoredObject {
            id: canonical_id.clone(),
            owner,
            owner_kind: address_owner(owner),
            type_name: "0x2::coin::Coin<0x2::kanari::KANARI>".to_string(),
            data: vec![2],
            version: 1,
        },
    )?;
    store.save(
        &ObjectStorage::legacy_owner_key(&owner),
        &vec![stale_id.clone()],
    )?;
    store.save(&owned_objects_key(&owner), &vec![canonical_id.clone()])?;

    let storage = ObjectStorage::new_with_store(store.clone())?;
    let objects = storage.get_objects_by_owner(&owner);

    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].id, canonical_id);
    assert_eq!(
        ObjectStorage::load_id_index(&store, &owned_objects_key(&owner))?,
        vec![canonical_id]
    );

    Ok(())
}

#[test]
fn persistent_startup_rejects_object_index_entry_without_object() -> Result<()> {
    let store = Arc::new(PersistentStore::open_in_memory()?);
    store.save(
        ObjectStorage::OBJECT_INDEX_KEY.as_bytes(),
        &vec!["0xmissing".to_string()],
    )?;

    let err = ObjectStorage::new_with_store(store)
        .err()
        .expect("partial object index must fail startup");
    assert!(
        err.to_string()
            .contains("object index references missing object")
    );
    Ok(())
}

#[test]
fn persistent_object_load_error_is_not_reported_as_missing() -> Result<()> {
    let store = Arc::new(PersistentStore::open_in_memory()?);
    store.apply_raw_changes(&[(b"object:0xcorrupt".to_vec(), vec![0x80])], &[])?;
    let storage = ObjectStorage::new_with_store(store)?;

    assert!(storage.get_object("0xcorrupt").is_err());
    Ok(())
}

#[test]
fn legacy_owner_index_is_migrated_to_owned_objects_index() -> Result<()> {
    let store = Arc::new(PersistentStore::open_in_memory()?);
    let owner = AccountAddress::from_hex_literal("0x2")?;
    let object_id = "0xcccc".to_string();

    store.save(
        format!("object:{}", object_id).as_bytes(),
        &StoredObject {
            id: object_id.clone(),
            owner,
            owner_kind: address_owner(owner),
            type_name: "0x2::coin::Coin<0x2::kanari::KANARI>".to_string(),
            data: vec![3],
            version: 1,
        },
    )?;
    store.save(
        &ObjectStorage::legacy_owner_key(&owner),
        &vec![object_id.clone()],
    )?;

    let storage = ObjectStorage::new_with_store(store.clone())?;
    let objects = storage.get_objects_by_owner(&owner);

    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].id, object_id);
    assert_eq!(
        ObjectStorage::load_id_index(&store, &owned_objects_key(&owner))?,
        vec!["0xcccc".to_string()]
    );
    assert!(
        store
            .load::<Vec<String>>(&ObjectStorage::legacy_owner_key(&owner))?
            .is_none()
    );

    Ok(())
}

#[test]
fn ownership_transfer_after_cache_clear_updates_both_owner_indexes() -> Result<()> {
    let store = Arc::new(PersistentStore::open_in_memory()?);
    let old_owner = AccountAddress::from_hex_literal("0x11")?;
    let new_owner = AccountAddress::from_hex_literal("0x22")?;
    let object_id = "0xdddd".to_string();
    let storage = ObjectStorage::new_with_store(store.clone())?;

    storage.store_object(StoredObject {
        id: object_id.clone(),
        owner: old_owner,
        owner_kind: address_owner(old_owner),
        type_name: "0x2::coin::Coin<0x2::kanari::KANARI>".to_string(),
        data: vec![4],
        version: 1,
    })?;

    // Checkpoint finalization clears the live cache. The next update must still
    // discover the persisted old owner before rewriting the canonical indexes.
    storage.clear()?;
    storage.store_object(StoredObject {
        id: object_id.clone(),
        owner: new_owner,
        owner_kind: address_owner(new_owner),
        type_name: "0x2::coin::Coin<0x2::kanari::KANARI>".to_string(),
        data: vec![5],
        version: 2,
    })?;

    assert!(ObjectStorage::load_id_index(&store, &owned_objects_key(&old_owner))?.is_empty());
    assert_eq!(
        ObjectStorage::load_id_index(&store, &owned_objects_key(&new_owner))?,
        vec![object_id]
    );

    Ok(())
}
