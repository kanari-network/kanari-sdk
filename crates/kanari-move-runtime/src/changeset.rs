use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Move VM Event representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub key: Vec<u8>,
    pub sequence_number: u64,
    pub type_tag: String,
    pub event_data: Vec<u8>,
}

/// Represents changes to account state from Move VM execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountChange {
    pub address: AccountAddress,
    pub balance_delta: i64, // Positive = credit, Negative = debit
    pub sequence_increment: u64,
    pub modules_added: Vec<String>,
}

impl AccountChange {
    pub fn new(address: AccountAddress) -> Self {
        Self {
            address,
            balance_delta: 0,
            sequence_increment: 0,
            modules_added: vec![],
        }
    }

    pub fn debit(&mut self, amount: u64) {
        self.balance_delta -= amount as i64;
    }

    pub fn credit(&mut self, amount: u64) {
        self.balance_delta += amount as i64;
    }

    pub fn increment_sequence(&mut self) {
        self.sequence_increment += 1;
    }

    pub fn add_module(&mut self, module_name: String) {
        self.modules_added.push(module_name);
    }
}

/// ChangeSet represents all state changes from Move VM execution
/// This is the canonical output from Move VM that StateManager will apply
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeSet {
    pub account_changes: HashMap<AccountAddress, AccountChange>,
    pub events: Vec<Event>,
    pub gas_used: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

impl ChangeSet {
    pub fn new() -> Self {
        Self {
            account_changes: HashMap::new(),
            events: Vec::new(),
            gas_used: 0,
            success: true,
            error_message: None,
        }
    }

    pub fn with_gas(gas_used: u64) -> Self {
        Self {
            account_changes: HashMap::new(),
            events: Vec::new(),
            gas_used,
            success: true,
            error_message: None,
        }
    }

    pub fn failed(error: String, gas_used: u64) -> Self {
        Self {
            account_changes: HashMap::new(),
            events: Vec::new(),
            gas_used,
            success: false,
            error_message: Some(error),
        }
    }

    pub fn get_or_create_change(&mut self, address: AccountAddress) -> &mut AccountChange {
        self.account_changes
            .entry(address)
            .or_insert_with(|| AccountChange::new(address))
    }

    /// Transfer operation: debit sender, credit receiver
    pub fn transfer(&mut self, from: AccountAddress, to: AccountAddress, amount: u64) {
        let sender = self.get_or_create_change(from);
        sender.debit(amount);
        sender.increment_sequence();

        let receiver = self.get_or_create_change(to);
        receiver.credit(amount);
    }

    /// Mint operation: create new tokens
    pub fn mint(&mut self, to: AccountAddress, amount: u64) {
        let receiver = self.get_or_create_change(to);
        receiver.credit(amount);
    }

    /// Burn operation: destroy tokens
    pub fn burn(&mut self, from: AccountAddress, amount: u64) {
        let sender = self.get_or_create_change(from);
        sender.debit(amount);
    }

    /// Module publish operation
    pub fn publish_module(&mut self, publisher: AccountAddress, module_name: String) {
        let account = self.get_or_create_change(publisher);
        account.add_module(module_name);
        account.increment_sequence();
    }

    /// Collect gas fees to DAO
    pub fn collect_gas(&mut self, dao_address: AccountAddress, gas_amount: u64) {
        let dao = self.get_or_create_change(dao_address);
        dao.credit(gas_amount);
    }

    pub fn set_gas_used(&mut self, gas: u64) {
        self.gas_used = gas;
    }

    pub fn mark_failed(&mut self, error: String) {
        self.success = false;
        self.error_message = Some(error);
    }

    pub fn is_empty(&self) -> bool {
        self.account_changes.is_empty()
    }

    pub fn account_count(&self) -> usize {
        self.account_changes.len()
    }

    /// Merge another ChangeSet into this one
    /// Used to combine Move VM changes with gas/sequence changes
    pub fn merge(&mut self, other: ChangeSet) {
        for (addr, other_change) in other.account_changes {
            let existing = self.get_or_create_change(addr);
            existing.balance_delta += other_change.balance_delta;
            existing.sequence_increment += other_change.sequence_increment;
            existing.modules_added.extend(other_change.modules_added);
        }
        self.events.extend(other.events);
        self.gas_used += other.gas_used;
        if !other.success {
            self.success = false;
            self.error_message = other.error_message;
        }
    }

    pub fn add_event(&mut self, event: Event) {
        self.events.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_changeset_transfer() {
        let mut cs = ChangeSet::new();
        let from = AccountAddress::from_hex_literal("0x1").unwrap();
        let to = AccountAddress::from_hex_literal("0x2").unwrap();

        cs.transfer(from, to, 100);

        assert_eq!(cs.account_changes.len(), 2);
        assert_eq!(cs.account_changes.get(&from).unwrap().balance_delta, -100);
        assert_eq!(cs.account_changes.get(&to).unwrap().balance_delta, 100);
        assert_eq!(cs.account_changes.get(&from).unwrap().sequence_increment, 1);
    }

    #[test]
    fn test_changeset_mint() {
        let mut cs = ChangeSet::new();
        let to = AccountAddress::from_hex_literal("0x1").unwrap();

        cs.mint(to, 1000);

        assert_eq!(cs.account_changes.len(), 1);
        assert_eq!(cs.account_changes.get(&to).unwrap().balance_delta, 1000);
    }

    #[test]
    fn test_changeset_burn() {
        let mut cs = ChangeSet::new();
        let from = AccountAddress::from_hex_literal("0x1").unwrap();

        cs.burn(from, 500);

        assert_eq!(cs.account_changes.len(), 1);
        assert_eq!(cs.account_changes.get(&from).unwrap().balance_delta, -500);
    }

    #[test]
    fn test_changeset_module_publish() {
        let mut cs = ChangeSet::new();
        let publisher = AccountAddress::from_hex_literal("0x2").unwrap();

        cs.publish_module(publisher, "kanari".to_string());

        let change = cs.account_changes.get(&publisher).unwrap();
        assert_eq!(change.modules_added.len(), 1);
        assert_eq!(change.modules_added[0], "kanari");
        assert_eq!(change.sequence_increment, 1);
    }
}
