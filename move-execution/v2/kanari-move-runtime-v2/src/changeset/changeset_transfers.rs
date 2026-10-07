// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Balance and module-publish operations on a changeset.

use crate::changeset::ChangeSet;
use move_core_types::account_address::AccountAddress;

impl ChangeSet {
    /// Transfer operation: debit sender, credit receiver
    pub fn transfer(&mut self, from: AccountAddress, to: AccountAddress, amount: u64) {
        let sender = self.get_or_create_owner_delta(from);
        sender.debit(amount);

        let receiver = self.get_or_create_owner_delta(to);
        receiver.credit(amount);
    }

    /// Mint operation: create new tokens
    pub fn mint(&mut self, to: AccountAddress, amount: u64) {
        self.get_or_create_owner_delta(to).credit(amount);
    }

    /// Burn operation: destroy tokens
    pub fn burn(&mut self, from: AccountAddress, amount: u64) {
        self.get_or_create_owner_delta(from).debit(amount);
    }

    /// Module publish operation. Sequence handling remains in the engine layer.
    pub fn publish_module(&mut self, publisher: AccountAddress, module_name: String) {
        self.get_or_create_owner_delta(publisher)
            .add_module(module_name);
    }
}
