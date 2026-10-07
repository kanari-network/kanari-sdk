// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Clock and locked-coin type predicates for changeset application.

use super::*;

impl StateManager {
    pub(crate) fn is_system_clock_object(&self, object_id: &str, type_name: &str) -> Result<bool> {
        let is_clock = Self::is_system_clock_type(type_name);
        match is_clock {
            Some(false) => return Ok(false),
            Some(true) => {}
            None => return Ok(false),
        }
        let object_address = AccountAddress::from_hex_literal(object_id).ok();
        let configured_clock = self.get_system_clock_object_id()?;
        Ok(configured_clock.is_none() || configured_clock == object_address)
    }

    pub(crate) fn is_system_clock_type(type_name: &str) -> Option<bool> {
        let Ok(struct_tag) = StructTag::from_str(type_name) else {
            return None;
        };
        Some(
            struct_tag.module.as_str() == ClockModule::CLOCK_MODULE
                && struct_tag.name.as_str() == ClockModule::CLOCK_STRUCT
                && struct_tag.type_params.is_empty(),
        )
    }

    pub(crate) fn is_object_locked_coin_holder_type(type_name: &str) -> bool {
        let Ok(struct_tag) = StructTag::from_str(type_name) else {
            return false;
        };

        if is_balance_struct(&struct_tag) {
            return false;
        }

        let module_name = struct_tag.module.as_str();
        let struct_name = struct_tag.name.as_str();
        !(module_name == CoinModule::COIN_MODULE
            && (struct_name == CoinModule::TREASURY_CAP_STRUCT || struct_name == "CoinMetadata"))
    }
}
