use super::MoveVMState;
use crate::storage::object_storage::StoredObject;
use anyhow::Result;
use kanari_types::transaction::ObjectOwnerKind;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::StructTag;
use std::str::FromStr;
use std::{collections::BTreeMap, sync::Arc};

#[test]
fn delete_resource_removes_saved_value() -> Result<()> {
    let state = MoveVMState::new_in_memory()?;
    let owner = AccountAddress::from_hex_literal("0x1234")?;
    let tag = StructTag::from_str("0x2::coin::Coin<0x2::kanari::KANARI>")?;
    let bytes = vec![1u8, 2, 3, 4];

    state.save_resource(&owner, &tag, &bytes)?;
    assert_eq!(state.get_resource(&owner, &tag), Some(bytes));

    state.delete_resource(&owner, &tag)?;
    assert_eq!(state.get_resource(&owner, &tag), None);
    Ok(())
}

#[test]
fn canonical_stored_object_is_available_to_resource_fallback_and_coin_mirror() -> Result<()> {
    let state = MoveVMState::new_in_memory()?;
    let owner = AccountAddress::from_hex_literal("0x1234")?;
    let key = format!("object:{}", owner.to_hex_literal());
    let object = StoredObject {
        id: owner.to_hex_literal(),
        owner,
        owner_kind: ObjectOwnerKind::AddressOwner(owner.to_hex_literal()),
        type_name: "0x2::coin::Coin<0x2::kanari::KANARI>".to_string(),
        data: vec![1, 2, 3],
        version: 1,
    };
    state.store.save(key.as_bytes(), &object)?;

    assert_eq!(state.try_get_object(&owner)?, Some(vec![1, 2, 3]));

    let tag = StructTag::from_str("0x2::coin::Coin<0x2::kanari::KANARI>")?;
    state.save_resource(&owner, &tag, &[9, 8, 7])?;
    assert_eq!(state.try_get_object(&owner)?, Some(vec![9, 8, 7]));
    Ok(())
}

#[test]
fn corrupted_module_bytes_are_reported_as_storage_error() -> Result<()> {
    let state = MoveVMState::new_in_memory()?;
    let module_id = move_core_types::language_storage::ModuleId::new(
        AccountAddress::from_hex_literal("0x42")?,
        move_core_types::identifier::Identifier::new("broken")?,
    );
    let key = format!(
        "module:{}:{}",
        module_id.address().to_hex_literal(),
        module_id.name()
    );
    state
        .store
        .apply_raw_changes(&[(key.into_bytes(), vec![0x80])], &[])?;

    assert!(state.try_get_module(&module_id).is_err());
    Ok(())
}

#[test]
fn overlay_object_update_and_delete_shadow_persistent_state() -> Result<()> {
    let state = MoveVMState::new_in_memory()?;
    let object_id = AccountAddress::from_hex_literal("0x1234")?;
    let key = format!("object:{}", object_id.to_hex_literal()).into_bytes();
    let mut object = StoredObject {
        id: object_id.to_hex_literal(),
        owner: object_id,
        owner_kind: ObjectOwnerKind::AddressOwner(object_id.to_hex_literal()),
        type_name: "0x2::test::Value".to_string(),
        data: vec![1],
        version: 1,
    };
    state.store.save(&key, &object)?;

    object.data = vec![2];
    object.version = 2;
    let updated = state.with_overlay(Some(Arc::new(BTreeMap::from([(
        key.clone(),
        Some(bcs::to_bytes(&object)?),
    )]))));
    assert_eq!(updated.try_get_object(&object_id)?, Some(vec![2]));

    let deleted = state.with_overlay(Some(Arc::new(BTreeMap::from([(key, None)]))));
    assert_eq!(deleted.try_get_object(&object_id)?, None);
    assert_eq!(state.try_get_object(&object_id)?, Some(vec![1]));
    Ok(())
}
