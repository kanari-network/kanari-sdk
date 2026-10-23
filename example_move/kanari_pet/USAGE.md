# Using `kanari_pet`

The showcase game NFT uses `kanari_system::krc_721::Nft` directly, with
collection-scoped display and transfer policy, royalty, per-token attributes,
paid progression, burn, and kiosk trading.

Every mutating call below is an `entry` function, so it is reachable from
`kanari move call`. Read-only helpers use `kanari client view`.

## Units

| Thing                    | Decimals | 1 unit              |
| ------------------------ | -------- | ------------------- |
| `USD` (`0x2::usd`)       | 6        | $1.00 = `1000000`   |
| `FEE` (royalty currency) | 9        | 1.00 = `1000000000` |

Attribute strings are bytes. `[a,b]` on the CLI becomes `vector<vector<u8>>`,
which is why the entry points take raw bytes rather than `String`.

## 0. Publish

```bash
cd example_move/kanari_pet
kanari move publish
```

`init` runs automatically and creates:

- a shared KRC-721 collection display (name / description / image_url / creator)
- a shared `TransferPolicy<krc_721::Nft>` scoped to this collection, with a 5%
  royalty to you
- a shared `Stats` — upgrade economics and authoritative per-NFT levels:
  `$1.00` per level, `10` power per level, level cap `1_000`
- a shared `Collection` (supply `10_000`)
- a shared `Krc721CollectionScope` for trade-time collection validation
- owned `NftCap` and collection-scoped policy capability

Find owned objects (including `NftCap`) with:

```powershell
cargo run -p kanari -- client objects
```

Match the `Type` column to identify the `NftCap` and copy its `ID`. `Stats`
and `Collection` are shared, so they will not appear in your owned-object
list; copy their IDs from the successful publish transaction's `RPC result`
under `effects`/`object_changes`. The publish result also contains the IDs of
the owned objects.

In PowerShell, assign the IDs before using the examples below:

```powershell
$NFT_CAP = "0x..."
$COLLECTION = "0x..."
$POLICY = "0x..."
$POLICY_SCOPE = "0x..."
$STATS = "0x..."
```

## 1. Create a collection (optional)

`init` already creates one and shares it, so most deployments skip this step.
Create another only when you want a separate supply pool with its own `NftCap`:
`NftCap` is deliberately not `drop`-able — losing it would strand the remaining
supply.

```bash
kanari move call --package 0x2 --module collection --function create_collection \
  --args "My Pets" "Pet collection" "https://example.com/banner.png" "https://example.com" 1000
```

`create_collection` returns `(Collection, NftCap)`. Keep both ids.

## 2. Mint

Plain mint — no attributes:

```bash
kanari move call --package $PKG --module pet --function mint \
  --args $NFT_CAP $COLLECTION $RECIPIENT $STATS
```

With free-form attributes (`[key,key]` / `[value,value]`):

```bash
kanari move call --package $PKG --module pet --function mint_with_attributes \
  --args $NFT_CAP $COLLECTION $RECIPIENT $STATS "[breed,mood]" "[shiba,calm]"
```

The framework KRC-721 object stores each pet's name, description, image URL,
collection id, mint ordinal, and keyed attributes. Pass the artwork URL when
minting with `mint_with_metadata`:

```bash
kanari move call --package $PKG --module pet --function mint_with_metadata \
  --args $NFT_CAP $COLLECTION $RECIPIENT $STATS "My Shiba" "A friendly Shiba Inu" \
  "https://example.com/shiba.png" "[breed,mood]" "[shiba,calm]"
```

With a starting level — `power` and `rarity` are derived from it:

```bash
kanari move call --package $PKG --module pet --function mint_with_level \
  --args $NFT_CAP $COLLECTION $RECIPIENT $STATS 5
```

Attributes are validated before supply is consumed, so a bad key/value count
costs you nothing. Pet-module mint functions also reject the reserved
`level`, `power`, and `rarity` keys.

The framework emits the standard `kanari_system::krc_721::MintLog` event for
each mint, so no app-specific event module or registration call is required.

## 3. Read attributes

```bash
kanari client view --package $PKG --module pet --function level       --arg $STATS --arg $PET
kanari client view --package $PKG --module pet --function power       --arg $STATS --arg $PET
kanari client view --package $PKG --module pet --function rarity      --arg $STATS --arg $PET
kanari client view --package $PKG --module pet --function attribute_count --arg $PET
kanari client view --package $PKG --module pet --function attribute_keys  --arg $PET
```

## 4. Edit free-form attributes

```bash
kanari move call --package $PKG --module pet --function set_attribute \
  --args $PET "nickname" "Pixel"

kanari move call --package $PKG --module pet --function set_attributes \
  --args $PET "[breed,mood]" "[akita,calm]"
```

`power`, `level` and `rarity` are **reserved** by the pet-module wrappers.
Writing them through these calls aborts with `E_RESERVED_ATTRIBUTE`; the
wrapper's wholesale `set_attributes` preserves them as display metadata.
However, KRC-721 intentionally exposes generic attribute setters to every NFT
holder, so these on-token strings are not authoritative and can be changed
directly. Gameplay must use the `level`, `power`, and `rarity` view functions,
which read progression from `Stats` and derive power/rarity from the recorded
level.

## 5. Upgrade (paid, in USD)

Progression is **flat-priced**: every level costs exactly `base_cost_usd`
(`1000000` = $1.00). A full climb to the cap of 1,000 therefore costs 1,000 x $1.00,
not an escalating sum.

Quote first:

```bash
kanari client view --package $PKG --module pet --function upgrade_cost \
  --arg $STATS --arg 0
```

Then pay:

```bash
kanari move call --package $PKG --module pet --function upgrade \
  --args $PET $STATS $USD_COIN_OBJECT
```

You pass a `Coin<USD>` **object**, and only the quoted amount is taken — the
remainder stays in your coin. Every step, whatever the level, costs `1000000`.

After a successful upgrade: `level + 1`, `power = 10 * level`,
`rarity` recomputed. USD goes to `Stats.treasury`. A `Upgraded` event is
emitted carrying `usd_paid`.

Aborts with `E_INSUFFICIENT_USD` (2) if you underpay, `E_MAX_LEVEL` (1) at the
cap of 1,000.

Rarity bands: Common <5, Uncommon 5–9, Rare 10–24, Epic 25–39, Legendary 40+.

## 6. Trade

Direct transfer, no royalty:

```bash
kanari move call --package $PKG --module pet --function transfer_pet \
  --args $PET $RECIPIENT
```

Through the policy, which enforces the 5% royalty:

```bash
kanari move call --package $PKG --module pet --function trade \
  --args $POLICY $POLICY_SCOPE $PET $FEE_COIN $RECIPIENT
```

`trade` opens a collection-bound `TransferRequest<krc_721::Nft>` hot potato,
pays the royalty, and confirms. A policy for another collection cannot be used
to trade this pet. Skipping either step aborts — a buyer cannot opt out of the
fee.

Quote the royalty first with `quote_royalty --arg $POLICY --arg <paid>`.

## 7. Burn

```bash
kanari move call --package $PKG --module pet --function burn \
  --args $NFT_CAP $COLLECTION $STATS $PET
```

The `(cap, collection)` pair is required so a cap transferred away from its
collection cannot reclaim supply from an unrelated pool.

**Known behaviour worth knowing:** burning increments `remaining`, so the cap
_reads_ as having a free slot — but `consume_for_mint` bounds the monotonic
`issued_counter` instead, so the re-mint is refused. A burned pet's slot is
therefore **not** reusable. See `test_burn_returns_remaining_without_reopening_minting`.

`Stats` stores canonical levels in a table keyed by NFT address. Mint, upgrade,
and burn therefore require a mutable `Stats` input; because it is a shared
object, these operations serialize against that object. This is an explicit
correctness tradeoff for the example's shared progression state.

## 8. Sell on the Kiosk

```bash
# create a kiosk, share it, and send the owner cap in one transaction
kanari move call --package 0x2 --module kiosk --function create --args $RECIPIENT

# place an item; 0x1001 is the field name you choose, not the object's real id
kanari move call --package 0x2 --module kiosk --function place_item \
  --args $KIOSK $KIOSK_CAP 0x1001 $PET

# lock it: it can then only leave through a purchase
kanari move call --package 0x2 --module kiosk --function lock_item \
  --args $KIOSK $KIOSK_CAP 0x1001 $POLICY $PET

kanari move call --package 0x2 --module kiosk --function list_item \
  --args $KIOSK $KIOSK_CAP 0x1001 500

kanari move call --package 0x2 --module kiosk --function delist_item \
  --args $KIOSK $KIOSK_CAP 0x1001

# owner takes it back (refused while locked)
kanari move call --package 0x2 --module kiosk --function take_to \
  --args $KIOSK $KIOSK_CAP 0x1001 $RECIPIENT
```

### Buying from the CLI

`purchase` returns a `TransferRequest<T>` hot potato, and an entry function can
neither return one nor leave it unresolved. So the only honest CLI sale path is
`buy_unregulated`, which requires a policy with **zero** rules:

```bash
kanari move call --package 0x2 --module kiosk --function buy_unregulated \
  --args $KIOSK $POLICY 0x1001 $FEE_COIN $RECIPIENT
```

KRC-721 collection policies require the collection scope, which this unregulated
purchase path does not accept. Sell pets with `kanari_pet::trade`, or use a
composing module that carries the matching scope through `kiosk::purchase` and
pays the royalty before confirming.

### Exclusive listings

`reserve` returns a `PurchaseCap`, which an entry function cannot hand back, so
the CLI wrapper `reserve_item` reserves and immediately burns the cap — leaving
the listing unsellable until `unreserve` runs. From a CLI, exclusive listings are
therefore not usable; use the non-entry `reserve` / `purchase_with_cap` from a
composing module.

See `crates/kanari-frameworks/SUI_PARITY.md` for the remaining Kiosk gaps: no
escrow (payment goes to the seller inline), no kiosk-wide `PurchaseCap`, and
`ItemId.item` is the dynamic-field name rather than the child's real object
address.

## Security notes

What the chain actually enforces, and what it does not.

**Enforced**

- Collection displays and transfer policies are bound to the collection's
  `NftCap`. Each policy owner cap is additionally bound to its specific policy
  object, preventing a cap for one collection from editing another policy.
- Transfer requests are bound to the policy that created them, and scoped
  KRC-721 requests verify that the NFT belongs to the policy's collection.
- `Stats` is unforgeable. `upgrade` and `mint_with_level` take a `&mut Stats`, so
  a caller-controlled `Stats` would mean caller-controlled economics. `new_stats`
  is `#[test_only]`; only `init` builds one. Pinned by the negative-compile
  fixture `pet_stats_forgery`.
- `power`, `level` and `rarity` are reserved by the `pet` module's convenience
  setters. Framework-level KRC-721 setters remain unrestricted; progression
  logic trusts the `Stats` table, not these mutable display strings.
- `power` and `rarity` are derived from the recorded level, never accepted from
  the caller.
- The royalty is a hot potato. `TransferRequest` has no abilities, so a trade
  cannot finish without paying it.
- Supply is bounded by the monotonic `issued_counter`, not `remaining`, so a
  burn/mint cycle cannot manufacture supply.
- `burn` requires the `(cap, collection)` pair, so a transferred cap cannot
  reclaim from an unrelated pool.

**Not enforced — read before relying on it**

- More than one collection display can exist for a collection. Clients should
  resolve the display by its collection id, not only by the shared KRC-721 type.
- **Free-form attributes are holder-editable.** A holder can rename their pet's
  `breed`. Nothing in the framework reads those attributes to make a decision,
  so this is descriptive-only — but a marketplace must not treat them as
  provenance.
- A KRC-721 pet cannot be sold through `kiosk::buy_unregulated` — its scoped
  policy has a royalty. Use `trade` with the matching scope, or a composing module.
- **Level is stored as a decimal string**, parsed leniently. A malformed value
  degrades to level 0 rather than aborting; only this module writes those keys,
  so it cannot occur in practice.

## Error codes (`kanari_pet::pet`)

| Code | Meaning                                                                              |
| ---- | ------------------------------------------------------------------------------------ |
| 0    | `E_RESERVED_ATTRIBUTE` — tried to write `power`/`level`/`rarity` via the free setter |
| 1    | `E_MAX_LEVEL` — already at the level cap                                             |
| 2    | `E_INSUFFICIENT_USD` — `Coin<USD>` does not cover the quote                          |
| 3    | `E_LEVEL_OUT_OF_RANGE` — minted above the cap                                        |

Attribute mismatches surface as `kanari_system::krc_721` code 0; supply errors as
`kanari_system::collection` code 1 (no supply) or 3 (cap/collection mismatch).

## Kiosk error codes (`kanari_system::kiosk`)

| Code | Meaning                                                            |
| ---- | ------------------------------------------------------------------ |
| 0–10 | owner / locked / listed / payment / supply errors (see the module) |
| 11   | `E_PRICE_ZERO` — a listing must cost more than 0                   |
| 12   | `E_LISTING_IS_EXCLUSIVE` — reserved listing closed to the public   |
| 13   | `E_ALREADY_EXCLUSIVE` — already reserved                           |
| 14   | `E_WRONG_PURCHASE_CAP` — cap is for another kiosk or item          |
| 15   | `E_NOT_EXCLUSIVE` — listing carries no cap                         |
| 16   | `E_POLICY_HAS_RULES` — `buy_unregulated` cannot discharge rules    |
