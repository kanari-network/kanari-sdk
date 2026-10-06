// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Cryptographic key utilities.

use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::StructTag;

pub(crate) fn metadata_key(prefix: &[u8], suffix: &str) -> Vec<u8> {
    let mut key = prefix.to_vec();
    key.extend_from_slice(suffix.as_bytes());
    key
}

pub(crate) fn object_key(id: &str) -> Vec<u8> {
    let mut key = b"object:".to_vec();
    key.extend_from_slice(id.as_bytes());
    key
}

pub(crate) fn module_key(address: &AccountAddress, module_name: &str) -> String {
    format!("module:{}:{}", address.to_hex_literal(), module_name)
}

pub(crate) fn resource_key(address: &AccountAddress, tag: &StructTag) -> String {
    format!("resource:{}:{}", address.to_hex_literal(), tag)
}

pub(crate) fn owned_objects_key(owner: &AccountAddress) -> Vec<u8> {
    let mut key = b"owned_objects:".to_vec();
    key.extend_from_slice(owner.as_ref());
    key
}

pub(crate) fn dynamic_field_key(object_id: &str, name_bytes: &[u8]) -> Vec<u8> {
    let hash = kanari_crypto::hash_data_blake3(name_bytes);
    let mut key = b"df:".to_vec();
    key.extend_from_slice(object_id.as_bytes());
    key.extend_from_slice(b":");
    key.extend_from_slice(hex::encode(&hash[0..16]).as_bytes());
    key
}
