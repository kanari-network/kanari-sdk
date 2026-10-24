// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Canonical identifiers and stable wire-layout helpers for KRC-721.
//!
//! The runtime uses this module rather than spelling framework paths or BCS
//! offsets inline.  `Nft` starts with `UID` (`ID` / address) followed by its
//! `collection: address`; both fields are fixed-width 32-byte values in BCS.

use crate::address::Address;
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
    pub const NFT_STRUCT: &'static str = "Nft";
    pub const DENY_LIST_INDEX: u64 = 1;
    pub const IS_DENIED_FUNCTION: &'static str = "is_denied";
    pub const IS_PAUSED_FUNCTION: &'static str = "is_paused";

    /// BCS offset of `Nft.collection`: `Nft.id: UID` occupies one address.
    pub const COLLECTION_OFFSET: usize = AccountAddress::LENGTH;
    pub const COLLECTION_END: usize = Self::COLLECTION_OFFSET + AccountAddress::LENGTH;

    pub fn module_id() -> ModuleId {
        ModuleId::new(
            Address::kanari_system_account_address(),
            Identifier::new(Self::MODULE).expect("KRC-721 module identifier is valid"),
        )
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
    ///
    /// Returning `None` for short/corrupt data lets consensus callers fail
    /// closed instead of accidentally treating an unverified NFT as allowed.
    pub fn collection_from_nft_bcs(data: &[u8]) -> Option<AccountAddress> {
        let bytes = data.get(Self::COLLECTION_OFFSET..Self::COLLECTION_END)?;
        AccountAddress::from_bytes(bytes).ok()
    }
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
