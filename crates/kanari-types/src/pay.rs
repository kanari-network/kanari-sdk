// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};

/// Rust-side binding for Move `kanari_system::pay`.
///
/// Keep this file aligned with:
/// `crates/kanari-frameworks/packages/kanari-system/sources/pay.move`.
pub struct PayModule;

impl PayModule {
    pub const PAY_MODULE: &'static str = "pay";

    /// Get the module ID for `0x2::pay`.
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;
        let module_name = Identifier::new(Self::PAY_MODULE).context("Invalid pay module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Fully qualified Move module path.
    pub fn module_path() -> String {
        format!("{}::{}", Address::KANARI_SYSTEM_ADDRESS, Self::PAY_MODULE)
    }

    /// Get function names exposed by `pay.move`.
    pub fn function_names() -> PayFunctions {
        PayFunctions {
            keep: "keep",
            split: "split",
            split_vec: "split_vec",
            split_and_transfer: "split_and_transfer",
            divide_and_keep: "divide_and_keep",
            join: "join",
            join_vec: "join_vec",
            join_vec_and_transfer: "join_vec_and_transfer",
        }
    }

    pub fn split_and_transfer_name() -> &'static str {
        Self::function_names().split_and_transfer
    }

    pub fn split_and_transfer_path() -> String {
        format!(
            "{}::{}",
            Self::module_path(),
            Self::split_and_transfer_name()
        )
    }
}

/// Function names in Move `kanari_system::pay`.
pub struct PayFunctions {
    pub keep: &'static str,
    pub split: &'static str,
    pub split_vec: &'static str,
    pub split_and_transfer: &'static str,
    pub divide_and_keep: &'static str,
    pub join: &'static str,
    pub join_vec: &'static str,
    pub join_vec_and_transfer: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pay_module_id_points_to_system_pay() {
        let module_id = PayModule::get_module_id().unwrap();

        assert_eq!(
            module_id.address().to_hex_literal(),
            Address::KANARI_SYSTEM_ADDRESS
        );
        assert_eq!(module_id.name().as_str(), PayModule::PAY_MODULE);
    }

    #[test]
    fn pay_function_names_match_move_module() {
        let names = PayModule::function_names();

        assert_eq!(names.keep, "keep");
        assert_eq!(names.split, "split");
        assert_eq!(names.split_vec, "split_vec");
        assert_eq!(names.split_and_transfer, "split_and_transfer");
        assert_eq!(names.divide_and_keep, "divide_and_keep");
        assert_eq!(names.join, "join");
        assert_eq!(names.join_vec, "join_vec");
        assert_eq!(names.join_vec_and_transfer, "join_vec_and_transfer");
        assert_eq!(
            PayModule::split_and_transfer_path(),
            "0x2::pay::split_and_transfer"
        );
    }
}
