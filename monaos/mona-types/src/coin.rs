use std::marker::PhantomData;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoinError {
    #[error("Invalid decimals")]
    InvalidDecimals,
    #[error("Supply overflow")]
    SupplyOverflow,
    #[error("Insufficient balance")]
    InsufficientBalance,
}

/// Metadata for a coin type
pub struct CoinMetadata {
    pub decimals: u8,
    pub symbol: String,
    pub name: String,
    pub description: String,
    pub logo_url: Option<String>,
}

/// A coin of type `T` worth `value`. Transferable and storable
pub struct Coin<T> {
    id: u64,  // Using u64 as a simple ID for demo
    balance: Balance<T>,
    _phantom: PhantomData<T>,
}

/// Balance type to track amount of a specific coin type
pub struct Balance<T> {
    value: u64,
    _phantom: PhantomData<T>,
}

/// Treasury capability for minting and managing coin supply
pub struct TreasuryCap<T> {
    id: u64,
    total_supply: Supply<T>,
}

/// Supply tracker for a coin type
pub struct Supply<T> {
    value: u64,
    _phantom: PhantomData<T>,
}

impl<T> Coin<T> {
    /// Get the coin's value
    pub fn value(&self) -> u64 {
        self.balance.value
    }

    /// Split coin into two, with specified amount going to new coin
    pub fn split(&mut self, split_amount: u64) -> Result<Coin<T>, &'static str> {
        if split_amount > self.balance.value {
            return Err("Insufficient balance for split");
        }
        
        self.balance.value -= split_amount;
        Ok(Coin {
            id: self.id + 1, // Simple ID generation
            balance: Balance {
                value: split_amount,
                _phantom: PhantomData
            },
            _phantom: PhantomData
        })
    }

    /// Join another coin into this one
    pub fn join(&mut self, other: Coin<T>) -> Result<(), &'static str> {
        let new_value = self.balance.value.checked_add(other.balance.value)
            .ok_or("Balance overflow")?;
        
        self.balance.value = new_value;
        // other coin gets dropped here
        Ok(())
    }
}

impl<T> TreasuryCap<T> {
    /// Get total supply of the coin
    pub fn total_supply(&self) -> u64 {
        self.total_supply.value
    }

    /// Mint new coins
    pub fn mint(&mut self, amount: u64) -> Result<Coin<T>, &'static str> {
        let new_supply = self.total_supply.value.checked_add(amount)
            .ok_or("Supply overflow")?;
        
        self.total_supply.value = new_supply;
        
        Ok(Coin {
            id: 0, // Simple ID for demo
            balance: Balance {
                value: amount,
                _phantom: PhantomData
            },
            _phantom: PhantomData
        })
    }

    /// Burn coins and reduce supply
    pub fn burn(&mut self, coin: Coin<T>) -> u64 {
        self.total_supply.value -= coin.value();
        coin.value()
    }

        /// Create a new currency with given parameters
        pub fn create_currency(
            _witness: T,
            decimals: u8,
            symbol: &str,
            name: &str,
            description: &str,
            logo_url: Option<&str>,
            _ctx: &mut crate::tx_context::TxContext,
        ) -> Result<(TreasuryCap<T>, CoinMetadata), CoinError> {
            if decimals > 18 {
                return Err(CoinError::InvalidDecimals);
            }
    
            let metadata = CoinMetadata {
                decimals,
                symbol: symbol.to_string(),
                name: name.to_string(),
                description: description.to_string(),
                logo_url: logo_url.map(String::from),
            };
    
            let treasury_cap = TreasuryCap {
                id: 0, // Simple ID for demo
                total_supply: Supply {
                    value: 0,
                    _phantom: PhantomData,
                },
            };
    
            Ok((treasury_cap, metadata))
        }
    
        /// Convert treasury cap into supply
        pub fn treasury_into_supply(self) -> Supply<T> {
            self.total_supply
        }
}


impl<T> Supply<T> {
    /// Increase supply by amount
    pub fn increase_supply(&mut self, amount: u64) -> Result<Balance<T>, CoinError> {
        let new_supply = self.value.checked_add(amount)
            .ok_or(CoinError::SupplyOverflow)?;
        
        self.value = new_supply;
        Ok(Balance {
            value: amount,
            _phantom: PhantomData,
        })
    }

    /// Destroy the supply
    pub fn destroy(self) {
        // Supply is dropped here
    }
}

