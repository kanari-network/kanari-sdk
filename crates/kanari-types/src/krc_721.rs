// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Canonical identifiers and stable wire-layout helpers for KRC-721.
//!
//! The runtime uses this module rather than spelling framework paths or BCS
//! offsets inline.  `Nft` starts with `UID` (`ID` / address) followed by its
//! `collection: address`; both fields are fixed-width 32-byte values in BCS.

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use std::str::FromStr;

/// KRC-721 framework module metadata.
pub struct Krc721Module;

impl Krc721Module {
    pub const MODULE: &'static str = "krc_721";
    pub const MODULE_PATH: &'static str = "0x2::krc_721";

    // Struct Names
    pub const NFT_STRUCT: &'static str = "Nft";
    pub const COLLECTION_DISPLAY_STRUCT: &'static str = "CollectionDisplay";
    pub const COLLECTION_POLICY_SCOPE_STRUCT: &'static str = "CollectionPolicyScope";
    pub const COLLECTION_POLICY_CAP_STRUCT: &'static str = "CollectionPolicyCap";
    pub const MINT_LOG_STRUCT: &'static str = "MintLog";

    pub const DENY_LIST_INDEX: u64 = 1;

    // Abort Codes from krc_721.move
    pub const E_ATTRIBUTE_LENGTH_MISMATCH: u64 = 0;
    pub const E_TOO_MANY_ATTRIBUTES: u64 = 1;
    pub const E_NFT_COLLECTION_MISMATCH: u64 = 2;
    pub const E_DISPLAY_FIELD_COUNT_MISMATCH: u64 = 3;
    pub const E_PROTECTED_DISPLAY_FIELD: u64 = 4;
    pub const E_DISPLAY_COLLECTION_MISMATCH: u64 = 5;
    pub const E_POLICY_MISMATCH: u64 = 6;
    pub const E_POLICY_COLLECTION_MISMATCH: u64 = 7;
    pub const E_DENIED_OR_PAUSED: u64 = 8;
    pub const MAX_ATTRIBUTES: u64 = 64;

    /// BCS offset of `Nft.collection`: `Nft.id: UID` occupies one address.
    pub const COLLECTION_OFFSET: usize = AccountAddress::LENGTH;
    pub const COLLECTION_END: usize = Self::COLLECTION_OFFSET + AccountAddress::LENGTH;

    /// Fully qualified Move module path for `0x2::krc_721`.
    pub fn module_path() -> String {
        format!("{}::{}", Address::KANARI_SYSTEM_ADDRESS, Self::MODULE)
    }

    /// Get the module ID for `kanari_system::krc_721`
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name = Identifier::new(Self::MODULE).context("Invalid krc_721 module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Compatibility alias for [`Self::get_module_id`].
    pub fn module_id() -> ModuleId {
        ModuleId::new(
            Address::kanari_system_account_address(),
            Identifier::new(Self::MODULE).expect("KRC-721 module identifier is valid"),
        )
    }

    /// Canonical function names used by the runtime and API.
    pub fn function_names() -> Krc721Functions {
        Krc721Functions {
            create_collection_display: "create_collection_display",
            share_collection_display: "share_collection_display",
            new_collection_policy: "new_collection_policy",
            mint: "mint",
            mint_with_attributes: "mint_with_attributes",
            burn: "burn",
            transfer: "transfer",
            transfer_checked: "transfer_checked",
            transfer_with_global_deny: "transfer_with_global_deny",
            update: "update",
            is_denied: "is_denied",
            is_paused: "is_paused",
        }
    }

    /// Returns true only for the canonical system KRC-721 `Nft` object.
    pub fn is_nft_object_type(object_type: &str) -> bool {
        let Ok(TypeTag::Struct(tag)) = TypeTag::from_str(object_type) else {
            return false;
        };
        tag.address == Address::kanari_system_account_address()
            && tag.module.as_str() == Self::MODULE
            && tag.name.as_str() == Self::NFT_STRUCT
            && tag.type_params.is_empty()
    }

    /// Extract the collection address from canonical KRC-721 NFT BCS data.
    pub fn collection_from_nft_bcs(data: &[u8]) -> Option<AccountAddress> {
        let bytes = data.get(Self::COLLECTION_OFFSET..Self::COLLECTION_END)?;
        AccountAddress::from_bytes(bytes).ok()
    }
}

/// KRC-721 module function names.
pub struct Krc721Functions {
    pub create_collection_display: &'static str,
    pub share_collection_display: &'static str,
    pub new_collection_policy: &'static str,
    pub mint: &'static str,
    pub mint_with_attributes: &'static str,
    pub burn: &'static str,
    pub transfer: &'static str,
    pub transfer_checked: &'static str,
    pub transfer_with_global_deny: &'static str,
    pub update: &'static str,
    pub is_denied: &'static str,
    pub is_paused: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_only_canonical_krc_721_nft() {
        assert!(Krc721Module::is_nft_object_type("0x2::krc_721::Nft"));
        assert!(!Krc721Module::is_nft_object_type("0x3::krc_721::Nft"));
        assert!(!Krc721Module::is_nft_object_type("0x2::krc_721::Nft<u64>"));
    }

    #[test]
    fn reads_collection_from_stable_prefix() {
        let collection = AccountAddress::from_hex_literal("0xcafe").unwrap();
        let mut bytes = vec![0u8; Krc721Module::COLLECTION_END];
        bytes[Krc721Module::COLLECTION_OFFSET..].copy_from_slice(collection.as_ref());
        assert_eq!(
            Krc721Module::collection_from_nft_bcs(&bytes),
            Some(collection)
        );
        assert_eq!(Krc721Module::collection_from_nft_bcs(&bytes[..63]), None);
    }
}
