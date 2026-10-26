// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::account_address::AccountAddress;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use serde::{Deserialize, Serialize};

/// UID wrapper used by Move `object::UID` (contains an address)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct UIDRecord {
    pub addr: AccountAddress,
}

impl UIDRecord {
    /// Create a new UIDRecord from an AccountAddress
    pub fn new(addr: AccountAddress) -> Self {
        Self { addr }
    }

    /// Return the underlying address
    pub fn address(&self) -> AccountAddress {
        self.addr
    }

    /// Convenience: construct from hex literal string like "0x1"
    pub fn from_hex_literal(hex: &str) -> Result<Self> {
        let addr = AccountAddress::from_hex_literal(hex).context("invalid address")?;
        Ok(Self::new(addr))
    }
}

/// ID wrapper used by Move `object::ID` (contains an address)
/// Added to support DEX/DeFi features where copyable IDs are needed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct IDRecord {
    pub bytes: AccountAddress,
}

impl IDRecord {
    /// Create a new IDRecord from an AccountAddress
    pub fn new(bytes: AccountAddress) -> Self {
        Self { bytes }
    }

    /// Return the underlying address
    pub fn address(&self) -> AccountAddress {
        self.bytes
    }

    /// Convenience: construct from hex literal string like "0x1"
    pub fn from_hex_literal(hex: &str) -> Result<Self> {
        let bytes = AccountAddress::from_hex_literal(hex).context("invalid address")?;
        Ok(Self::new(bytes))
    }
}

/// Object module constants and utilities
pub struct ObjectModule;

/// Canonical full-form spelling of an address used as an object id:
/// `0x` + 64 lowercase hex digits.
pub fn object_id_for_address(address: &AccountAddress) -> String {
    format!("0x{}", hex::encode(address.as_ref()))
}

/// Canonicalize an object id string to the full 64-hex form.
///
/// This is the SDK's ONLY object-id conversion — the Move runtime (storage
/// keys, access keys, created-object registration) and the RPC boundary must
/// share it. Move's `to_hex_literal` strips leading zeros (`0x0139…` →
/// `0x139…`), and an `ObjectRef` hashes its `object_id` string verbatim, so
/// any layer that canonicalizes differently silently breaks transaction
/// signature verification.
///
/// Returns `None` when the input is not a valid hex-encoded Move address:
/// such a value never names an object. Read paths treat it as not-found;
/// write paths must never receive it (validated at the RPC boundary).
pub fn canonical_object_id(id: &str) -> Option<String> {
    let trimmed = id.trim();
    // Reject non-hex characters early so the error is caught here instead of
    // producing a misleading canonical form.
    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        let hex_part = &trimmed[2..];
        if hex_part.is_empty() || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
    } else if !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    AccountAddress::from_hex_literal(trimmed)
        .ok()
        .map(|address| object_id_for_address(&address))
}

/// Canonical object id from raw bytes (32-byte address prefix).
///
/// Returns `None` for short or all-zero byte sequences, which never name an
/// object.
pub fn object_id_from_bytes(bytes: &[u8]) -> Option<String> {
    if bytes.len() < AccountAddress::LENGTH {
        return None;
    }
    let mut arr = [0u8; AccountAddress::LENGTH];
    arr.copy_from_slice(&bytes[..AccountAddress::LENGTH]);
    if arr.iter().all(|byte| *byte == 0) {
        return None;
    }
    Some(object_id_for_address(&AccountAddress::new(arr)))
}

/// Canonical object id from exactly 32 address bytes (BCS `address` layout).
///
/// Unlike [`object_id_from_bytes`], this does not reject the all-zero address
/// — it is the pure byte→id encoding for values already known to be address
/// bytes (UID serialization, raw function arguments). Returns `None` when the
/// input is not exactly `AccountAddress::LENGTH` bytes.
pub fn object_id_from_address_bytes(bytes: &[u8]) -> Option<String> {
    AccountAddress::from_bytes(bytes)
        .ok()
        .map(|address| object_id_for_address(&address))
}

impl ObjectModule {
    pub const OBJECT_MODULE: &'static str = "object";

    /// Name of the UID struct in Move
    pub const UID_STRUCT: &'static str = "UID";

    /// Name of the ID struct in Move
    pub const ID_STRUCT: &'static str = "ID";

    /// Get the module ID for kanari_system::object
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name =
            Identifier::new(Self::OBJECT_MODULE).context("Invalid object module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names used in object module
    pub fn function_names() -> ObjectFunctions {
        ObjectFunctions {
            new: "new",
            uid_to_inner: "uid_to_inner",
            id_from_address: "id_from_address",
            id_to_address: "id_to_address",
            id_to_bytes: "id_to_bytes",
            uid_address: "uid_address",
            uid_to_u64: "uid_to_u64",
            uid_to_bytes: "uid_to_bytes",
            id_bytes: "id_bytes",
            save_object: "save_object",
            borrow_global_mut: "borrow_global_mut",
            borrow_global: "borrow_global",
            delete: "delete",
        }
    }
}

/// Object module function names
pub struct ObjectFunctions {
    pub new: &'static str,
    pub uid_to_inner: &'static str,
    pub id_from_address: &'static str,
    pub id_to_address: &'static str,
    pub id_to_bytes: &'static str,
    pub uid_address: &'static str,
    pub uid_to_u64: &'static str,
    pub uid_to_bytes: &'static str,
    pub id_bytes: &'static str,
    pub save_object: &'static str,
    pub borrow_global_mut: &'static str,
    pub borrow_global: &'static str,
    pub delete: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::account_address::AccountAddress;

    #[test]
    fn canonical_object_id_uses_full_form() {
        assert_eq!(
            canonical_object_id(
                "0x139b412b277799f0fb3e20dbd435bdaefd009535f2b1f1d44ddefd2e41c8c9d"
            )
            .unwrap(),
            "0x0139b412b277799f0fb3e20dbd435bdaefd009535f2b1f1d44ddefd2e41c8c9d"
        );
        let full = canonical_object_id("0x2").unwrap();
        assert_eq!(full.len(), 66);
        assert_eq!(full, format!("0x{}2", "0".repeat(63)));
        // Idempotent over its own output.
        assert_eq!(canonical_object_id(&full).unwrap(), full);
    }

    #[test]
    fn canonical_object_id_rejects_non_hex() {
        assert!(canonical_object_id("0xzz").is_none());
        assert!(canonical_object_id("").is_none());
        assert!(canonical_object_id("not-hex").is_none());
    }

    #[test]
    fn bytes_forms() {
        assert!(object_id_from_bytes(&[0u8; AccountAddress::LENGTH]).is_none());
        assert!(object_id_from_bytes(&[0u8; 31]).is_none());
        let mut bytes = [0u8; AccountAddress::LENGTH];
        bytes[31] = 7;
        assert_eq!(
            object_id_from_bytes(&bytes).unwrap(),
            canonical_object_id("0x7").unwrap()
        );
        assert_eq!(
            object_id_from_address_bytes(&bytes).unwrap(),
            canonical_object_id("0x7").unwrap()
        );
        assert!(object_id_from_address_bytes(&[0u8; 31]).is_none());
    }

    #[test]
    fn test_uid_record_from_hex() {
        use crate::address::Address as KanariAddress;

        let uid = UIDRecord::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap();
        let expected = AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap();
        assert_eq!(uid.addr, expected);
    }

    #[test]
    fn test_id_record_from_hex() {
        use crate::address::Address as KanariAddress;

        let id = IDRecord::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap();
        let expected = AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap();
        assert_eq!(id.bytes, expected);
    }
}
