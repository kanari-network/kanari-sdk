module kanari_system::staking {
    use std::vector;
    use kanari_framework::tx_context::{Self, TxContext};
    use kanari_framework::balance::{Self, Balance};
    use kanari_framework::transfer;
    use kanari_framework::coin::{Self, Coin, TreasuryCap};
    use kanari_framework::kari::KARI;
    use kanari_framework::clock::{Self, Clock};

    /// Minimum staking amount (0.1 KARI = 10000000 units with 8 decimals)
    const MIN_STAKE: u64 = 10000000;
    
    /// Unbonding period in milliseconds (24 hours)
    const UNBONDING_PERIOD_MS: u64 = 86400000; // 24 hours in milliseconds

    /// Error codes
    const E_INSUFFICIENT_STAKE: u64 = 1;
    const E_NO_STAKE_FOUND: u64 = 2;
    const E_UNBONDING_IN_PROGRESS: u64 = 3;
    const E_UNBONDING_PERIOD_NOT_ENDED: u64 = 4;
    const E_ALREADY_STAKING: u64 = 5;
    const E_INVALID_AMOUNT: u64 = 6;

    /// Staking pool configuration and state
    struct StakingPool has key {
        /// Total KARI tokens staked
        balance: Balance<KARI>,
        /// Running total of staked tokens
        total_staked: u64,
        /// Number of active stakers
        staker_count: u64,
        /// Minimum stake requirement
        min_stake: u64,
    }

    /// Individual staker information
    struct StakeInfo has key {
        /// Amount of KARI tokens staked
        amount: u64,
        /// Timestamp when unbonding started (in ms)
        unbonding_start_ms: u64,
        /// Whether tokens are in unbonding period
        is_unbonding: bool,
        /// Staking start timestamp
        staking_start_ms: u64,
    }

    /// Events
    struct StakeEvent has copy, drop {
        staker: address,
        amount: u64,
        timestamp_ms: u64,
    }

    struct UnbondEvent has copy, drop {
        staker: address,
        amount: u64,
        timestamp_ms: u64,
    }

    /// Create new stake info
    public fun create_stake_info(
        amount: u64,
        clock: &Clock,
        ctx: &mut TxContext
    ): StakeInfo {
        StakeInfo {
            amount,
            unbonding_start_ms: 0,
            is_unbonding: false,
            staking_start_ms: clock::timestamp_ms(clock),
        }
    }

    /// Get stake info
    public fun get_stake_info(
        stake_info: &StakeInfo
    ): (u64, u64, bool, u64) {
        (
            stake_info.amount,
            stake_info.unbonding_start_ms,
            stake_info.is_unbonding,
            stake_info.staking_start_ms
        )
    }

    /// Update stake amount
    public fun update_stake_amount(
        stake_info: &mut StakeInfo,
        new_amount: u64
    ) {
        assert!(new_amount >= MIN_STAKE, E_INSUFFICIENT_STAKE);
        stake_info.amount = new_amount;
    }

    /// Start unbonding period
    public fun start_unbonding(
        stake_info: &mut StakeInfo,
        clock: &Clock
    ) {
        assert!(!stake_info.is_unbonding, E_UNBONDING_IN_PROGRESS);
        assert!(stake_info.amount > 0, E_NO_STAKE_FOUND);

        stake_info.is_unbonding = true;
        stake_info.unbonding_start_ms = clock::timestamp_ms(clock);
    }

    /// Check if unbonding period is complete
    public fun is_unbonding_complete(
        stake_info: &StakeInfo,
        clock: &Clock
    ): bool {
        if (!stake_info.is_unbonding) return false;
        
        let current_time = clock::timestamp_ms(clock);
        current_time >= stake_info.unbonding_start_ms + UNBONDING_PERIOD_MS
    }

    /// Reset unbonding state
    public fun reset_unbonding(stake_info: &mut StakeInfo) {
        stake_info.is_unbonding = false;
        stake_info.unbonding_start_ms = 0;
    }

    /// Get staking duration in milliseconds
    public fun get_staking_duration(
        stake_info: &StakeInfo,
        clock: &Clock
    ): u64 {
        let current_time = clock::timestamp_ms(clock);
        if (current_time < stake_info.staking_start_ms) return 0;
        current_time - stake_info.staking_start_ms
    }


}
