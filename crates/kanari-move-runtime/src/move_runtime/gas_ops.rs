// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Gas metering and accounting operations
use crate::changeset::ChangeSet;
use crate::gas::{GasMeter, GasOperation};
use anyhow::Result;
use kanari_types::address::Address as KanariAddress;
use move_core_types::account_address::AccountAddress;

impl super::MoveRuntime {
    /// Helper to apply gas accounting to a ChangeSet. Handles sender debit + sequence increment
    /// and credits gas to DAO. `sender` may be `None` for system-level calls.
    pub(crate) fn apply_gas_info(
        &self,
        cs: &mut ChangeSet,
        sender: Option<AccountAddress>,
        gas_limit: u64,
        gas_price: u64,
        gas_op: GasOperation,
    ) -> Result<()> {
        let mut meter = GasMeter::new(gas_limit, gas_price);
        meter.consume(gas_op.gas_units())?;
        let gas_cost = meter.total_cost();

        if let Some(saddr) = sender {
            let sender_change = cs.get_or_create_change(saddr);
            sender_change.increment_sequence();
            sender_change.debit(gas_cost);
        }

        let dao_addr = AccountAddress::from_hex_literal(KanariAddress::DAO_ADDRESS)?;
        cs.collect_gas(dao_addr, gas_cost);
        cs.set_gas_used(meter.gas_used);
        Ok(())
    }
}
