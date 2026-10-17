// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

/// Move VM Event representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub key: Vec<u8>,
    pub sequence_number: u64,
    pub type_tag: String,
    pub event_data: Vec<u8>,
}

/// Rust-side binding for Move `kanari_system::event`.
///
/// Keep this file aligned with:
/// `crates/kanari-frameworks/packages/kanari-system/sources/event.move`.
pub struct EventModule;

impl EventModule {
    pub const EVENT_MODULE: &'static str = "event";

    /// Get the module ID for `0x2::event`.
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;
        let module_name =
            Identifier::new(Self::EVENT_MODULE).context("Invalid event module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Fully qualified Move module path.
    pub fn module_path() -> String {
        format!("{}::{}", Address::KANARI_SYSTEM_ADDRESS, Self::EVENT_MODULE)
    }

    /// Get function names used in event module.
    pub fn function_names() -> EventFunctions {
        EventFunctions { emit: "emit" }
    }
}

/// Event module function names.
pub struct EventFunctions {
    pub emit: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_function_names_match_move_module() {
        let module_id = EventModule::get_module_id().unwrap();
        assert_eq!(module_id.name().as_str(), EventModule::EVENT_MODULE);
        assert_eq!(EventModule::function_names().emit, "emit");
    }
}
