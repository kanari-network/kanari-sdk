# Signer

The `signer` type in Move represents transaction authorization. It's a privileged type that can only be created by the runtime when a transaction is submitted.

## Signer Basics

### What is a Signer?

A `signer` proves that the owner of an address has authorized a transaction:

```move
use kanari_system::tx_context::TxContext;

public fun get_authorized_address(ctx: &TxContext): address {
    tx_context::sender(ctx) // Returns address of signer
}
```

### Signer Properties

- Cannot be created in Move code (only by runtime)
- Cannot be copied or stored
- Must be consumed or passed along
- Proves transaction authorization

## Using Signers

### Transaction Entry Points

```move
/// Transfer tokens - requires signer authorization
public entry fun transfer_tokens(
    coins: Coin<KANARI>,
    recipient: address,
    ctx: &mut TxContext
) {
    // ctx contains the signer information
    let sender = tx_context::sender(ctx);
    
    assert!(sender != recipient, E_CANNOT_SEND_TO_SELF);
    transfer::public_transfer(coins, recipient);
}
```

### Capability Pattern

Use signers to create capabilities:

```move
struct AdminCap has key, store {
    id: UID,
}

/// Create admin capability - only callable by authorized address
public fun create_admin_cap(ctx: &mut TxContext): AdminCap {
    let sender = tx_context::sender(ctx);
    
    // Check if sender is authorized
    assert!(is_admin_address(sender), E_UNAUTHORIZED);
    
    AdminCap {
        id: object::new(ctx),
    }
}

fun is_admin_address(addr: address): bool {
    addr == @0x1 || addr == @0x2 // Example admin addresses
}
```

## Common Patterns

### Ownership Verification

```move
struct NFT has key, store {
    id: UID,
    owner: address,
}

public fun transfer_nft(
    nft: NFT,
    new_owner: address,
    ctx: &mut TxContext
) {
    let sender = tx_context::sender(ctx);
    
    // Verify current owner is authorizing transfer
    assert!(nft.owner == sender, E_NOT_OWNER);
    
    // Update ownership
    transfer::public_transfer(nft, new_owner);
}
```

### Multi-Signature Pattern

```move
struct MultiSigWallet has key, store {
    id: UID,
    owners: vector<address>,
    threshold: u64,
}

public fun execute_transaction(
    wallet: &mut MultiSigWallet,
    signatures: vector<vector<u8>>,
    ctx: &mut TxContext
) {
    let sender = tx_context::sender(ctx);
    
    // Verify sender is one of the owners
    assert!(is_owner(wallet, sender), E_NOT_OWNER);
    
    // Verify sufficient signatures
    assert!(vector::length(&signatures) >= wallet.threshold, E_INSUFFICIENT_SIGS);
    
    // Execute transaction
}

fun is_owner(wallet: &MultiSigWallet, addr: address): bool {
    vector::contains(&wallet.owners, &addr)
}
```

### Delegate Authority

```move
struct DelegatedCap has key, store {
    id: UID,
    delegator: address,
    delegatee: address,
    expires_at: u64,
}

public fun create_delegation(
    delegatee: address,
    duration_ms: u64,
    ctx: &mut TxContext
): DelegatedCap {
    let delegator = tx_context::sender(ctx);
    let expires = clock::timestamp_ms() + duration_ms;
    
    DelegatedCap {
        id: object::new(ctx),
        delegator,
        delegatee,
        expires_at: expires,
    }
}

public fun use_delegated_authority(
    cap: &DelegatedCap,
    ctx: &mut TxContext
) {
    let caller = tx_context::sender(ctx);
    
    // Verify caller is the delegatee
    assert!(cap.delegatee == caller, E_UNAUTHORIZED);
    
    // Verify not expired
    assert!(clock::timestamp_ms() <= cap.expires_at, E_EXPIRED);
    
    // Execute delegated action
}
```

## Signer in Practice

### Token Minting

```move
struct MintCap has key, store {
    id: UID,
}

public fun mint_tokens(
    cap: &mut MintCap,
    amount: u64,
    recipient: address,
    ctx: &mut TxContext
): Coin<TOKEN> {
    let sender = tx_context::sender(ctx);
    
    // Verify cap owner is authorizing
    assert!(object::owner(&cap.id) == sender, E_UNAUTHORIZED);
    
    coin::mint(cap, amount, ctx)
}
```

### Account Management

```move
struct Account has key, store {
    id: UID,
    owner: address,
    balance: u64,
}

public fun create_account(ctx: &mut TxContext): Account {
    let owner = tx_context::sender(ctx);
    
    Account {
        id: object::new(ctx),
        owner,
        balance: 0,
    }
}

public fun deposit_to_account(
    account: &mut Account,
    amount: u64,
    ctx: &mut TxContext
) {
    // Anyone can deposit
    account.balance += amount;
}

public fun withdraw_from_account(
    account: &mut Account,
    amount: u64,
    ctx: &mut TxContext
) {
    let sender = tx_context::sender(ctx);
    
    // Only owner can withdraw
    assert!(account.owner == sender, E_UNAUTHORIZED);
    assert!(account.balance >= amount, E_INSUFFICIENT_BALANCE);
    
    account.balance -= amount;
}
```

### Governance Voting

```move
struct Vote has key, store {
    id: UID,
    voter: address,
    proposal_id: u64,
    support: bool,
}

public fun cast_vote(
    proposal_id: u64,
    support: bool,
    ctx: &mut TxContext
) {
    let voter = tx_context::sender(ctx);
    
    // Create vote tied to signer
    let vote = Vote {
        id: object::new(ctx),
        voter,
        proposal_id,
        support,
    };
    
    // Store vote
    transfer::public_transfer(vote, GOVERNANCE_ADDRESS);
}
```

## Best Practices

### 1. Always Verify Authorization

```move
// Bad: No verification
public fn unprotected_function() {
    // Anyone can call
}

// Good: Verify signer
public fn protected_function(ctx: &mut TxContext) {
    let sender = tx_context::sender(ctx);
    assert!(is_authorized(sender), E_UNAUTHORIZED);
}
```

### 2. Use Signer Early

```move
public fn process_transaction(ctx: &mut TxContext) {
    // Get signer first
    let sender = tx_context::sender(ctx);
    
    // Validate before expensive operations
    assert!(is_whitelisted(sender), E_NOT_WHITELISTED);
    
    // Then proceed with logic
}
```

### 3. Don't Trust Parameters

```move
// Bad: Trusts provided address
public fn bad_pattern(provided_addr: address) {
    // Assumes provided_addr is legitimate
}

// Good: Uses actual signer
public fn good_pattern(ctx: &mut TxContext) {
    let actual_sender = tx_context::sender(ctx);
    // Uses verified sender
}
```

### 4. Document Signer Requirements

```move
/// Mints new tokens
/// 
/// # Authorization
/// Requires minting capability owned by transaction sender
/// 
/// # Arguments
/// * `cap` - Mint capability
/// * `amount` - Amount to mint
/// * `ctx` - Transaction context (contains signer)
public fun mint(
    cap: &mut MintCap,
    amount: u64,
    ctx: &mut TxContext
) {
    let sender = tx_context::sender(ctx);
    assert!(object::owner(&cap.id) == sender, E_UNAUTHORIZED);
    
    // Mint logic
}
```

## Security Considerations

### Reentrancy Protection

```move
struct SecureAccount has key, store {
    id: UID,
    owner: address,
    balance: u64,
    locked: bool,
}

public fun withdraw(
    account: &mut SecureAccount,
    amount: u64,
    ctx: &mut TxContext
) {
    let sender = tx_context::sender(ctx);
    
    assert!(account.owner == sender, E_UNAUTHORIZED);
    assert!(!account.locked, E_REENTRANCY);
    
    account.locked = true;
    account.balance -= amount;
    account.locked = false;
}
```

### Front-Running Protection

```move
public fun execute_with_deadline(
    operation: vector<u8>,
    deadline: u64,
    ctx: &mut TxContext
) {
    let current_time = clock::timestamp_ms();
    
    // Protect against stale transactions
    assert!(current_time <= deadline, E_DEADLINE_EXCEEDED);
    
    // Execute operation
}
```

### Signature Replay Protection

```move
struct UsedNonce has key, store {
    id: UID,
    nonce: u64,
}

public fun execute_signed_message(
    message: vector<u8>,
    signature: vector<u8>,
    nonce: u64,
    ctx: &mut TxContext
) {
    let sender = tx_context::sender(ctx);
    
    // Check nonce hasn't been used
    assert!(!nonce_used(sender, nonce), E_NONCE_ALREADY_USED);
    
    // Verify signature
    assert!(verify_signature(sender, &message, &signature), E_INVALID_SIGNATURE);
    
    // Mark nonce as used
    mark_nonce_used(sender, nonce, ctx);
    
    // Execute
}
```

## Testing Signers

```move
#[test]
fun test_signer_authorization() {
    let ctx = &mut tx_context::dummy();
    let sender = tx_context::sender(ctx);
    
    // Sender should be valid address
    assert!(sender != @0x0, 0);
    
    // Can use sender for operations
    let account = create_account_for(sender, ctx);
    assert!(account.owner == sender, 1);
}

#[test]
#[expected_failure(abort_code = E_UNAUTHORIZED)]
fun test_unauthorized_access() {
    let ctx = &mut tx_context::dummy();
    
    // Try to access without proper authorization
    restricted_function(ctx);
}
```

## Next Steps

- Learn about [Transaction Context](usage-examples.md#transaction-context)
- Study [Access Control Patterns](../patterns/access-control.md)
- Explore [Security Best Practices](../security/best-practices.md)
