# Move Tutorial: Creating Your First Token

This tutorial walks you through creating, minting, and transferring custom tokens on the Kanari blockchain using Move.

## Prerequisites

- Basic understanding of Move syntax
- Kanari SDK installed
- A wallet with test KARI tokens for gas fees

## Step 1: Define Your Token Type

Every token in Move needs a unique type. We use a "witness" pattern to ensure uniqueness:

```move
module my_project::my_token {
    use kanari_system::coin;
    use kanari_system::tx_context::TxContext;
    use std::option;
    
    /// Witness type - proves we own this token type
    struct MY_TOKEN has drop {}
    
    // ... more code will go here
}
```

The `has drop` ability allows the witness to be consumed during currency creation.

## Step 2: Create the Currency

Now let's add a function to create our token with metadata:

```move
public entry fun create_my_token(ctx: &mut TxContext) {
    // Create the currency with metadata
    let (treasury_cap, metadata) = coin::create_currency<MY_TOKEN>(
        MY_TOKEN {},              // Consume witness
        9,                        // 9 decimals (like KARI)
        b"MTK",                   // Symbol: short identifier
        b"My Token",              // Name: full name
        b"A tutorial token",      // Description
        option::none(),           // Icon URL (optional)
        ctx
    );
    
    // Freeze metadata so it can't be changed
    kanari_system::transfer::public_freeze_object(metadata);
    
    // Keep treasury_cap to mint/burn tokens
    // In production, transfer this to a secure wallet
}
```

### Understanding Parameters

- **Decimals**: Determines precision. 9 decimals means 1 token = 1,000,000,000 base units
- **Symbol**: Short code (3-5 chars) shown in wallets
- **Name**: Full descriptive name
- **Description**: Explains what your token represents
- **Icon URL**: Link to token logo image

## Step 3: Mint Tokens

Create a function to mint new tokens:

```move
public entry fun mint_tokens(
    treasury_cap: &mut coin::TreasuryCap<MY_TOKEN>,
    amount: u64,
    recipient: address,
    ctx: &mut TxContext
) {
    // Mint and send directly to recipient
    coin::mint_and_transfer(treasury_cap, amount, recipient, ctx);
}
```

Usage example:

```move
// Mint 1000 tokens (with 9 decimals, that's 1000 * 10^9 base units)
mint_tokens(&mut treasury_cap, 1000000000000, @0x123..., ctx);
```

## Step 4: Transfer Tokens

Users can transfer tokens using the standard transfer module:

```move
use kanari_system::transfer;

public entry fun send_tokens(
    coins: coin::Coin<MY_TOKEN>,
    recipient: address
) {
    transfer::public_transfer(coins, recipient);
}
```

## Step 5: Check Balances

Query token balance:

```move
public fun get_balance(coins: &coin::Coin<MY_TOKEN>): u64 {
    coin::value(coins)
}
```

## Complete Example

Here's the complete module:

```move
module my_project::my_token {
    use kanari_system::coin;
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    use std::option;
    
    /// Witness type for MY_TOKEN
    struct MY_TOKEN has drop {}
    
    /// Initialize the token
    public entry fun initialize(ctx: &mut TxContext) {
        let (treasury_cap, metadata) = coin::create_currency<MY_TOKEN>(
            MY_TOKEN {},
            9,
            b"MTK",
            b"My Token",
            b"Created following the tutorial",
            option::none(),
            ctx
        );
        
        // Freeze metadata permanently
        transfer::public_freeze_object(metadata);
        
        // Mint initial supply to deployer
        let initial_supply = 1000000000000000; // 1 million tokens
        let coins = coin::mint(&mut treasury_cap, initial_supply, ctx);
        
        // Keep treasury_cap for future minting
        // Store it securely or transfer to governance
    }
    
    /// Mint additional tokens
    public entry fun mint_more(
        treasury_cap: &mut coin::TreasuryCap<MY_TOKEN>,
        amount: u64,
        to: address,
        ctx: &mut TxContext
    ) {
        coin::mint_and_transfer(treasury_cap, amount, to, ctx);
    }
    
    /// Burn tokens to reduce supply
    public entry fun burn_tokens(
        treasury_cap: &mut coin::TreasuryCap<MY_TOKEN>,
        coins: coin::Coin<MY_TOKEN>
    ) {
        coin::burn(treasury_cap, coins);
    }
    
    /// Split a coin into two
    public entry fun split_coin(
        coin_obj: &mut coin::Coin<MY_TOKEN>,
        amount: u64,
        ctx: &mut TxContext
    ): coin::Coin<MY_TOKEN> {
        coin::split(coin_obj, amount, ctx)
    }
    
    /// Merge two coins
    public entry fun merge_coins(
        coin1: &mut coin::Coin<MY_TOKEN>,
        coin2: coin::Coin<MY_TOKEN>
    ) {
        coin::join(coin1, coin2);
    }
}
```

## Testing Your Token

Add unit tests to verify functionality:

```move
#[test]
fun test_token_creation() {
    use kanari_system::tx_context;
    
    // Create test context
    let ctx = &mut tx_context::dummy();
    
    // Create currency
    let (treasury_cap, metadata) = coin::create_currency<MY_TOKEN>(
        MY_TOKEN {},
        9,
        b"MTK",
        b"My Token",
        b"Test token",
        option::none(),
        ctx
    );
    
    // Verify metadata
    assert!(coin::total_supply(&treasury_cap) == 0, 0);
    
    // Mint some tokens
    let coins = coin::mint(&mut treasury_cap, 1000, ctx);
    assert!(coin::value(&coins) == 1000, 1);
    
    // Burn tokens
    let burned = coin::burn(&mut treasury_cap, coins);
    assert!(burned == 1000, 2);
    assert!(coin::total_supply(&treasury_cap) == 0, 3);
}
```

## Common Patterns

### Pattern 1: Fixed Supply Token

```move
public entry fun create_fixed_supply(ctx: &mut TxContext) {
    let (mut treasury_cap, metadata) = coin::create_currency<MY_TOKEN>(
        MY_TOKEN {}, 9, b"FIX", b"Fixed Token", 
        b"Fixed supply token", option::none(), ctx
    );
    
    transfer::public_freeze_object(metadata);
    
    // Mint all tokens immediately
    let total_supply = 1000000000000000;
    let coins = coin::mint(&mut treasury_cap, total_supply, ctx);
    
    // Destroy treasury cap to prevent future minting
    // This makes supply truly fixed
    destroy_treasury_cap(treasury_cap);
}

fun destroy_treasury_cap<T>(_cap: coin::TreasuryCap<T>) {
    // Consumes the cap without using it
}
```

### Pattern 2: Mintable Token with Cap

```move
public entry fun create_capped_token(max_supply: u64, ctx: &mut TxContext) {
    let (treasury_cap, metadata) = coin::create_currency<MY_TOKEN>(
        MY_TOKEN {}, 9, b"CAP", b"Capped Token",
        b"Token with max supply", option::none(), ctx
    );
    
    transfer::public_freeze_object(metadata);
    
    // Store max_supply in a config object
    // Check before each mint operation
}

public entry fun mint_with_cap(
    treasury_cap: &mut coin::TreasuryCap<MY_TOKEN>,
    max_supply: u64,
    amount: u64,
    to: address,
    ctx: &mut TxContext
) {
    let current = coin::total_supply(treasury_cap);
    assert!(current + amount <= max_supply, 0);
    
    coin::mint_and_transfer(treasury_cap, amount, to, ctx);
}
```

### Pattern 3: Governance Token

```move
struct GovernanceToken has drop {}

public entry fun create_governance_token(ctx: &mut TxContext) {
    let (treasury_cap, metadata) = coin::create_currency<GovernanceToken>(
        GovernanceToken {}, 9, b"GOV", b"Governance Token",
        b"Used for voting", option::none(), ctx
    );
    
    // Transfer treasury to DAO/governance contract
    // Don't freeze - allow controlled minting
}
```

## Best Practices

1. **Always freeze metadata** unless you need to update it
2. **Use meaningful symbols** (3-5 uppercase letters)
3. **Choose appropriate decimals** (6-9 is common)
4. **Document your token** with clear descriptions
5. **Test thoroughly** before deploying to mainnet
6. **Secure treasury caps** - they control minting
7. **Consider tokenomics** - fixed vs inflationary supply

## Next Steps

- Learn about [NFT Collections](usage-examples.md#nft-collections)
- Explore [Advanced Token Features](usage-examples.md#coin--token-management)
- Study [Security Best Practices](coding-conventions.md)

## Troubleshooting

### Error: `EOVERFLOW`

You're trying to mint more tokens than u64 can hold. Reduce the amount.

### Error: `EZERO_AMOUNT`

Mint amount must be greater than 0.

### Error: `EINVALID_DECIMALS`

Decimals must be ≤ 27. Use 6-9 for most tokens.

### Can't modify metadata after freezing

This is intentional! Plan your metadata carefully before calling `public_freeze_object`.
