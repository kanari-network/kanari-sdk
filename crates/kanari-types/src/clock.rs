// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::address::Address;
use crate::object::UIDRecord;

use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

// =================================================================
// 1. BCS Data Structure (for reading State from Database)
// =================================================================

/// Structure that mirrors `struct Clock has key, store` in Move
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Clock {
    pub id: UIDRecord,
    pub timestamp_ms: u64,
}

impl Clock {
    /// Read current time from Object
    pub fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }
}

// =================================================================
// 2. Module Constants & Utilities (for running Move VM)
// =================================================================

/// Clock module constants and utilities
pub struct ClockModule;

impl ClockModule {
    pub const CLOCK_MODULE: &'static str = "clock";
    /// Name of the Clock struct in Move
    pub const CLOCK_STRUCT: &'static str = "Clock";

    /// Abort codes from `clock.move`.
    pub const E_NOT_SYSTEM_ADDRESS: u64 = 0;
    pub const E_TIMESTAMP_NOT_MONOTONIC: u64 = 1;

    /// Get the module ID for kanari_system::clock
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name =
            Identifier::new(Self::CLOCK_MODULE).context("Invalid clock module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names used in clock module
    pub fn function_names() -> ClockFunctions {
        ClockFunctions {
            timestamp_ms: "timestamp_ms",
            timestamp_ms_by_address: "timestamp_ms_by_address",
            create: "create",
            consensus_commit_prologue: "consensus_commit_prologue",
            create_for_testing: "create_for_testing",
            increment_for_testing: "increment_for_testing",
            set_for_testing: "set_for_testing",
            destroy_for_testing: "destroy_for_testing",
        }
    }

    /// Whether a type name is a `Clock` object type, matching the module and
    /// struct name regardless of the publishing address.
    pub fn is_clock_type(type_name: &str) -> bool {
        type_name.contains(&format!("::{}::{}", Self::CLOCK_MODULE, Self::CLOCK_STRUCT))
    }
}

/// Clock module function names
pub struct ClockFunctions {
    pub timestamp_ms: &'static str,
    pub timestamp_ms_by_address: &'static str,
    pub create: &'static str,
    pub consensus_commit_prologue: &'static str,
    /// The remaining four are `#[test_only]` in `clock.move`.
    pub create_for_testing: &'static str,
    pub increment_for_testing: &'static str,
    pub set_for_testing: &'static str,
    pub destroy_for_testing: &'static str,
}
