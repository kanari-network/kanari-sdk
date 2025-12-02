use crate::address::Address;
use anyhow::{Context, Result};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

/// Coin wrapper (mirrors `Coin<T>` in Move)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoinRecord {
    pub value: u64,
}

impl CoinRecord {
    /// Create a new coin
    pub fn new(value: u64) -> Self {
        Self { value }
    }

    /// Get coin value
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Burn coin and return value (consumes coin)
    pub fn burn(self) -> u64 {
        self.value
    }

    /// Convert coin into a raw balance (same as burn but kept for API parity)
    pub fn into_balance(self) -> u64 {
        self.value
    }

    /// Construct a coin from a raw balance
    pub fn from_balance(balance: u64) -> Self {
        Self { value: balance }
    }

    /// Split off `amount` from this coin, returning a new coin with that amount.
    /// Panics if `amount` is greater than current value.
    pub fn split(&mut self, amount: u64) -> Self {
        assert!(amount <= self.value, "split amount exceeds coin value");
        self.value = self.value - amount;
        Self { value: amount }
    }

    /// Join another coin into this one (adds value)
    pub fn join(&mut self, other: CoinRecord) {
        self.value = self.value + other.value;
    }
}

/// Currency metadata structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CurrencyMetadata {
    pub symbol: Vec<u8>,
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub decimals: u8,
    pub icon_url: Option<Vec<u8>>,
}

impl CurrencyMetadata {
    /// Create new metadata
    pub fn new(
        symbol: Vec<u8>,
        name: Vec<u8>,
        description: Vec<u8>,
        decimals: u8,
        icon_url: Option<Vec<u8>>,
    ) -> Self {
        Self {
            symbol,
            name,
            description,
            decimals,
            icon_url,
        }
    }

    /// Get symbol as string
    pub fn symbol_str(&self) -> Result<String> {
        String::from_utf8(self.symbol.clone()).context("Invalid UTF-8 in symbol")
    }

    /// Get name as string
    pub fn name_str(&self) -> Result<String> {
        String::from_utf8(self.name.clone()).context("Invalid UTF-8 in name")
    }
}

/// Treasury capability (tracks total supply and acts as mint/burn authority)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TreasuryCap {
    pub total_supply: u64,
}

impl TreasuryCap {
    /// Create a new treasury cap with zero supply
    pub fn new() -> Self {
        Self { total_supply: 0 }
    }

    /// Get total supply tracked by this cap
    pub fn total_supply(&self) -> u64 {
        self.total_supply
    }

    /// Mint new coins, increasing the tracked total supply and returning a CoinRecord
    pub fn mint(&mut self, amount: u64) -> CoinRecord {
        assert!(amount > 0, "zero amount");
        let new_total = self.total_supply.checked_add(amount).expect("overflow");
        self.total_supply = new_total;
        CoinRecord::new(amount)
    }

    /// Burn coins and decrease tracked total supply. Returns burned value.
    pub fn burn(&mut self, coin: CoinRecord) -> u64 {
        let value = coin.burn();
        assert!(self.total_supply >= value, "underflow");
        self.total_supply = self.total_supply - value;
        value
    }
}

/// Coin module constants and utilities
pub struct CoinModule;

impl CoinModule {
    pub const COIN_MODULE: &'static str = "coin";

    /// Get the module ID for kanari_system::coin
    pub fn get_module_id() -> Result<ModuleId> {
        let address = AccountAddress::from_hex_literal(Address::KANARI_SYSTEM_ADDRESS)
            .context("Invalid system address")?;

        let module_name = Identifier::new(Self::COIN_MODULE).context("Invalid coin module name")?;

        Ok(ModuleId::new(address, module_name))
    }

    /// Get function names used in coin module
    pub fn function_names() -> CoinFunctions {
        CoinFunctions {
            create_currency: "create_currency",
            mint: "mint",
            mint_and_transfer: "mint_and_transfer",
            from_balance: "from_balance",
            burn: "burn",
            total_supply: "total_supply",
            value: "value",
            split: "split",
            join: "join",
            treasury_into_supply: "treasury_into_supply",
            into_balance: "into_balance",
        }
    }
}

/// Coin module function names
pub struct CoinFunctions {
    pub create_currency: &'static str,
    pub mint: &'static str,
    pub mint_and_transfer: &'static str,
    pub from_balance: &'static str,
    pub burn: &'static str,
    pub total_supply: &'static str,
    pub value: &'static str,
    pub split: &'static str,
    pub join: &'static str,
    pub treasury_into_supply: &'static str,
    pub into_balance: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coin_creation() {
        let coin = CoinRecord::new(1000);
        assert_eq!(coin.value(), 1000);
    }

    #[test]
    fn test_coin_burn() {
        let coin = CoinRecord::new(500);
        let value = coin.burn();
        assert_eq!(value, 500);
    }

    #[test]
    fn test_metadata() {
        let metadata = CurrencyMetadata::new(
            b"KANARI".to_vec(),
            b"Kanari Coin".to_vec(),
            b"Native coin of Kanari".to_vec(),
            9u8,
            None,
        );
        assert_eq!(metadata.symbol_str().unwrap(), "KANARI");
        assert_eq!(metadata.name_str().unwrap(), "Kanari Coin");
        assert_eq!(metadata.decimals, 9u8);
    }
}
