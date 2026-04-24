# NFT Tutorial: Creating and Managing Collections

This tutorial shows you how to create NFT collections, mint NFTs, and manage them on the Kanari blockchain.

## Overview

NFTs (Non-Fungible Tokens) are unique digital assets. In Move, we use the `collection` module to create and manage NFT collections with built-in supply control and ownership tracking.

## Step 1: Create an NFT Collection

First, let's create a collection with metadata and supply limits:

```move
module my_project::my_nft {
    use kanari_system::collection;
    use kanari_system::tx_context::TxContext;
    
    /// Create a new NFT collection
    public entry fun create_collection(
        name: vector<u8>,
        description: vector<u8>,
        max_supply: u64,
        ctx: &mut TxContext
    ): (collection::Collection, collection::NftCap) {
        let (col, cap) = collection::create_collection(
            name,
            description,
            max_supply,
            ctx
        );
        
        (col, cap)
    }
}
```

Usage example:

```move
// Create collection with max 10,000 NFTs
let (my_collection, nft_cap) = create_collection(
    b"My Awesome NFTs",
    b"A collection of unique digital art",
    10000,
    ctx
);
```

## Step 2: Mint NFTs

Use the `NftCap` to mint new NFTs from your collection:

```move
public entry fun mint_nft(
    cap: &mut collection::NftCap,
    ctx: &mut TxContext
): collection::Nft {
    // Mint a new NFT
    let nft = collection::mint(cap, ctx);
    
    nft
}
```

Each call to `mint()` consumes one unit from the cap's remaining supply.

## Step 3: Check Collection Status

Monitor your collection's supply and issuance:

```move
public fun collection_info(cap: &collection::NftCap) {
    // How many NFTs can still be minted
    let remaining = collection::remaining(cap);
    
    // Total NFTs minted so far
    let issued = collection::issued_counter(cap);
    
    assert!(remaining + issued == MAX_SUPPLY, 0);
}
```

## Step 4: Transfer NFTs

Transfer NFTs between addresses:

```move
use kanari_system::transfer;

public entry fun transfer_nft(
    nft: collection::Nft,
    recipient: address
) {
    transfer::public_transfer(nft, recipient);
}
```

## Complete NFT Module

Here's a complete example with additional features:

```move
module my_project::art_collection {
    use kanari_system::collection;
    use kanari_system::transfer;
    use kanari_system::object;
    use kanari_system::tx_context::TxContext;
    
    const MAX_SUPPLY: u64 = 1000;
    
    /// Initialize the collection
    public entry fun initialize(ctx: &mut TxContext) {
        let (col, cap) = collection::create_collection(
            b"Digital Art Collection",
            b"Unique generative art pieces",
            MAX_SUPPLY,
            ctx
        );
        
        // Transfer collection object to treasury/admin
        // Keep cap for minting
        
        // Store cap securely - it controls minting!
    }
    
    /// Mint a single NFT
    public entry fun mint(
        cap: &mut collection::NftCap,
        recipient: address,
        ctx: &mut TxContext
    ) {
        assert!(collection::remaining(cap) > 0, 0);
        
        let nft = collection::mint(cap, ctx);
        transfer::public_transfer(nft, recipient);
    }
    
    /// Mint multiple NFTs at once
    public entry fun mint_batch(
        cap: &mut collection::NftCap,
        quantity: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        assert!(quantity > 0, 0);
        assert!(collection::remaining(cap) >= quantity, 1);
        
        let mut i = 0;
        while (i < quantity) {
            let nft = collection::mint(cap, ctx);
            transfer::public_transfer(nft, recipient);
            i = i + 1;
        };
    }
    
    /// Burn an NFT (return supply to cap)
    public entry fun burn_nft(
        cap: &mut collection::NftCap,
        _nft: collection::Nft
    ) {
        // Destroy the NFT
        // Return supply to cap
        collection::return_from_burn(cap);
    }
    
    /// Get collection statistics
    public fun stats(cap: &collection::NftCap): (u64, u64) {
        (collection::issued_counter(cap), collection::remaining(cap))
    }
}
```

## Advanced: NFT with Metadata

Create NFTs with associated metadata using dynamic fields:

```move
module my_project::metadata_nft {
    use kanari_system::collection;
    use kanari_system::dynamic_object_field;
    use kanari_system::object::{UID, new};
    use kanari_system::tx_context::TxContext;
    use std::string;
    
    /// NFT with custom attributes
    struct Artwork has key, store {
        id: UID,
        nft: collection::Nft,
        title: string::String,
        artist: string::String,
        year: u64,
    }
    
    /// Create artwork NFT
    public entry fun create_artwork(
        cap: &mut collection::NftCap,
        title_bytes: vector<u8>,
        artist_bytes: vector<u8>,
        year: u64,
        ctx: &mut TxContext
    ) {
        // Mint base NFT
        let nft = collection::mint(cap, ctx);
        
        // Create artwork with metadata
        let artwork = Artwork {
            id: new(ctx),
            nft,
            title: string::utf8(title_bytes),
            artist: string::utf8(artist_bytes),
            year,
        };
        
        // Store artwork - in production, transfer to owner
    }
    
    /// Get artwork metadata
    public fun get_title(artwork: &Artwork): &string::String {
        &artwork.title
    }
}
```

## NFT Marketplace Example

Simple marketplace for listing and selling NFTs:

```move
module my_project::nft_marketplace {
    use kanari_system::collection;
    use kanari_system::coin;
    use kanari_system::transfer;
    use kanari_system::object::{UID, new};
    use kanari_system::tx_context::TxContext;
    
    /// A listed NFT for sale
    struct Listing has key, store {
        id: UID,
        nft: collection::Nft,
        price: u64,
        seller: address,
        token_type: type,
    }
    
    /// List NFT for sale
    public entry fun list_nft<T: drop>(
        nft: collection::Nft,
        price: u64,
        ctx: &mut TxContext
    ) {
        assert!(price > 0, 0);
        
        let listing = Listing {
            id: new(ctx),
            nft,
            price,
            seller: tx_context::sender(ctx),
            token_type: T {},
        };
        
        // Make listing publicly accessible
        transfer::share_object(listing);
    }
    
    /// Purchase a listed NFT
    public entry fun buy_listing<T: drop>(
        listing: Listing,
        payment: coin::Coin<T>,
        ctx: &mut TxContext
    ) {
        assert!(coin::value(&payment) >= listing.price, 0);
        
        let Listing { id, nft, price: _, seller, token_type: _ } = listing;
        
        // Transfer NFT to buyer
        transfer::public_transfer(nft, tx_context::sender(ctx));
        
        // Transfer payment to seller
        transfer::public_transfer(payment, seller);
        
        // Remove listing
        object::delete(id);
    }
    
    /// Cancel listing
    public entry fun cancel_listing(listing: Listing, ctx: &mut TxContext) {
        assert!(listing.seller == tx_context::sender(ctx), 0);
        
        let Listing { id, nft, price: _, seller: _, token_type: _ } = listing;
        
        // Return NFT to seller
        transfer::public_transfer(nft, seller);
        
        // Delete listing
        object::delete(id);
    }
}
```

## Testing NFT Functions

```move
#[test]
fun test_nft_minting() {
    use kanari_system::tx_context;
    
    let ctx = &mut tx_context::dummy();
    
    // Create collection
    let (col, mut cap) = collection::create_collection(
        b"Test Collection",
        b"Testing",
        100,
        ctx
    );
    
    // Check initial state
    assert!(collection::remaining(&cap) == 100, 0);
    assert!(collection::issued_counter(&cap) == 0, 1);
    
    // Mint NFT
    let nft = collection::mint(&mut cap, ctx);
    
    // Verify state changes
    assert!(collection::remaining(&cap) == 99, 2);
    assert!(collection::issued_counter(&cap) == 1, 3);
    
    // Mint another
    let nft2 = collection::mint(&mut cap, ctx);
    assert!(collection::remaining(&cap) == 98, 4);
}

#[test]
#[expected_failure(abort_code = 0)]
fun test_mint_exceeds_supply() {
    use kanari_system::tx_context;
    
    let ctx = &mut tx_context::dummy();
    
    // Create collection with 1 NFT max
    let (_, mut cap) = collection::create_collection(
        b"Limited",
        b"One only",
        1,
        ctx
    );
    
    // Mint first NFT - OK
    let _ = collection::mint(&mut cap, ctx);
    
    // Try to mint second - should fail
    let _ = collection::mint(&mut cap, ctx);
}
```

## Common Patterns

### Pattern 1: Open Edition (Unlimited Supply)

```move
public entry fun create_open_edition(ctx: &mut TxContext) {
    // Use u64::MAX for practical unlimited supply
    let (col, cap) = collection::create_collection(
        b"Open Edition",
        b"Mint as many as you want",
        18446744073709551615, // u64::MAX
        ctx
    );
}
```

### Pattern 2: Timed Mint

```move
use kanari_system::clock;

public entry fun mint_during_window(
    cap: &mut collection::NftCap,
    start_time: u64,
    end_time: u64,
    ctx: &mut TxContext
) {
    let current_time = clock::timestamp_ms();
    assert!(current_time >= start_time, 0);
    assert!(current_time <= end_time, 1);
    
    let nft = collection::mint(cap, ctx);
    transfer::public_transfer(nft, tx_context::sender(ctx));
}
```

### Pattern 3: Whitelist Minting

```move
use std::vector;

struct Whitelist has key, store {
    id: UID,
    addresses: vector<address>,
}

public entry fun mint_whitelist(
    whitelist: &Whitelist,
    cap: &mut collection::NftCap,
    ctx: &mut TxContext
) {
    let sender = tx_context::sender(ctx);
    assert!(is_whitelisted(whitelist, sender), 0);
    
    let nft = collection::mint(cap, ctx);
    transfer::public_transfer(nft, sender);
}

fun is_whitelisted(list: &Whitelist, addr: address): bool {
    let len = vector::length(&list.addresses);
    let mut i = 0;
    let mut found = false;
    
    while (i < len) {
        if (*vector::borrow(&list.addresses, i) == addr) {
            found = true;
        };
        i = i + 1;
    };
    
    found
}
```

## Best Practices

1. **Set reasonable supply limits** - Can't increase later
2. **Secure NftCap** - Anyone with it can mint
3. **Freeze collection metadata** if it shouldn't change
4. **Validate inputs** - Check prices, quantities, etc.
5. **Handle edge cases** - Empty collections, zero supply
6. **Test thoroughly** - Especially minting limits
7. **Consider gas costs** - Batch operations when possible

## Common Errors

### Error: Not enough supply

```move
assert!(collection::remaining(cap) > 0, 0);
```

Solution: Check remaining supply before minting.

### Error: Invalid quantity

```move
assert!(quantity > 0, 0);
```

Solution: Always validate quantity is positive.

### Lost NftCap

If you lose the `NftCap`, you can't mint more NFTs. Store it securely!

## Next Steps

- Learn about [Token Standards](usage-examples.md#coin--token-management)
- Explore [Marketplace Development](usage-examples.md#complete-examples)
- Study [Security Patterns](coding-conventions.md)

## Resources

- [Collection Module Docs](../../crates/kanari-frameworks/packages/kanari-system/docs/collection.md)
- [Transfer Module](../../crates/kanari-frameworks/packages/kanari-system/docs/transfer.md)
- [Object Management](../../crates/kanari-frameworks/packages/kanari-system/docs/object.md)
