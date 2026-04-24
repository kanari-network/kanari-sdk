# DeFi Tutorial: Building a Staking Protocol

This tutorial demonstrates how to build a simple staking protocol where users can stake tokens to earn rewards.

## Overview

We'll create a staking system with these features:
- Stake tokens to earn rewards
- Time-based reward calculation
- Unstaking with rewards
- Admin controls for reward rate

## Step 1: Define Data Structures

```move
module my_project::staking {
    use kanari_system::coin;
    use kanari_system::object::{UID, new};
    use kanari_system::clock;
    use kanari_system::tx_context::TxContext;
    use std::vector;
    
    /// Staking pool configuration
    struct StakingPool has key, store {
        id: UID,
        staked_token_type: type,
        reward_token_type: type,
        total_staked: u64,
        reward_rate: u64, // Rewards per second
        last_update_time: u64,
        accumulated_rewards: u64,
    }
    
    /// Individual stake position
    struct StakePosition has key, store {
        id: UID,
        owner: address,
        staked_amount: u64,
        stake_time: u64,
        pending_rewards: u64,
        pool_id: address,
    }
    
    const E_INSUFFICIENT_BALANCE: u64 = 0;
    const E_NOT_OWNER: u64 = 1;
    const E_NO_STAKE: u64 = 2;
}
```

## Step 2: Create Staking Pool

```move
/// Initialize a new staking pool
public entry fun create_pool<T: drop, R: drop>(
    reward_rate: u64,
    ctx: &mut TxContext
) {
    let pool = StakingPool {
        id: new(ctx),
        staked_token_type: T {},
        reward_token_type: R {},
        total_staked: 0,
        reward_rate,
        last_update_time: clock::timestamp_ms(),
        accumulated_rewards: 0,
    };
    
    // Share pool so users can interact with it
    kanari_system::transfer::share_object(pool);
}
```

## Step 3: Stake Tokens

```move
use kanari_system::transfer;

/// Stake tokens into the pool
public entry fun stake<T: drop>(
    pool: &mut StakingPool,
    tokens: coin::Coin<T>,
    ctx: &mut TxContext
) {
    let amount = coin::value(&tokens);
    assert!(amount > 0, E_INSUFFICIENT_BALANCE);
    
    // Update pool rewards
    update_pool_rewards(pool);
    
    // Create stake position
    let position = StakePosition {
        id: new(ctx),
        owner: tx_context::sender(ctx),
        staked_amount: amount,
        stake_time: clock::timestamp_ms(),
        pending_rewards: 0,
        pool_id: object::id_to_address(&pool.id),
    };
    
    // Update pool state
    pool.total_staked = pool.total_staked + amount;
    kanari_system::object::save_object(pool);
    
    // Store staked tokens in pool (simplified)
    // In production, use proper vault pattern
    
    // Give position to user
    transfer::public_transfer(position, tx_context::sender(ctx));
}
```

## Step 4: Calculate Rewards

```move
/// Update pool's accumulated rewards
fun update_pool_rewards(pool: &mut StakingPool) {
    let current_time = clock::timestamp_ms();
    let time_elapsed = current_time - pool.last_update_time;
    
    if (time_elapsed > 0 && pool.total_staked > 0) {
        // Calculate new rewards
        let new_rewards = (pool.reward_rate * time_elapsed) / 1000; // Convert ms to seconds
        
        pool.accumulated_rewards = pool.accumulated_rewards + new_rewards;
        pool.last_update_time = current_time;
        
        kanari_system::object::save_object(pool);
    }
}

/// Calculate pending rewards for a position
fun calculate_position_rewards(
    pool: &StakingPool,
    position: &StakePosition
): u64 {
    if (position.staked_amount == 0) {
        return 0;
    }
    
    let current_time = clock::timestamp_ms();
    let time_staked = current_time - position.stake_time;
    
    // Simple reward calculation: proportional to stake
    let reward_share = (position.staked_amount * pool.accumulated_rewards) 
                      / pool.total_staked;
    
    reward_share + position.pending_rewards
}
```

## Step 5: Unstake and Claim Rewards

```move
/// Unstake tokens and claim rewards
public entry fun unstake<T: drop, R: drop>(
    pool: &mut StakingPool,
    position: StakePosition,
    ctx: &mut TxContext
) {
    assert!(position.owner == tx_context::sender(ctx), E_NOT_OWNER);
    assert!(position.staked_amount > 0, E_NO_STAKE);
    
    // Update pool
    update_pool_rewards(pool);
    
    // Calculate final rewards
    let rewards = calculate_position_rewards(pool, &position);
    
    // Return staked tokens (simplified)
    // In production, withdraw from vault
    
    // Distribute reward tokens
    // mint_and_transfer_rewards<R>(pool, position.owner, rewards, ctx);
    
    // Update pool total
    pool.total_staked = pool.total_staked - position.staked_amount;
    kanari_system::object::save_object(pool);
    
    // Delete position
    kanari_system::object::delete(position.id);
}

/// Claim rewards without unstaking
public entry fun claim_rewards<R: drop>(
    pool: &mut StakingPool,
    position: &mut StakePosition,
    ctx: &mut TxContext
) {
    assert!(position.owner == tx_context::sender(ctx), E_NOT_OWNER);
    
    update_pool_rewards(pool);
    
    let rewards = calculate_position_rewards(pool, position);
    assert!(rewards > 0, E_INSUFFICIENT_BALANCE);
    
    // Distribute rewards
    // mint_and_transfer_rewards<R>(pool, position.owner, rewards, ctx);
    
    // Reset pending rewards
    position.pending_rewards = 0;
    position.stake_time = clock::timestamp_ms();
}
```

## Step 6: Admin Functions

```move
/// Update reward rate (admin only)
public entry fun update_reward_rate(
    pool: &mut StakingPool,
    admin_cap: &AdminCap,
    new_rate: u64
) {
    // Verify admin
    assert!(admin_cap.is_admin, 0);
    
    update_pool_rewards(pool);
    pool.reward_rate = new_rate;
    kanari_system::object::save_object(pool);
}

/// Emergency withdraw (admin only)
public entry fun emergency_withdraw<T: drop>(
    pool: &mut StakingPool,
    admin_cap: &AdminCap,
    amount: u64,
    ctx: &mut TxContext
) {
    assert!(admin_cap.is_admin, 0);
    
    // Withdraw tokens from pool
    // Implementation depends on vault design
}

struct AdminCap has key, store {
    id: UID,
    is_admin: bool,
}
```

## Complete Staking Module

Here's a simplified but functional version:

```move
module examples::simple_staking {
    use kanari_system::coin;
    use kanari_system::object::{UID, new};
    use kanari_system::clock;
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    
    struct STAKE_TOKEN has drop {}
    struct REWARD_TOKEN has drop {}
    
    /// Simplified staking contract
    struct StakingContract has key, store {
        id: UID,
        total_staked: u64,
        reward_per_token: u64,
        last_update: u64,
    }
    
    struct UserStake has key, store {
        id: UID,
        owner: address,
        amount: u64,
        reward_debt: u64,
    }
    
    /// Initialize staking
    public entry fun init(ctx: &mut TxContext) {
        let contract = StakingContract {
            id: new(ctx),
            total_staked: 0,
            reward_per_token: 0,
            last_update: clock::timestamp_ms(),
        };
        
        transfer::share_object(contract);
    }
    
    /// Stake tokens
    public entry fun stake(
        contract: &mut StakingContract,
        tokens: coin::Coin<STAKE_TOKEN>,
        ctx: &mut TxContext
    ) {
        let amount = coin::value(&tokens);
        assert!(amount > 0, 0);
        
        update_rewards(contract);
        
        let stake = UserStake {
            id: new(ctx),
            owner: tx_context::sender(ctx),
            amount,
            reward_debt: 0,
        };
        
        contract.total_staked += amount;
        kanari_system::object::save_object(contract);
        
        transfer::public_transfer(stake, tx_context::sender(ctx));
    }
    
    /// Unstake and claim
    public entry fun unstake(
        contract: &mut StakingContract,
        stake: UserStake,
        ctx: &mut TxContext
    ) {
        update_rewards(contract);
        
        let rewards = calculate_rewards(contract, &stake);
        
        contract.total_staked -= stake.amount;
        kanari_system::object::save_object(contract);
        
        // Distribute rewards here
        
        kanari_system::object::delete(stake.id);
    }
    
    fun update_rewards(contract: &mut StakingContract) {
        let now = clock::timestamp_ms();
        let elapsed = now - contract.last_update;
        
        if (elapsed > 0 && contract.total_staked > 0) {
            contract.reward_per_token += elapsed * 100 / contract.total_staked;
            contract.last_update = now;
            kanari_system::object::save_object(contract);
        }
    }
    
    fun calculate_rewards(
        contract: &StakingContract,
        stake: &UserStake
    ): u64 {
        (stake.amount * contract.reward_per_token) / 100 - stake.reward_debt
    }
}
```

## Testing the Staking Protocol

```move
#[test]
fun test_staking_flow() {
    use kanari_system::tx_context;
    use kanari_system::coin;
    
    let ctx = &mut tx_context::dummy();
    
    // Initialize
    init(ctx);
    
    // Create test tokens
    // Note: This requires setting up test coins
    
    // Stake tokens
    // Verify stake position created
    
    // Advance time (mock clock)
    // Calculate expected rewards
    
    // Unstake and verify rewards received
}
```

## Advanced Features

### Feature 1: Multiple Pools

```move
struct PoolRegistry has key, store {
    id: UID,
    pools: vector<address>,
}

public entry fun create_new_pool<T: drop, R: drop>(
    registry: &mut PoolRegistry,
    reward_rate: u64,
    ctx: &mut TxContext
) {
    // Create pool
    // Add to registry
    vector::push_back(&mut registry.pools, pool_address);
}
```

### Feature 2: Lock Periods

```move
struct LockedStake has key, store {
    id: UID,
    owner: address,
    amount: u64,
    unlock_time: u64,
    boost_multiplier: u64,
}

public entry fun unstake_locked(
    stake: LockedStake,
    ctx: &mut TxContext
) {
    assert!(clock::timestamp_ms() >= stake.unlock_time, 0);
    
    // Higher rewards for longer locks
    let rewards = calculate_boosted_rewards(&stake);
}
```

### Feature 3: Auto-Compounding

```move
public entry fun compound_rewards(
    contract: &mut StakingContract,
    stake: &mut UserStake,
    ctx: &mut TxContext
) {
    let rewards = calculate_rewards(contract, stake);
    
    // Automatically restake rewards
    stake.amount += rewards;
    stake.reward_debt = stake.amount * contract.reward_per_token / 100;
    
    contract.total_staked += rewards;
    kanari_system::object::save_object(contract);
}
```

## Security Considerations

1. **Reentrancy Protection**: Use checks-effects-interactions pattern
2. **Overflow Checks**: Always validate arithmetic operations
3. **Access Control**: Restrict admin functions properly
4. **Reward Manipulation**: Prevent flash loan attacks
5. **Emergency Pauses**: Include pause mechanisms
6. **Auditing**: Get professional security audits

## Gas Optimization Tips

1. **Batch Operations**: Process multiple stakes in one transaction
2. **Lazy Updates**: Calculate rewards on-demand, not continuously
3. **Efficient Storage**: Minimize on-chain data
4. **Avoid Loops**: Use constant-time operations when possible

## Next Steps

- Explore [Advanced DeFi Patterns](usage-examples.md#complete-examples)
- Learn about [Security Best Practices](coding-conventions.md)
- Study [Token Economics](creating-coins.md)

## Resources

- [Clock Module](../../crates/kanari-frameworks/packages/kanari-system/docs/clock.md)
- [Coin Module](../../crates/kanari-frameworks/packages/kanari-system/docs/coin.md)
- [Object Management](../../crates/kanari-frameworks/packages/kanari-system/docs/object.md)
