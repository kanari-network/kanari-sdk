# Using `kanari_pet`

The Sui-parity showcase token: `Publisher` + `Display` + `TransferPolicy` +
royalty, per-token attributes, paid progression, burn, and kiosk trading.

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

`init` runs automatically and creates, all owned by the publisher:

- a shared `Display<KanariPet>` (name / description / image_url / creator)
- a shared `TransferPolicy<KanariPet>` with a 5% royalty to you
- a shared `Stats` — upgrade economics: flat `$1.00` per level, `10` power per
  level, level cap `1_000`
- a shared `Collection` (supply `10_000`) plus its `NftCap`
- a `TransferPolicyCap<KanariPet>`, the `NftCap`, and the `Publisher<KANARI_PET>`

Record the object ids from the publish output. You will need
`Stats`, `Collection`, `NftCap`, and the `FEE` `TreasuryCap`.

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
  --args $NFT_CAP $COLLECTION $RECIPIENT
```

With free-form attributes (`[key,key]` / `[value,value]`):

```bash
kanari move call --package $PKG --module pet --function mint_with_attributes \
  --args $NFT_CAP $COLLECTION $RECIPIENT "[breed,mood]" "[shiba,calm]"
```

Every pet stores its name, description, and image URL directly on the NFT.
Pass the artwork URL when minting with `mint_with_metadata`; traits remain
key/value attributes:

```bash
kanari move call --package $PKG --module pet --function mint_with_metadata \
  --args $NFT_CAP $COLLECTION $RECIPIENT "My Shiba" "A friendly Shiba Inu" \
  "https://example.com/shiba.png" "[breed,mood]" "[shiba,calm]" "001"
```

With a starting level — `power` and `rarity` are derived from it:

```bash
kanari move call --package $PKG --module pet --function mint_with_level \
  --args $NFT_CAP $COLLECTION $RECIPIENT $STATS 5
```

Attributes are validated before supply is consumed, so a bad key/value count
costs you nothing.

To show pets in the Collection Explorer, publish its `nft::MintLog` event
module once, then upgrade the existing `pet` module. The event module is
separate because `pet` is already published and a package upgrade cannot add a
new module:

```powershell
Set-Location ..\kanari_pet_nft_events
cargo r -p kanari move publish --gas-limit 1000000

Set-Location ..\kanari_pet
cargo r -p kanari move upgrade --gas-limit 10000000
```

After the upgrade, register pets minted before collection events were added.
Use the collection creator wallet and the matching `NftCap`:

```powershell
cargo r -p kanari move call --package $PKG --module pet --function register_existing_pet `
  --args $PET $NFT_CAP $COLLECTION
```

## 3. Read attributes

```bash
kanari client view --package $PKG --module pet --function level       --arg $PET
kanari client view --package $PKG --module pet --function power       --arg $PET
kanari client view --package $PKG --module pet --function rarity      --arg $PET
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

`power`, `level` and `rarity` are **reserved**. Writing them here aborts with
`E_RESERVED_ATTRIBUTE` — they only move via `upgrade`. A wholesale
`set_attributes` _preserves_ them rather than dropping them, so replacing your
traits never zeroes a paid level.

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
  --args $POLICY $PET $FEE_COIN $RECIPIENT
```

`trade` opens a `TransferRequest<KanariPet>` hot potato, pays the royalty, and
confirms. Skipping either step aborts — a buyer cannot opt out of the fee.

Quote the royalty first with `quote_royalty --arg $POLICY --arg <paid>`.

## 7. Burn

```bash
kanari move call --package $PKG --module pet --function burn \
  --args $NFT_CAP $COLLECTION $PET
```

The `(cap, collection)` pair is required so a cap transferred away from its
collection cannot reclaim supply from an unrelated pool.

**Known behaviour worth knowing:** burning increments `remaining`, so the cap
_reads_ as having a free slot — but `consume_for_mint` bounds the monotonic
`issued_counter` instead, so the re-mint is refused. A burned pet's slot is
therefore **not** reusable. See `test_burn_returns_remaining_without_reopening_minting`.

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

On a policy that _has_ rules this aborts with `E_POLICY_HAS_RULES` (16). That is
deliberate: discharging those rules means paying them, and the rules belong to
the type owner, not to the marketplace. **A `KanariPet` cannot be sold through
this path** — it has a 5% royalty. Sell pets with `kanari_pet::trade`, or write
a composing module that calls `kiosk::purchase` and pays the royalty before
confirming.

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

- **`Display<T>` and `TransferPolicy<T>` cannot be minted for your type by a
  foreign package.** Both require a `Publisher` whose witness was defined in
  T's own package (`package::assert_governs_type`, package-level binding since
  type-level binding would force `drop` onto tokens). Same-package
  cross-module use is accepted; a package publishes all of its modules
  together.
- `Stats` is unforgeable. `upgrade` and `mint_with_level` take a `&Stats`, so
  a caller-controlled `Stats` would mean caller-controlled economics. `new_stats`
  is `#[test_only]`; only `init` builds one. Pinned by the negative-compile
  fixture `pet_stats_forgery`.
- `power`, `level` and `rarity` are reserved attribute keys. The free setters
  refuse them, and a wholesale `set_attributes` preserves rather than drops
  them. Six tests cover this specifically.
- `power` and `rarity` are derived from level, never accepted from the caller.
- The royalty is a hot potato. `TransferRequest` has no abilities, so a trade
  cannot finish without paying it.
- Supply is bounded by the monotonic `issued_counter`, not `remaining`, so a
  burn/mint cycle cannot manufacture supply.
- `burn` requires the `(cap, collection)` pair, so a transferred cap cannot
  reclaim from an unrelated pool.

**Not enforced — read before relying on it**

- **More than one `Display<T>` per type is possible** inside T's package.
  Sui closes this host-side with a one-Display-per-type registry; this runtime
  has no equivalent. A wallet must not treat the first `Display` it finds for
  a type as canonical.
- **Free-form attributes are holder-editable.** A holder can rename their pet's
  `breed`. Nothing in the framework reads those attributes to make a decision,
  so this is descriptive-only — but a marketplace must not treat them as
  provenance.
- **A `KanariPet` cannot be sold through `kiosk::buy_unregulated`** — it has a
  royalty. Use `trade`, or a composing module.
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

Attribute mismatches surface as `kanari_system::nft` code 0; supply errors as
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
