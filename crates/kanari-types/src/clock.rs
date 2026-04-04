// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::address::Address;
use crate::object::UIDRecord; // 🚨 แก้ไขตรงนี้

use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

// =================================================================
// 1. BCS Data Structure (สำหรับอ่านข้อมูล State จาก Database)
// =================================================================

/// โครงสร้างที่จำลองมาจาก `struct Clock has key, store` ใน Move
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Clock {
    pub id: UIDRecord, // 🚨 แก้ไขตรงนี้
    pub timestamp_ms: u64,
}

impl Clock {
    /// อ่านเวลาปัจจุบันจาก Object
    pub fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }
}

// =================================================================
// 2. Module Constants & Utilities (สำหรับรัน Move VM)
// =================================================================

/// Clock module constants and utilities
pub struct ClockModule;

impl ClockModule {
    pub const CLOCK_MODULE: &'static str = "clock";
    /// Name of the Clock struct in Move
    pub const CLOCK_STRUCT: &'static str = "Clock";

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
            create: "create",
            timestamp_ms: "timestamp_ms",
            consensus_commit_prologue: "consensus_commit_prologue",
        }
    }
}

/// Clock module function names
pub struct ClockFunctions {
    pub create: &'static str,
    pub timestamp_ms: &'static str,
    pub consensus_commit_prologue: &'static str,
}
