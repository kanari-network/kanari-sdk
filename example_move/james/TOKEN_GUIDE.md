# JAMES Token - Usage Guide (Simple)

## How the new system works

The token system is designed to be used via CLI only:

1. **No auto mint** — publishing the module does not mint tokens automatically
2. **Use `init` function** — runs on publish to create TreasuryCap
3. **TreasuryCap is sent to the publisher** — used to mint tokens later
4. **Call `mint` via CLI** — flexible, controllable

---

## Usage Steps

### Step 1: Publish the module

```bash
cd james
kanari move publish --sender <YOUR_ADDRESS> --password <PASSWORD>
```

What happens:

- Module `james::james` is published
- `init()` runs automatically
- TreasuryCap<JAMES> is created and sent to your address
- **No tokens are minted yet**

### Step 2: Mint tokens

After publishing, use the CLI to mint tokens:

```bash
kanari move call \
  --sender <YOUR_ADDRESS> \
  --password <PASSWORD> \
  --module "<YOUR_MODULE_ADDRESS>::james" \
  --function mint \
  --args <TREASURY_CAP_OBJECT_ID> <token_amount> <recipient_address>
```

Example:

```bash
kanari move call \
  --sender 0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea \
  --password 12345678 \
  --module "0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea::james" \
  --function mint \
  --args <TREASURY_CAP_OBJECT_ID> 1000000000 0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea
```

Parameter descriptions:

- `<TREASURY_CAP_OBJECT_ID>` — Object ID of TreasuryCap (check via `kanari account get`)
- `<token_amount>` — Amount of tokens to mint, e.g. `1000000000` (1 billion JAMES)
- `<recipient_address>` — Address that will receive the tokens

### Step 3: Check balance

```bash
kanari account get --address <YOUR_ADDRESS>
```

Example output:

```json
{
  "address": "0x840512ff...",
  "balance": 999999900000,
  "sequence": 3,
  "modules": ["james"],
  "token_balances": {
    "0x840512ff...::james::JAMES": 1000000000
  }
}
```

### Step 4: Transfer tokens

```bash
kanari move call \
  --sender <YOUR_ADDRESS> \
  --password <PASSWORD> \
  --module "<YOUR_MODULE_ADDRESS>::james" \
  --function transfer_amount \
  --args <COIN_OBJECT_ID> <amount> <recipient_address>
```

---

## Why the new approach is better

| Benefit   | Description                                                          |
| --------- | -------------------------------------------------------------------- |
| Flexible  | No locked amount at deploy time, mint multiple times                 |
| Universal | Same pattern for all tokens, no need to change code often            |
| Secure    | Only TreasuryCap owner can mint, deny list supported, burn supported |

---

## Key functions

### `init(witness: JAMES, ctx: &mut TxContext)`

Runs automatically when publishing the module — creates regulated currency + metadata and sends TreasuryCap + DenyCapV2 to the publisher.

### `mint(treasury_cap: &mut TreasuryCap<JAMES>, amount: u64, recipient: address, ctx: &mut TxContext)`

Mints tokens and sends them to the recipient — requires TreasuryCap.

### `transfer_amount(c: &mut Coin<JAMES>, amount: u64, recipient: address, ctx: &mut TxContext)`

Regulated transfer — checks deny list before transferring. The shared DenyList singleton (`@0x403`) is borrowed by the framework; callers pass no deny-list argument.

### `burn(treasury_cap: &mut TreasuryCap<JAMES>, coin: Coin<JAMES>)`

Burns tokens (reduces total supply) — requires TreasuryCap.

### Deny-list management (`deny_add` / `deny_remove` / `deny_pause` / `deny_unpause`)

Deny-list writes for coins require the coin's **DenyCapV2** or legacy **DenyCap**. KariKid NFT deny-list writes use the collection's **NftCap** through `kanari_system::krc_721`; entries are stored in the framework `DenyList` under a KRC-721 namespace and keyed by collection, so one issuer cannot affect another collection or coin entries. NFT transfers and management calls take the shared `DenyList` object as an argument. Coin reads are open: `is_denied(deny_list, addr)`, `is_paused(deny_list)`.

---

## Complete workflow example

```bash
# 1. Start the node
cargo run -p kanari-node

# 2. Create an account
kanari account create --password 12345678

# 3. Publish module
cd james
kanari move publish --sender <YOUR_ADDRESS> --password 12345678

# 4. Get account info (find TreasuryCap object ID)
kanari account get --address <YOUR_ADDRESS>

# 5. Mint tokens
kanari move call \
  --sender <YOUR_ADDRESS> \
  --password 12345678 \
  --module "<YOUR_MODULE_ADDRESS>::james" \
  --function mint \
  --args <TREASURY_CAP_ID> 1000000000 <YOUR_ADDRESS>

# 6. Check balance
kanari account get --address <YOUR_ADDRESS>

# 7. Transfer tokens (deny list is resolved by the framework)
kanari move call \
  --sender <YOUR_ADDRESS> \
  --password 12345678 \
  --module "<YOUR_MODULE_ADDRESS>::james" \
  --function transfer_amount \
  --args <COIN_OBJECT_ID> 1000 <recipient_address>
```

---

## Deploying other tokens

Copy this pattern to use for other tokens:

```move
module mytoken::mytoken {
    use kanari_system::coin;
    use kanari_system::coin::{Coin, TreasuryCap};
    use kanari_system::tx_context::TxContext;
    use kanari_system::transfer;
    use std::string;
    use std::ascii;
    use std::option;

    struct MYTOKEN has drop {}

    fun init(witness: MYTOKEN, ctx: &mut TxContext) {
        let (treasury_cap, metadata) = coin::create_currency(
            witness,
            6,  // decimals
            ascii::string(b"MYT"),
            string::utf8(b"My Token"),
            string::utf8(b"Description"),
            option::none(),
            ctx
        );
        transfer::public_freeze_object(metadata);
        transfer::public_transfer(treasury_cap, tx_context::sender(ctx));
    }

    public entry fun mint(
        treasury_cap: &mut TreasuryCap<MYTOKEN>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        let coin = coin::mint(treasury_cap, amount, ctx);
        transfer::public_transfer(coin, recipient);
    }
}
```

---

## Frequently asked questions

**Where is TreasuryCap?**

```
kanari account get --address <YOUR_ADDRESS>
```

Check the objects list.

**Cannot mint?**

- You must have a TreasuryCap object
- Check that the object ID is correct
- Check that the sender is the owner of the TreasuryCap

**Balance not updating?**

- Check that the transaction succeeded
- Check events in the transaction receipt
- Restart the node if necessary
