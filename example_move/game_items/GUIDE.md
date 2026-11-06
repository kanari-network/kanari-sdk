# Game Items Example - KRC-1155 Semi-Fungible Token Guide

This example demonstrates how to build a game-item system on top of the Kanari KRC-1155 semi-fungible token standard.

## Overview

KRC-1155 is Kanari's multi-token standard (similar to ERC-1155). It allows:

- A single contract to manage multiple token types (e.g., Health Potions, Iron Swords, Gold Coins)
- Each token type has its own max supply and total supply
- Balances are tracked per-holder in `Balance` objects
- Minting requires a `TokenCap` (mint authority)
- Burning requires both `TokenCap` and the `Balance` to burn

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  game_items module                  │
├─────────────────────────────────────────────────────┤
│  ItemCatalog (shared)     │  Inventory (owned)     │
│  - name                   │  - owner               │
│  - token_type (address)   │                        │
│  - max_supply             │                        │
├─────────────────────────────────────────────────────┤
│  TokenType (shared)       │  TokenCap (owned)      │
│  - name, description      │  - mint authority      │
│  - image_url              │  - burn authority      │
│  - max_supply             │                        │
│  - total_supply           │                        │
├─────────────────────────────────────────────────────┤
│  Balance (owned)          │  Balance (owned)       │
│  - token_type             │  - token_type          │
│  - amount: 100            │  - amount: 50          │
│  - owner: Alice           │  - owner: Bob          │
└─────────────────────────────────────────────────────┘
```

## Key Concepts

### 1. Token Type (TokenType)

- Each item type is a KRC-1155 `TokenType` with its own `max_supply`
- `TokenType` is shared for discovery (read-only)
- `TokenCap` is owned by the creator and required for minting/burning

### 2. Balance (Coin)

- Each player's holdings are `Balance` objects they own directly
- `Balance` objects can be split, joined, and transferred
- Balance amount is tracked per-holder, per-token-type

### 3. TokenCap (Mint Authority)

- `TokenCap` is the sole authority to mint and burn tokens
- Must be kept secure (only the creator should hold it)
- Can be frozen or transferred to another address

### 4. Events

- `ItemMoved`: Emitted when items are transferred between players
- `ItemBurned`: Emitted when items are burned (supply reduced)
- `Minted`/`Burned`/`Transferred`: KRC-1155 standard events (auto-emitted)

## Usage Flow

### Step 1: Create Item Type

```move
let (catalog, cap) = create_item_type(
    b"Health Potion",
    b"Restores 50 HP",
    b"https://cdn.example.com/potion.png",
    1000,  // max supply
    &mut ctx,
);
```

### Step 2: Mint Items to Player

```move
mint_to_player(&mut token, &cap, recipient_address, 100, &mut ctx);
```

### Step 3: Transfer Items

```move
transfer_item(&mut balance, 20, recipient_address, &mut ctx);
```

### Step 4: Split & Join

```move
let split_off = split_item(&mut balance, 30, &mut ctx);
join_items(&mut balance, split_off);
```

### Step 5: Burn Items

```move
burn_item(&mut token, &cap, balance, &ctx);
```

## Read-Only Helpers

| Function                           | Description              |
| ---------------------------------- | ------------------------ |
| `item_name(catalog)`               | Get item name            |
| `item_type(catalog)`               | Get token type address   |
| `item_max_supply(catalog)`         | Get max supply           |
| `item_total_supply(token)`         | Get current total supply |
| `item_balance_amount(balance)`     | Get balance amount       |
| `item_balance_token_type(balance)` | Get balance's token type |

## Calling Functions (kanari move call)

After publishing the module, you can interact with it using `kanari move call`:

### Create Item Type

```bash
kanari move call \
  --package <PACKAGE_ADDRESS> \
  --module game_items \
  --function create_item_type \
  --args "Health Potion" "Restores 50 HP" "https://cdn.example.com/potion.png" 1000
```

### Mint Items to Player

```bash
kanari move call \
  --package <PACKAGE_ADDRESS> \
  --module game_items \
  --function mint_to_player \
  --args <TOKEN_OBJECT_ID> <TOKEN_CAP_OBJECT_ID> <RECIPIENT_ADDRESS> 100
```

### Transfer Items

```bash
kanari move call \
  --package <PACKAGE_ADDRESS> \
  --module game_items \
  --function transfer_item \
  --args <BALANCE_OBJECT_ID> 20 <RECIPIENT_ADDRESS>
```

### Burn Items

```bash
kanari move call \
  --package <PACKAGE_ADDRESS> \
  --module game_items \
  --function burn_item \
  --args <TOKEN_OBJECT_ID> <TOKEN_CAP_OBJECT_ID> <BALANCE_OBJECT_ID>
```

## Building & Testing

```bash
# Build
kanari move build

# Test
kanari move test

# Publish
kanari move publish
```

## Test Cases

### test_item_lifecycle

- Creates an item type with max supply 1000
- Verifies name and max supply

### test_mint_split_join_and_burn_conserves_supply

- Creates item type with max supply 1000
- Mints 100 potions (total supply = 100)
- Splits 30 off (70 + 30 = 100)
- Joins back (100)
- Splits 50 and burns (total supply = 50)
- Verifies conservation: 50 burned + 50 remaining = 100

## KRC-1155 Standard Events

The KRC-1155 standard automatically emits events for:

- **Minted**: When tokens are minted
- **Burned**: When tokens are burned
- **Transferred**: When tokens are transferred between addresses

These events are indexed by the Kanari indexer and can be queried via RPC:

- `kanari_getKrc1155Events` - by token type
- `kanari_getKrc1155EventsByAddress` - by address
- `kanari_getKrc1155Balances` - balances for an address

## Safety Notes

1. **TokenCap security**: Only the holder of `TokenCap` can mint/burn. Keep it secure.
2. **Balance ownership**: `Balance` objects are owned by players. Only the owner can transfer/split.
3. **Max supply enforcement**: `krc_1155::create` enforces max supply at creation time.
4. **Supply conservation**: Minting increases total supply, burning decreases it. Transfers don't change total supply.

## Related Files

- `crates/kanari-frameworks/packages/kanari-system/sources/krc/krc_1155.move` - KRC-1155 standard
- `crates/kanari-types/src/krc_1155.rs` - Rust type definitions
- `crates/kanari-indexer/src/` - Indexer for KRC events
- `crates/kanari-rpc-server/src/nft/mod.rs` - RPC endpoints for KRC events
- `kanariexplorer/app/nft/collection/[id]/page.tsx` - Explorer UI for KRC-1155
