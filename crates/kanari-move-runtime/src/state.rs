use crate::changeset::{ChangeSet, CreatedObject, Event};
use anyhow::Result;
use kanari_crypto::hash_data_blake3;
use kanari_types::address::Address as KanariAddress;
use kanari_types::kanari::KanariModule;
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Account state in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: AccountAddress,
    pub balance: u64, // Native KANARI balance in Mist
    pub sequence_number: u64,
    pub modules: HashSet<String>,
    /// Token balances: token_type -> amount
    /// Example: "0x840512ff...::james::JAMES" -> 1000000000
    pub token_balances: HashMap<String, u64>,
}

impl Account {
    pub fn new(address: AccountAddress, balance: u64) -> Self {
        Self {
            address,
            balance,
            sequence_number: 0,
            modules: HashSet::new(),
            token_balances: HashMap::new(),
        }
    }

    pub fn add_module(&mut self, module_name: String) {
        self.modules.insert(module_name);
    }

    pub fn set_token_balance(&mut self, token_type: String, amount: u64) {
        self.token_balances.insert(token_type, amount);
    }

    pub fn get_token_balance(&self, token_type: &str) -> u64 {
        self.token_balances.get(token_type).copied().unwrap_or(0)
    }

    pub fn add_token(&mut self, token_type: String, amount: u64) {
        let current = self.get_token_balance(&token_type);
        self.token_balances.insert(token_type, current + amount);
    }

    pub fn sub_token(&mut self, token_type: &str, amount: u64) -> Result<()> {
        let current = self.get_token_balance(token_type);
        if current < amount {
            anyhow::bail!("Insufficient token balance");
        }
        self.token_balances
            .insert(token_type.to_string(), current - amount);
        Ok(())
    }

    pub fn to_hex_string(&self) -> String {
        format!("{:#x}", self.address)
    }

    pub fn increment_sequence(&mut self) {
        self.sequence_number += 1;
    }
}

/// Global state manager for accounts and balances
/// This is a pure data layer that applies ChangeSet from Move VM execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateManager {
    pub accounts: HashMap<AccountAddress, Account>,
    pub total_supply: u64,
    pub events: Vec<Event>,
    /// Per-token total supplies tracked via TreasuryCap (token_type -> total_supply)
    pub token_supplies: HashMap<String, u64>,
    /// Owner of TreasuryCap for a token type (token_type -> owner address)
    pub token_treasuries: HashMap<String, AccountAddress>,
    /// Map of object id -> CreatedObject for objects created by Move executions
    pub objects: HashMap<String, CreatedObject>,
    /// Per-account list of owned object ids
    pub owned_objects: HashMap<AccountAddress, Vec<String>>,
}

impl StateManager {
    /// Create new state with genesis allocation
    /// Total supply: 100 million KANARI = 100,000,000,000,000,000 Mist
    /// Dev address gets entire supply according to kanari.move
    pub fn new() -> Self {
        let mut accounts = HashMap::new();

        // Total supply in Mist (from kanari-types constants)
        let total_supply_mist: u64 = KanariModule::TOTAL_SUPPLY_MIST;

        // Initialize system accounts
        let genesis_addr =
            AccountAddress::from_hex_literal(KanariAddress::GENESIS_ADDRESS).unwrap();
        let std_addr = AccountAddress::from_hex_literal(KanariAddress::STD_ADDRESS).unwrap();
        let system_addr =
            AccountAddress::from_hex_literal(KanariAddress::KANARI_SYSTEM_ADDRESS).unwrap();
        let dao_addr = AccountAddress::from_hex_literal(KanariAddress::DAO_ADDRESS).unwrap();
        let dev_addr = AccountAddress::from_hex_literal(KanariAddress::DEV_ADDRESS).unwrap();

        accounts.insert(genesis_addr, Account::new(genesis_addr, 0));
        accounts.insert(std_addr, Account::new(std_addr, 0));
        accounts.insert(system_addr, Account::new(system_addr, 0));
        accounts.insert(dao_addr, Account::new(dao_addr, 0));
        accounts.insert(dev_addr, Account::new(dev_addr, total_supply_mist));

        Self {
            accounts,
            total_supply: total_supply_mist,
            events: Vec::new(),
            token_supplies: HashMap::new(),
            token_treasuries: HashMap::new(),
            objects: HashMap::new(),
            owned_objects: HashMap::new(),
        }
    }

    pub fn get_or_create_account(&mut self, address: AccountAddress) -> &mut Account {
        self.accounts
            .entry(address)
            .or_insert_with(|| Account::new(address, 0))
    }

    pub fn get_account(&self, address: &AccountAddress) -> Option<&Account> {
        self.accounts.get(address)
    }

    pub fn get_account_by_hex(&self, hex_address: &str) -> Option<&Account> {
        if let Ok(addr) = AccountAddress::from_hex_literal(hex_address) {
            self.accounts.get(&addr)
        } else {
            None
        }
    }

    /// Apply ChangeSet from Move VM execution
    /// This is the ONLY way to modify state - all changes must come from Move VM
    ///
    /// CRITICAL: This method applies ALL changes in the ChangeSet, regardless of
    /// the 'success' status. Failed transactions MUST still deduct gas fees and
    /// increment sequence numbers to prevent replay attacks.
    ///
    /// The BlockchainEngine is responsible for ensuring that failed transaction
    /// ChangeSets only contain necessary changes (gas deduction, sequence increment).
    pub fn apply_changeset(&mut self, changeset: &ChangeSet) -> Result<()> {
        // Track total supply change (for mint/burn operations)
        let mut supply_delta: i64 = 0;

        for (address, change) in &changeset.account_changes {
            let account = self.get_or_create_account(*address);

            // Apply balance delta
            if change.balance_delta > 0 {
                let amount = change.balance_delta as u64;
                account.balance = account
                    .balance
                    .checked_add(amount)
                    .ok_or_else(|| anyhow::anyhow!("Balance overflow"))?;
                supply_delta += change.balance_delta;
            } else if change.balance_delta < 0 {
                let debit = (-change.balance_delta) as u64;
                if account.balance < debit {
                    anyhow::bail!(
                        "Insufficient balance for address {:#x}: need {} but have {}",
                        address,
                        debit,
                        account.balance
                    );
                }
                account.balance -= debit;
                supply_delta += change.balance_delta;
            }

            // Apply sequence number increment
            account.sequence_number += change.sequence_increment;

            // Apply module additions
            for module_name in &change.modules_added {
                account.add_module(module_name.clone());
            }
            // If modules were added, treat this as a transaction from the publisher
            // and consume a sequence number (engine behavior).
            if !change.modules_added.is_empty() {
                account.sequence_number += 1;
            }
        }

        // Update total supply if there was mint/burn (supply_delta != 0)
        if supply_delta != 0 {
            if supply_delta > 0 {
                self.total_supply = self
                    .total_supply
                    .checked_add(supply_delta as u64)
                    .ok_or_else(|| anyhow::anyhow!("Total supply overflow"))?;
            } else {
                let burn_amount = (-supply_delta) as u64;
                if self.total_supply < burn_amount {
                    anyhow::bail!("Cannot burn more than total supply");
                }
                self.total_supply -= burn_amount;
            }
        }

        // Apply treasury creations/updates from ChangeSet (track token supplies and owners)
        for (owner, token_type, total_supply) in &changeset.treasuries {
            // set or update total supply for this token
            self.token_supplies
                .insert(token_type.clone(), *total_supply);
            // record treasury owner
            self.token_treasuries.insert(token_type.clone(), *owner);
        }

        // Apply per-account token balance sets from ChangeSet (absolute values)
        for (owner, token_type, amount) in &changeset.token_balance_sets {
            let account = self.get_or_create_account(*owner);
            account.set_token_balance(token_type.clone(), *amount);
        }

        // Persist created objects from ChangeSet into the object registry and
        // register ownership mapping so RPC/account queries can list owned objects.
        for created in &changeset.created_objects {
            // store the object by id
            self.objects.insert(created.id.clone(), created.clone());

            // add to owner's owned_objects list
            self.owned_objects
                .entry(created.owner)
                .or_insert_with(Vec::new)
                .push(created.id.clone());
        }

        // Persist events emitted by Move VM into state event store
        if !changeset.events.is_empty() {
            self.events.extend(changeset.events.clone());
        }

        Ok(())
    }

    /// Validate transaction sequence number before execution
    pub fn validate_sequence(
        &self,
        address: &AccountAddress,
        expected_sequence: u64,
    ) -> Result<()> {
        if let Some(account) = self.get_account(address) {
            if account.sequence_number != expected_sequence {
                anyhow::bail!(
                    "Sequence number mismatch for {:#x}: expected {}, got {}",
                    address,
                    account.sequence_number,
                    expected_sequence
                );
            }
        } else if expected_sequence != 0 {
            anyhow::bail!(
                "Account {:#x} does not exist, expected sequence must be 0",
                address
            );
        }
        Ok(())
    }

    // Deprecated direct mutation helpers removed. Use `ChangeSet` + `apply_changeset()` instead.

    pub fn get_balance(&self, address: &str) -> u64 {
        if let Ok(addr) = AccountAddress::from_hex_literal(address) {
            self.accounts.get(&addr).map(|acc| acc.balance).unwrap_or(0)
        } else {
            0
        }
    }

    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    pub fn compute_state_root(&self) -> Vec<u8> {
        let serialized = bcs::to_bytes(&self.accounts).unwrap();
        hash_data_blake3(&serialized)
    }

    /// Drain and return all accumulated events from the state event store.
    /// This moves events out so callers can process them without cloning.
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    // Gas collection is handled via ChangeSet; no separate `collect_gas` method on StateManager.
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_manager_creation() {
        let state = StateManager::new();
        assert_eq!(state.accounts.len(), 5); // Genesis, Std, System, DAO, Dev
        let dev_addr = AccountAddress::from_hex_literal(KanariAddress::DEV_ADDRESS).unwrap();
        assert!(state.accounts.contains_key(&dev_addr));
    }

    #[test]
    fn test_get_or_create_account() {
        let mut state = StateManager::new();
        let addr = AccountAddress::from_hex_literal("0x123").unwrap();
        let account = state.get_or_create_account(addr);
        assert_eq!(account.address, addr);
        assert_eq!(account.balance, 0);
    }

    #[test]
    fn test_apply_changeset_transfer() {
        let mut state = StateManager::new();
        let from = AccountAddress::from_hex_literal("0x1").unwrap();
        let to = AccountAddress::from_hex_literal("0x2").unwrap();

        // Give initial balance to sender
        state.get_or_create_account(from).balance = 1000;

        // Create changeset for transfer
        let mut cs = ChangeSet::new();
        cs.transfer(from, to, 500);

        state.apply_changeset(&cs).unwrap();

        assert_eq!(state.get_account(&from).unwrap().balance, 500);
        assert_eq!(state.get_account(&to).unwrap().balance, 500);
    }

    #[test]
    fn test_apply_changeset_mint() {
        let mut state = StateManager::new();
        let to = AccountAddress::from_hex_literal("0x1").unwrap();

        let mut cs = ChangeSet::new();
        cs.mint(to, 1000);

        state.apply_changeset(&cs).unwrap();
        assert_eq!(state.get_account(&to).unwrap().balance, 1000);
    }

    #[test]
    fn test_apply_changeset_module_publish() {
        let mut state = StateManager::new();
        let publisher = AccountAddress::from_hex_literal("0x2").unwrap();

        let mut cs = ChangeSet::new();
        cs.publish_module(publisher, "kanari".to_string());

        state.apply_changeset(&cs).unwrap();

        let account = state.get_account(&publisher).unwrap();
        assert!(account.modules.contains("kanari"));
        assert_eq!(account.sequence_number, 1);
    }

    #[test]
    fn test_legacy_transfer_equivalent() {
        // Ensure legacy behavior (mint + transfer) can be represented via ChangeSet + apply_changeset
        let mut state = StateManager::new();

        // Mint 1000 to 0x1 using ChangeSet
        let from = AccountAddress::from_hex_literal("0x1").unwrap();
        let to = AccountAddress::from_hex_literal("0x2").unwrap();

        let mut cs = ChangeSet::new();
        cs.mint(from, 1000);
        cs.transfer(from, to, 500);

        state.apply_changeset(&cs).unwrap();

        assert_eq!(state.get_balance("0x1"), 500);
        assert_eq!(state.get_balance("0x2"), 500);
    }

    #[test]
    fn test_total_supply_tracking() {
        let mut state = StateManager::new();
        let initial_supply = state.total_supply;

        // Mint increases supply
        let to = AccountAddress::from_hex_literal("0x123").unwrap();
        let mut cs = ChangeSet::new();
        cs.mint(to, 1000);
        state.apply_changeset(&cs).unwrap();

        assert_eq!(state.total_supply, initial_supply + 1000);

        // Burn decreases supply
        let mut cs = ChangeSet::new();
        cs.burn(to, 500);
        state.apply_changeset(&cs).unwrap();

        assert_eq!(state.total_supply, initial_supply + 500);
    }

    #[test]
    fn test_sequence_validation() {
        let state = StateManager::new();
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();

        // Account exists with sequence 0
        assert!(state.validate_sequence(&addr, 0).is_ok());

        // Wrong sequence should fail
        assert!(state.validate_sequence(&addr, 1).is_err());

        // Non-existent account with sequence 0 should pass
        let new_addr = AccountAddress::from_hex_literal("0x999").unwrap();
        assert!(state.validate_sequence(&new_addr, 0).is_ok());

        // Non-existent account with non-zero sequence should fail
        assert!(state.validate_sequence(&new_addr, 1).is_err());
    }

    #[test]
    fn test_balance_overflow_protection() {
        let mut state = StateManager::new();
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();

        // Set balance to near max
        state.get_or_create_account(addr).balance = u64::MAX - 100;

        // Try to add more than available space
        let mut cs = ChangeSet::new();
        cs.mint(addr, 200);

        // Should fail with overflow error
        assert!(state.apply_changeset(&cs).is_err());
    }

    #[test]
    fn test_changeset_with_multiple_operations() {
        let mut state = StateManager::new();
        let from = AccountAddress::from_hex_literal("0x1").unwrap();
        let to = AccountAddress::from_hex_literal("0x2").unwrap();
        let dao = AccountAddress::from_hex_literal(KanariAddress::DAO_ADDRESS).unwrap();

        // Setup initial balance
        state.get_or_create_account(from).balance = 1000;

        // Create changeset with transfer + gas collection
        let mut cs = ChangeSet::new();
        cs.transfer(from, to, 100);
        cs.collect_gas(dao, 10); // Gas collected to DAO
        cs.set_gas_used(10);

        // Apply changeset
        state.apply_changeset(&cs).unwrap();

        // Verify: sender lost 100, receiver gained 100, DAO gained 10
        assert_eq!(state.get_account(&from).unwrap().balance, 900);
        assert_eq!(state.get_account(&to).unwrap().balance, 100);
        assert_eq!(state.get_account(&dao).unwrap().balance, 10);

        // Verify sequence incremented for sender
        assert_eq!(state.get_account(&from).unwrap().sequence_number, 1);
    }

    #[test]
    fn test_failed_transaction_gas_deduction() {
        // CRITICAL TEST: Failed transactions MUST still deduct gas
        let mut state = StateManager::new();
        let sender = AccountAddress::from_hex_literal("0x123").unwrap();
        let dao = AccountAddress::from_hex_literal(KanariAddress::DAO_ADDRESS).unwrap();

        // Setup sender with 1000 balance
        state.get_or_create_account(sender).balance = 1000;

        // Create a FAILED transaction changeset (success: false)
        // But it should still contain gas deduction and sequence increment
        let mut cs = ChangeSet::new();
        cs.mark_failed("Execution failed: out of gas".to_string());

        // Even though transaction failed, gas is charged
        let gas_cost = 50u64;
        let sender_change = cs.get_or_create_change(sender);
        sender_change.debit(gas_cost); // Deduct gas from sender
        sender_change.increment_sequence(); // Increment sequence to prevent replay

        cs.collect_gas(dao, gas_cost); // DAO receives gas
        cs.set_gas_used(gas_cost);

        // Apply the FAILED changeset
        state.apply_changeset(&cs).unwrap();

        // ASSERTIONS: Even though transaction failed, gas was deducted
        assert_eq!(
            state.get_account(&sender).unwrap().balance,
            950,
            "Failed transaction MUST deduct gas from sender"
        );
        assert_eq!(
            state.get_account(&dao).unwrap().balance,
            50,
            "Failed transaction MUST credit gas to DAO"
        );
        assert_eq!(
            state.get_account(&sender).unwrap().sequence_number,
            1,
            "Failed transaction MUST increment sequence to prevent replay"
        );
    }
}
