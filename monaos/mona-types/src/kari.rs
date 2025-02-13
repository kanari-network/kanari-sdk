use crate::{
    balance::{Balance, Supply},
    coin::{self, Coin, TreasuryCap},
    transfer,
    tx_context::TxContext,
};

/// Error codes
#[derive(Debug, thiserror::Error)]
pub enum KariError {
    #[error("Already minted")]
    AlreadyMinted,
    #[error("Not system address")]
    NotSystemAddress,
}

/// The amount of Mist per Kari token (10^-9)
pub const MIST_PER_KARI: u64 = 1_000_000_000;

/// The total supply of Kari in whole tokens (100 Million)
pub const TOTAL_SUPPLY_KARI: u64 = 100_000_000;

/// The total supply of Kari in Mist (100 Million * 10^9)
pub const TOTAL_SUPPLY_MIST: u64 = 100_000_000_000_000_000;

/// KARI token marker
#[derive(Debug, Clone)]
pub struct KARI;

impl KARI {
    /// Register the KARI Coin to acquire its Supply.
    /// This should be called only once during genesis creation.
    pub fn new(ctx: &mut TxContext) -> Result<Balance<KARI>, KariError> {
        // Check system address
        if ctx.sender() != &[0u8; 32] {
            return Err(KariError::NotSystemAddress);
        }

        // Check epoch
        if ctx.epoch() != 0 {
            return Err(KariError::AlreadyMinted);
        }

        // Create currency
        let (treasury, metadata) = coin::create_currency(
            KARI,
            9,                          // decimals
            "KARI",                     // symbol
            "Karura Network Coin",      // name
            "",                         // description
            None,                       // logo url
            ctx,
        );

        // Freeze metadata
        transfer::Transfer::public_freeze_object(metadata)?;

        // Create initial supply
        let mut supply = coin::treasury_into_supply(treasury);
        let total_kari = supply.increase_supply(TOTAL_SUPPLY_MIST)?;
        supply.destroy();

        Ok(total_kari)
    }

    /// Transfer KARI tokens to recipient
    pub fn transfer(coin: Coin<KARI>, recipient: [u8; 32]) -> Result<(), transfer::TransferError> {
        transfer::Transfer::public_transfer(coin, recipient)
    }

    /// Burns KARI tokens, decreasing total supply
    pub fn burn(treasury_cap: &mut TreasuryCap<KARI>, coin: Coin<KARI>) -> Result<(), coin::CoinError> {
        coin::burn(treasury_cap, coin)
    }
}

