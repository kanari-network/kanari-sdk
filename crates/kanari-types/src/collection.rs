// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::address::Address;
use crate::object::UIDRecord;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

/// Collection record (mirrors `Collection` in Move)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CollectionRecord {
    pub id: UIDRecord,
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub creator: AccountAddress,
    pub max_supply: u64,
}

impl CollectionRecord {
    /// Create a new collection record
    pub fn new(
        id: UIDRecord,
        name: Vec<u8>,
        description: Vec<u8>,
        creator: AccountAddress,
        max_supply: u64,
    ) -> Self {
        Self {
            id,
            name,
            description,
            creator,
            max_supply,
        }
    }

    /// Return the collection id address (derived from UID)
    pub fn collection_id_address(&self) -> AccountAddress {
        self.id.address()
    }

    /// Return name as UTF-8 string
    pub fn name_str(&self) -> Result<String> {
        String::from_utf8(self.name.clone()).context("Invalid UTF-8 in name")
    }

    /// Return description as UTF-8 string
    pub fn description_str(&self) -> Result<String> {
        String::from_utf8(self.description.clone()).context("Invalid UTF-8 in description")
    }
}

/// NFT capability record (mirrors `NftCap` in Move)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NftCapRecord {
    pub id: UIDRecord,
    pub remaining: u64,
    pub issued_counter: u64,
    pub collection_id: AccountAddress,
}

impl NftCapRecord {
    /// Create a new NftCap
    pub fn new(
        id: UIDRecord,
        remaining: u64,
        issued_counter: u64,
        collection_id: AccountAddress,
    ) -> Self {
        Self {
            id,
            remaining,
            issued_counter,
            collection_id,
        }
    }

    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    pub fn issued(&self) -> u64 {
        self.issued_counter
    }

    /// Consume one supply unit for minting (panics on no supply)
    pub fn consume_for_mint(&mut self) {
        assert!(self.remaining > 0, "no supply");
        self.issued_counter = self.issued_counter.saturating_add(1);
        self.remaining = self.remaining.saturating_sub(1);
    }

    /// Return one supply unit to the cap (used on burn)
    pub fn return_from_burn(&mut self) {
        self.remaining = self.remaining.saturating_add(1);
    }

    pub fn cap_collection_id(&self) -> AccountAddress {
        self.collection_id
    }
}

/// Collection module constants and utilities
pub struct CollectionModule;

impl CollectionModule {
    pub const COLLECTION_MODULE: &'static str = "collection";
    pub const COLLECTION_STRUCT: &'static str = "Collection";
    pub const NFTCAP_STRUCT: &'static str = "NftCap";

    /// Get the module ID for kanari_system::collection
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name =
            Identifier::new(Self::COLLECTION_MODULE).context("Invalid collection module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names used in collection module
    pub fn function_names() -> CollectionFunctions {
        CollectionFunctions {
            create_collection: "create_collection",
            collection_id: "collection_id",
            cap_collection_id: "cap_collection_id",
            collection_creator: "collection_creator",
            max_supply: "max_supply",
            remaining: "remaining",
            issued: "issued",
            consume_for_mint: "consume_for_mint",
            return_from_burn: "return_from_burn",
            transfer_collection: "transfer_collection",
            transfer_cap: "transfer_cap",
        }
    }
}

/// Collection module function names
pub struct CollectionFunctions {
    pub create_collection: &'static str,
    pub collection_id: &'static str,
    pub cap_collection_id: &'static str,
    pub collection_creator: &'static str,
    pub max_supply: &'static str,
    pub remaining: &'static str,
    pub issued: &'static str,
    pub consume_for_mint: &'static str,
    pub return_from_burn: &'static str,
    pub transfer_collection: &'static str,
    pub transfer_cap: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::Address as KanariAddress;

    #[test]
    fn test_nftcap_consume_and_return() {
        let uid = UIDRecord::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap();
        let mut cap = NftCapRecord::new(
            uid,
            2,
            0,
            AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap(),
        );

        assert_eq!(cap.remaining(), 2);
        cap.consume_for_mint();
        assert_eq!(cap.remaining(), 1);
        assert_eq!(cap.issued(), 1);

        cap.return_from_burn();
        assert_eq!(cap.remaining(), 2);
    }

    #[test]
    fn test_collection_record_strings() {
        let uid = UIDRecord::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap();
        let coll = CollectionRecord::new(
            uid,
            b"Test".to_vec(),
            b"Desc".to_vec(),
            AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap(),
            10,
        );

        assert_eq!(coll.name_str().unwrap(), "Test");
        assert_eq!(coll.description_str().unwrap(), "Desc");
    }
}
