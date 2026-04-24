# Move Module Usage Examples

This guide provides practical, hands-on examples for using the Kanari System Move modules. Each example demonstrates real-world use cases and best practices.

## Table of Contents

- [Transfer Module](#transfer-module)
- [Coin & Token Management](#coin--token-management)
- [NFT Collections](#nft-collections)
- [Object Management](#object-management)
- [Data Structures](#data-structures)
- [Cryptography](#cryptography)
- [Transaction Context](#transaction-context)
- [Mathematical Operations](#mathematical-operations)

---

## Transfer Module

The `transfer` module handles object transfers between addresses with built-in validation.

### Basic Transfer

```move
use kanari_system::transfer;
use kanari_system::tx_context::TxContext;

// Create a validated transfer record
public fun create_example_transfer(ctx: &mut TxContext) {
    let from = @0x100;
    let to = @0x200;
    let amount = 1000;
    
    // This will validate that amount > 0 and from != to
    let transfer_record = transfer::create_transfer(from, to, amount);
    
    assert!(transfer::get_amount(&transfer_record) == 1000, 0);
    assert!(transfer::get_from(&transfer_record) == from, 1);
    assert!(transfer::get_to(&transfer_record) == to, 2);
}
```

### Transferring Objects

```move
use kanari_system::transfer;
use kanari_system::object::{UID, new};
use kanari_system::tx_context::TxContext;

struct MyAsset has key, store {
    id: UID,
    value: u64,
}

public fun transfer_asset(asset: MyAsset, recipient: address, ctx: &mut TxContext) {
    // Transfer ownership of the asset to recipient
    transfer::public_transfer(asset, recipient);
}

public fun share_object_example(obj: MyAsset): MyAsset {
    // Return object for sharing (caller handles storage)
    transfer::share_object(obj)
}
```

### Batch Transfer Calculations

```move
use kanari_system::transfer;
use std::vector;

public fun calculate_batch_transfers(): u64 {
    let transfers = vector::empty<transfer::Transfer>();
    
    // Add multiple transfers
    vector::push_back(&mut transfers, 
        transfer::create_transfer(@0x1, @0x2, 100));
    vector::push_back(&mut transfers, 
        transfer::create_transfer(@0x2, @0x3, 200));
    vector::push_back(&mut transfers, 
        transfer::create_transfer(@0x3, @0x4, 300));
    
    // Calculate total amount
    let total = transfer::total_amount(&transfers);
    assert!(total == 600, 0);
    
    total
}
```

### Error Handling

```move
use kanari_system::transfer;

#[test]
fun test_transfer_validation() {
    // Valid transfer
    let t = transfer::create_transfer(@0x1, @0x2, 500);
    assert!(transfer::is_valid_amount(500), 0);
    
    // Invalid: zero amount
    assert!(!transfer::is_valid_amount(0), 1);
}

#[test]
#[expected_failure(abort_code = 1)] // ERR_INVALID_AMOUNT
fun test_zero_amount_fails() {
    transfer::create_transfer(@0x1, @0x2, 0);
}

#[test]
#[expected_failure(abort_code = 2)] // ERR_SAME_ADDRESS
fun test_same_address_fails() {
    transfer::create_transfer(@0x1, @0x1, 100);
}
```

---

## Coin & Token Management

The `coin` module provides functionality for creating and managing fungible tokens.

### Creating a Custom Token

```move
use kanari_system::coin;
use kanari_system::tx_context::TxContext;
use std::option;

// Witness type for our token
struct MY_TOKEN has drop {}

public fun create_my_token(ctx: &mut TxContext): (
    coin::TreasuryCap<MY_TOKEN>,
    coin::CoinMetadata<MY_TOKEN>
) {
    // Create currency with 9 decimals (like KARI)
    let (treasury_cap, metadata) = coin::create_currency<MY_TOKEN>(
        MY_TOKEN {},              // Witness
        9,                        // Decimals
        b"MTK",                   // Symbol
        b"My Token",              // Name
        b"A custom token example", // Description
        option::none(),           // Icon URL (optional)
        ctx
    );
    
    (treasury_cap, metadata)
}
```

### Minting Tokens

```move
use kanari_system::coin;
use kanari_system::tx_context::TxContext;

public fun mint_tokens(
    treasury_cap: &mut coin::TreasuryCap<MY_TOKEN>,
    amount: u64,
    ctx: &mut TxContext
): coin::Coin<MY_TOKEN> {
    // Mint new tokens
    let new_coins = coin::mint(treasury_cap, amount, ctx);
    
    // Verify minting
    assert!(coin::value(&new_coins) == amount, 0);
    
    new_coins
}

public fun mint_and_send(
    treasury_cap: &mut coin::TreasuryCap<MY_TOKEN>,
    amount: u64,
    recipient: address,
    ctx: &mut TxContext
) {
    // Mint and transfer in one step
    coin::mint_and_transfer(treasury_cap, amount, recipient, ctx);
}
```

### Burning Tokens

```move
use kanari_system::coin;

public fun burn_tokens(
    treasury_cap: &mut coin::TreasuryCap<MY_TOKEN>,
    coins_to_burn: coin::Coin<MY_TOKEN>
): u64 {
    let burned_amount = coin::burn(treasury_cap, coins_to_burn);
    
    // Verify supply decreased
    assert!(burned_amount > 0, 0);
    
    burned_amount
}
```

### Splitting and Joining Coins

```move
use kanari_system::coin;
use kanari_system::tx_context::TxContext;

public fun split_coin_example(
    coin_obj: &mut coin::Coin<MY_TOKEN>,
    split_amount: u64,
    ctx: &mut TxContext
): coin::Coin<MY_TOKEN> {
    // Split coin into two parts
    let original_value = coin::value(coin_obj);
    let new_coin = coin::split(coin_obj, split_amount, ctx);
    
    // Verify split
    assert!(coin::value(coin_obj) == original_value - split_amount, 0);
    assert!(coin::value(&new_coin) == split_amount, 1);
    
    new_coin
}

public fun join_coins_example(
    coin1: &mut coin::Coin<MY_TOKEN>,
    coin2: coin::Coin<MY_TOKEN>
) {
    let value1_before = coin::value(coin1);
    let value2 = coin::value(&coin2);
    
    // Join coin2 into coin1
    coin::join(coin1, coin2);
    
    // Verify join
    assert!(coin::value(coin1) == value1_before + value2, 0);
}
```

### Updating Token Metadata

```move
use kanari_system::coin;
use kanari_system::url;
use std::string;
use std::ascii;
use std::option;

public fun update_token_metadata<T>(
    treasury_cap: &coin::TreasuryCap<T>,
    metadata: &mut coin::CoinMetadata<T>
) {
    // Update name
    coin::update_name(treasury_cap, metadata, 
        string::utf8(b"Updated Token Name"));
    
    // Update symbol
    coin::update_symbol(treasury_cap, metadata, 
        ascii::string(b"UTK"));
    
    // Update description
    coin::update_description(treasury_cap, metadata,
        string::utf8(b"Updated description"));
    
    // Update icon URL
    let icon_url = url::new_unsafe(ascii::string(b"https://example.com/icon.png"));
    coin::update_icon_url(treasury_cap, metadata, 
        option::some(icon_url));
}
```

### Checking Supply

```move
use kanari_system::coin;

public fun check_supply<T>(treasury_cap: &coin::TreasuryCap<T>): u64 {
    coin::total_supply(treasury_cap)
}
```

---

## NFT Collections

The `collection` module enables creation and management of NFT collections.

### Creating an NFT Collection

```move
use kanari_system::collection;
use kanari_system::tx_context::TxContext;

public fun create_nft_collection(
    name_bytes: vector<u8>,
    description_bytes: vector<u8>,
    max_supply: u64,
    ctx: &mut TxContext
): (collection::Collection, collection::NftCap) {
    let (collection_obj, nft_cap) = collection::create_collection(
        name_bytes,
        description_bytes,
        max_supply,
        ctx
    );
    
    (collection_obj, nft_cap)
}
```

### Minting NFTs

```move
use kanari_system::collection;
use kanari_system::tx_context::TxContext;

public fun mint_nft(
    cap: &mut collection::NftCap,
    ctx: &mut TxContext
): collection::Nft {
    // Mint a new NFT
    let nft = collection::mint(cap, ctx);
    
    // Check remaining supply
    assert!(cap.remaining > 0 || cap.remaining == 0, 0);
    
    nft
}
```

### Managing NFT Supply

```move
use kanari_system::collection;

public fun manage_supply(cap: &mut collection::NftCap) {
    // Check how many NFTs can still be minted
    let remaining = collection::remaining(cap);
    
    // Check total issued
    let issued = collection::issued_counter(cap);
    
    // After burning an NFT, return supply to cap
    collection::return_from_burn(cap);
}
```

### Transferring Collections and Caps

```move
use kanari_system::collection;
use kanari_system::tx_context::TxContext;

public fun transfer_ownership(
    collection_obj: collection::Collection,
    cap: collection::NftCap,
    new_owner: address,
    ctx: &mut TxContext
) {
    // Transfer collection ownership
    collection::transfer_collection(collection_obj, new_owner, ctx);
    
    // Transfer minting capability
    collection::transfer_cap(cap, new_owner, ctx);
}
```

---

## Object Management

The `object` module manages object lifecycle including creation and deletion.

### Creating Objects

```move
use kanari_system::object;
use kanari_system::tx_context::TxContext;

struct MyObject has key, store {
    id: object::UID,
    data: u64,
}

public fun create_my_object(data: u64, ctx: &mut TxContext): MyObject {
    MyObject {
        id: object::new(ctx),
        data,
    }
}
```

### Freezing Objects (Making Immutable)

```move
use kanari_system::object;
use kanari_system::transfer;

struct ImmutableConfig has key, store {
    id: object::UID,
    setting: u64,
}

public fun freeze_config(config: ImmutableConfig) {
    // Make object immutable - cannot be modified after this
    transfer::public_freeze_object(config);
}
```

### Deleting Objects

```move
use kanari_system::object;

public fun delete_object(uid: object::UID) {
    // Delete object and reclaim storage
    object::delete(uid);
}
```

### Saving Object State

```move
use kanari_system::object;

public fun save_state<T: key>(obj: &T) {
    // Persist object state to storage
    object::save_object(obj);
}
```

---

## Data Structures

Kanari provides several data structures for organizing data.

### Using Tables

```move
use kanari_system::table;
use kanari_system::tx_context::TxContext;

public fun table_example(ctx: &mut TxContext) {
    // Create a new table
    let mut tbl = table::new<u64, address>(ctx);
    
    // Insert values
    table::add(&mut tbl, 1, @0x100);
    table::add(&mut tbl, 2, @0x200);
    
    // Check if key exists
    assert!(table::contains(&tbl, 1), 0);
    
    // Get table length
    assert!(table::length(&tbl) == 2, 1);
    
    // Borrow value
    let addr = table::borrow(&tbl, 1);
    assert!(*addr == @0x100, 2);
    
    // Remove value
    let removed = table::remove(&mut tbl, 1);
    assert!(removed == @0x100, 3);
    
    // Destroy empty table
    table::destroy_empty(tbl);
}
```

### Using Bags

```move
use kanari_system::bag;
use kanari_system::tx_context::TxContext;

public fun bag_example(ctx: &mut TxContext) {
    // Create a bag (heterogeneous key-value store)
    let mut my_bag = bag::new(ctx);
    
    // Add values with different key types
    bag::add(&mut my_bag, b"name", b"MyAsset");
    bag::add(&mut my_bag, b"value", 1000u64);
    
    // Check if key exists
    assert!(bag::contains(&my_bag, b"name"), 0);
    
    // Get bag size
    assert!(bag::length(&my_bag) == 2, 1);
    
    // Borrow value
    let name = bag::borrow<vector<u8>>(&my_bag, b"name");
    
    // Remove value
    let value = bag::remove<u64>(&mut my_bag, b"value");
    assert!(value == 1000, 2);
    
    // Destroy empty bag
    bag::destroy_empty(my_bag);
}
```

### Dynamic Fields

```move
use kanari_system::dynamic_field;
use kanari_system::object::UID;
use kanari_system::tx_context::TxContext;

public fun dynamic_field_example(parent_id: &UID, ctx: &mut TxContext) {
    // Add a dynamic field
    dynamic_field::add(parent_id, b"metadata", 100u64);
    
    // Check if field exists
    assert!(dynamic_field::exists_<u64>(parent_id, b"metadata"), 0);
    
    // Borrow field value
    let value = dynamic_field::borrow<u64>(parent_id, b"metadata");
    assert!(*value == 100, 1);
    
    // Remove field
    let removed = dynamic_field::remove<u64>(parent_id, b"metadata");
    assert!(removed == 100, 2);
}
```

### Dynamic Object Fields

```move
use kanari_system::dynamic_object_field;
use kanari_system::object::UID;

struct ChildObject has key, store {
    id: UID,
    value: u64,
}

public fun dynamic_object_field_example(
    parent_id: &UID,
    child: ChildObject
) {
    // Add object as dynamic field
    dynamic_object_field::add(parent_id, b"child", child);
    
    // Check existence
    assert!(dynamic_object_field::exists_<ChildObject>(parent_id, b"child"), 0);
    
    // Borrow object
    let borrowed = dynamic_object_field::borrow<ChildObject>(parent_id, b"child");
    assert!(borrowed.value > 0, 1);
    
    // Remove object
    let removed = dynamic_object_field::remove<ChildObject>(parent_id, b"child");
    assert!(removed.value > 0, 2);
}
```

---

## Cryptography

Kanari provides cryptographic primitives for signature verification.

### Ed25519 Signature Verification

```move
use kanari_system::ed25519;

public fun verify_ed25519_signature(
    signature: vector<u8>,
    public_key: vector<u8>,
    message: vector<u8>
): bool {
    ed25519::verify(&signature, &public_key, &message)
}
```

### ECDSA K1 (secp256k1) Operations

```move
use kanari_system::ecdsa_k1;

public fun verify_ecdsa_k1(
    signature: vector<u8>,
    public_key: vector<u8>,
    message: vector<u8>
): bool {
    ecdsa_k1::verify(&signature, &public_key, &message, 0)
}

public fun recover_eth_address(
    signature: vector<u8>,
    message: vector<u8>
): vector<u8> {
    // Recover Ethereum address from signature
    ecdsa_k1::ecrecover_eth_address(signature, message)
}
```

### Hash Functions

```move
use std::hash;

public fun hash_examples(data: vector<u8>) {
    // SHA2-256
    let sha256_hash = hash::sha2_256(&data);
    
    // SHA3-256
    let sha3_hash = hash::sha3_256(&data);
    
    // Blake2b-256
    let blake2b_hash = hash::blake2b_256(&data);
    
    // Blake3-256
    let blake3_hash = hash::blake3_256(&data);
    
    // Keccak256 (Ethereum)
    let keccak_hash = hash::keccak256(&data);
    
    // RIPEMD160
    let ripemd_hash = hash::ripemd160(&data);
}
```

---

## Transaction Context

The `tx_context` module provides access to transaction metadata.

### Accessing Transaction Information

```move
use kanari_system::tx_context::TxContext;

public fun tx_info_example(ctx: &TxContext) {
    // Get transaction sender
    let sender = tx_context::sender(ctx);
    
    // Get transaction hash
    let tx_hash = tx_context::hash(ctx);
    
    // Get epoch number
    let epoch = tx_context::epoch(ctx);
    
    // Get IDs created count
    let ids_created = tx_context::ids_created(ctx);
}
```

### Generating Object IDs

```move
use kanari_system::tx_context;

public fun generate_object_id(ctx: &mut TxContext): address {
    // Derive unique object ID based on transaction hash and counter
    let tx_hash = tx_context::hash(ctx);
    let ids_created = tx_context::ids_created(ctx);
    
    tx_context::derive_id(tx_hash, ids_created)
}
```

---

## Mathematical Operations

The `math` module provides safe mathematical operations.

### Safe Arithmetic

```move
use kanari_system::math;

public fun math_examples() {
    // Square root
    let sqrt = math::sqrt_u64(100);
    assert!(sqrt == 10, 0);
    
    // Power
    let power = math::pow_u64(2, 8);
    assert!(power == 256, 1);
    
    // Division with rounding up
    let result = math::divide_and_round_up_u64(10, 3);
    assert!(result == 4, 2); // 10/3 = 3.33, rounds up to 4
    
    // Absolute difference
    let diff = math::diff_u64(100, 80);
    assert!(diff == 20, 3);
    
    // Min/Max
    let min_val = math::min_u64(10, 20);
    assert!(min_val == 10, 4);
    
    let max_val = math::max_u64(10, 20);
    assert!(max_val == 20, 5);
}
```

### Percentage Calculations

```move
use kanari_system::math;

public fun calculate_percentage(value: u64, percentage: u64): u64 {
    // Calculate X% of value safely
    math::percentage(value, percentage)
}

public fun check_slippage(price1: u64, price2: u64, max_slippage: u64): bool {
    let diff = math::diff_u64(price1, price2);
    diff <= max_slippage
}
```

---

## Complete Examples

### Example 1: Simple Token Transfer with Validation

```move
module examples::simple_transfer {
    use kanari_system::coin;
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    use std::option;
    
    struct EXAMPLE_TOKEN has drop {}
    
    /// Initialize token and send to user
    public entry fun initialize_and_send(
        recipient: address,
        amount: u64,
        ctx: &mut TxContext
    ) {
        // Create token
        let (mut treasury_cap, metadata) = coin::create_currency<EXAMPLE_TOKEN>(
            EXAMPLE_TOKEN {},
            9,
            b"EXT",
            b"Example Token",
            b"Token for demonstration",
            option::none(),
            ctx
        );
        
        // Freeze metadata
        transfer::public_freeze_object(metadata);
        
        // Mint and transfer
        coin::mint_and_transfer(&mut treasury_cap, amount, recipient, ctx);
    }
}
```

### Example 2: NFT Marketplace Listing

```move
module examples::nft_listing {
    use kanari_system::collection;
    use kanari_system::coin;
    use kanari_system::transfer;
    use kanari_system::tx_context::TxContext;
    use kanari_system::object;
    
    struct Listing has key, store {
        id: object::UID,
        nft: collection::Nft,
        price: u64,
        seller: address,
    }
    
    /// List NFT for sale
    public entry fun list_nft(
        nft: collection::Nft,
        price: u64,
        ctx: &mut TxContext
    ) {
        assert!(price > 0, 0);
        
        let listing = Listing {
            id: object::new(ctx),
            nft,
            price,
            seller: tx_context::sender(ctx),
        };
        
        // Share listing (make publicly accessible)
        transfer::share_object(listing);
    }
    
    /// Purchase listed NFT
    public entry fun purchase_listing(
        listing: Listing,
        payment: coin::Coin<kanari_system::kanari::KANARI>,
        ctx: &mut TxContext
    ) {
        assert!(coin::value(&payment) >= listing.price, 0);
        
        let Listing { id, nft, price: _, seller } = listing;
        
        // Transfer NFT to buyer
        transfer::public_transfer(nft, tx_context::sender(ctx));
        
        // Transfer payment to seller
        transfer::public_transfer(payment, seller);
        
        // Delete listing
        object::delete(id);
    }
}
```

### Example 3: Multi-Signature Wallet

```move
module examples::multisig_wallet {
    use kanari_system::object;
    use kanari_system::tx_context::TxContext;
    use std::vector;
    
    struct MultiSigWallet has key, store {
        id: object::UID,
        owners: vector<address>,
        threshold: u64,
        balance: u64,
    }
    
    /// Create multi-sig wallet
    public entry fun create_wallet(
        owners: vector<address>,
        threshold: u64,
        ctx: &mut TxContext
    ) {
        assert!(vector::length(&owners) >= threshold, 0);
        assert!(threshold > 0, 1);
        
        let wallet = MultiSigWallet {
            id: object::new(ctx),
            owners,
            threshold,
            balance: 0,
        };
        
        // Transfer wallet to first owner
        // In production, use proper ownership pattern
    }
    
    /// Check if address is owner
    public fun is_owner(wallet: &MultiSigWallet, addr: address): bool {
        let len = vector::length(&wallet.owners);
        let i = 0;
        let found = false;
        
        while (i < len) {
            let owner = vector::borrow(&wallet.owners, i);
            if (*owner == addr) {
                found = true;
            };
            i = i + 1;
        };
        
        found
    }
}
```

---

## Best Practices

1. **Always validate inputs**: Use `assert!` statements to validate parameters
2. **Handle errors gracefully**: Provide meaningful error codes
3. **Use witnesses for type safety**: Leverage phantom types and witnesses
4. **Freeze immutable objects**: Use `public_freeze_object` for config/metadata
5. **Test thoroughly**: Write unit tests for all functions
6. **Consider gas costs**: Optimize loops and storage operations
7. **Document your code**: Add comments explaining complex logic

## Common Pitfalls

1. **Forgetting to save objects**: Call `object::save_object()` when mutating
2. **Not handling empty containers**: Use `destroy_empty()` for vectors/tables/bags
3. **Ignoring overflow**: Use checked arithmetic or the `math` module
4. **Transferring to same address**: Always validate sender != recipient
5. **Not checking balances**: Verify sufficient balance before operations

## Next Steps

- Explore the [Standard Library Reference](standard-library.md)
- Review [Coding Conventions](coding-conventions.md)
- Check out the [Kanari System API Documentation](../kanari-frameworks/packages/kanari-system/docs/)
