# Sui parity plan for the Kanari NFT stack

Kanari's NFT layer should track `sui-framework`'s primitives rather than
invent its own. This records the gap, the target architecture, and the order
the work lands in. Companion doc: `NFT_STANDARDS.md` (ERC-721/1155 analogues),
which stays as the _semantic_ reference for what each standard must express.

## Current gap

Status as of the Kiosk landing. "present" means the primitive exists in
`kanari-system` with tests; it does **not** mean every Sui feature came with it.

| Sui primitive                                                    | Purpose                                           | Kanari                                                  |
| ---------------------------------------------------------------- | ------------------------------------------------- | ------------------------------------------------------- |
| `sui::package::Publisher`                                        | Owns a type; gate for everything type-scoped      | **present** (`package.move`)                            |
| `sui::display::Display<T>`                                       | Type-keyed metadata for any type                  | **present** (`display.move`)                            |
| `sui::policy`                                                    | Rule registration, receipts, balances             | **present**, merged into `transfer_policy.move`         |
| `sui::transfer_policy`                                           | `TransferRequest<T>` hot potato; creator approval | **present**                                             |
| royalty rule (bps + min)                                         | Percentage/floor fee on trade                     | **present** (`royalty_rule.move`)                       |
| `sui::kiosk`                                                     | Marketplace primitive, `Kiosk` / `KioskOwnerCap`  | **present** (`kiosk.move`), no `PurchaseCap` escrow     |
| `sui::kiosk::PurchaseCap`                                        | Reserve a listing for one buyer                   | **present** (`reserve` / `purchase_with_cap`)           |
| `sui::nft::Nft`                                                  | Minimal generic NFT                               | **present** (`nft.move`)                                |
| `sui::collection`                                                | Display-only collection object                    | exists, but carries supply accounting Sui does not have |
| `sui::package::claim`                                            | OTW -> Publisher                                  | `types::claim_witness` (different purpose)              |
| `dynamic_field`, `deny_list`, `event`, `vec_map`, `bag`, `table` | shared infra                                      | present                                                 |
| `sui::kiosk_rule::personal_witness`, `deny_list_rule`            | extra rules                                       | absent                                                  |

## Target architecture

```
kanari_system::package        Publisher<T>                        (landed)
kanari_system::display        Display<T>                          (landed)
kanari_system::transfer_policy TransferPolicy<T>, TransferRequest<T>  (landed)
kanari_system::royalty_rule   percentage + min fee                (landed)
kanari_system::nft            Nft (minimal generic)               (landed)
kanari_system::kiosk          Kiosk, PurchaseCap                  (landed)
kanari_system::collection     existing                            (kept, see below)
```

`policy.move` and `transfer_policy.move` were specified as two modules and
shipped as one. Legacy Move gives a friend no way to read another module's
private fields, and a friend-only registry in `policy` that `transfer_policy`
must inspect creates exactly that deadlock plus a mutual `use` cycle. Merging
them keeps the same rule/receipt model with no private-field reach-around.

### `Publisher<T>` — the root

Sui's is created from the one-time witness in `init`. Kanari already has the
same pattern in `coin::create_regulated_currency<T: drop>(witness: T, ...)`:
passing the witness **as the type parameter** pins `T` to the witness's own
type, and only the defining module can construct that type. That is the whole
authorization argument, and it needs no host support.

**This plan originally proposed binding `&Publisher<T>` in every consumer, so a
module could not configure a type it does not own. That was reversed during
implementation, for a reason worth keeping.** `package::new<T: drop>` requires
the witness type to be droppable. Binding the publisher to the _token_ type
would therefore force `drop` onto every token, and a droppable token can be
silently discarded by whoever holds it — which bypasses supply accounting
outright, since the destroyed unit never returns to the cap. Keeping `drop` off
tokens is worth more than the tighter signature.

The consequence is Sui's own: consumers take an **unbound** `&Publisher<W>`,
so holding a publisher proves you own _some_ type in your own package, not the
specific `T` being configured. Sui closes that with a host-side one-Display-per-
type registry, which this runtime still lacks — two Displays for one type inside
T's own package remain possible.

What a runtime change would have been needed for is closed now, at Move level:
type names carry the defining package's address (`0x43::pet::KanariPet`), and
`package::assert_governs_type` — enforced by `display` and `transfer_policy` on
every mutation and minting path — aborts a publisher whose witness was defined
in a different package. A foreign package cannot mint a `Display<T>` or
`TransferPolicy<T>` for your `T` with its own publisher. The comparison accepts
same-package cross-module use: correct, because a package is the unit that
publishes and owns all of its modules together. Verified in-package by tests
with a hand-built foreign `type_name` (forgeable only through
`forged_publisher_for_test`; the production `new` derives `type_name` from the
compiler-resolved witness type, so a real foreign claim always carries the
foreign package address).

### `Display<T>` — type-keyed metadata

The important departure from the ERC-oriented design in `NFT_STANDARDS.md`.
Sui does not store name/image on each token object; it stores a
`VecMap<String, String>` on a single shared `Display<T>` keyed by the type's
name. Per-token uniqueness comes from the object id. This is what makes
wallets able to render any object type without per-object metadata.

So Kanari's `Display<T>` is keyed by `type_name::get<T>()`, and james'
`KariKid` should get a `Display` instead of carrying `name` / `image_url` /
`description` on every token. That is a real migration cost, recorded below.

### `TransferPolicy<T>` + `TransferRequest<T>`

`TransferRequest<T>` is a **hot potato**: a no-`store`, no-`drop` value. If a
transaction creates one and never confirms it, the transaction cannot finish —
there is no way to drop it. That is the enforcement mechanism, and it is why
this primitive is safe by construction rather than by convention.

Kiosk trades become impossible for a type without a shared `TransferPolicy`,
which is how creators disable trading for their own type without freezing
transfers entirely.

### Royalty as a rule, not a field

Royalty is not metadata on the token. It is a `Policy` added to a
`TransferPolicy`, so it applies to every trade of that type, can be added or
removed by the type owner at any time, and takes effect immediately across all
instances. `NFT_STANDARDS.md` treats royalty as out of scope for that reason;
with `policy` in place it becomes a small module.

## What stays different from Sui, deliberately

**Supply accounting in `collection`.** Sui's `Collection` is a display-only
object; `max_supply` is the creator's own module logic, usually enforced with
a `TreasuryCap`-shaped counter. Kanari keeps `max_supply` + `NftCap` in the
framework. It was just hardened (`consume_for_mint` now bounds the monotonic
`issued_counter` against `max_supply`, because `return_from_burn` can raise
`remaining` without a proof of burn). Keeping it is a Kanari extension, not a
divergence to fix — but it must not be confused with the Sui contract, and
`create_collection` should document that it is not `sui::collection`.

**The deny-list story.** Sui's `deny_list` is coin-only and the
`public(friend)` hardening just completed here matches that. Object families
own their own blocklist (`james::kdeny`) because Move cannot bind a framework
capability to an arbitrary package's type. Sui has the same constraint and
solves it by not offering object-family deny lists at all.

The checked regulated paths (`pay::split_and_transfer_checked`,
`pay::join_vec_and_transfer_checked`,
`multisig::execute_transfer_checked`/`execute_checked_entry`) now enforce the
list in **both directions**: a denied address cannot appear as the sender, not
only as the recipient — matching Sui's semantics where the list restricts
sending as well as receiving, so a denied holder cannot drain their balance
through a checked path. The pause flag is checked on every one of them
(fail-closed).

The unchecked `coin::*` / `pay::split_and_transfer` family is now closed at
the runtime level, mirroring Sui's choice to enforce deny lists in the object
runtime. Two host-side gates run around every entry execution: an input gate
rejects transactions whose sender is denied (or whose coin type is globally
paused) the moment a `Coin<T>` enters as an object input, and an output gate
runs just before the first store write in the success path, scanning the
assembled changeset plus the drained transfer list for any `Coin<T>` object
whose final owner is an address — a denied recipient cannot receive through
_any_ entry path (plain `coin::split` + `public_transfer`,
`pay::split_and_transfer`, redirecting game rewards), and a paused type
cannot move anywhere at all. Both gates execute the framework's own
`deny_list_contains`/`deny_list_paused` reads in a fresh session, so the
deny-key rendering can never drift from the framework. Both gates are
**fail-closed**: a missing, corrupt, or otherwise unreadable deny singleton
rejects the transaction rather than waving it through, so deny state that
cannot be verified can never be used to launder a coin transfer — pinned by
`deny_policy_rejects_missing_singleton` and
`deny_policy_rejects_corrupt_singleton` in `tests/unit/move_runtime_tests.rs`.
Destination-side enforcement is pinned by
`move-execution/v2/kanari-move-runtime-v2/tests/deny_list_recipient_enforcement.rs`
(denied recipient rejected and state untouched, allowed recipient passes, pausing
blocks everyone, unpausing restores). The gates' inner policy read runs on a
fixed budget (`DENY_POLICY_GAS_BUDGET` in `entry.rs`), decoupled from the
transaction's own gas allowance — `tests/deny_list_batch_capacity.rs` pins it by
populating the list to 64 entries and proving an allowed transfer still passes
(full-list `contains` scan) and a denied transfer is still rejected. Royalty
probing closed:
`royalty_rule::pay` is pinned by tests for the full-flow split, the
E_INSUFFICIENT abort, and the duplicate receipt abort (`vec_set::insert`
code 0) that stops one request from being charged twice.

**Mutable declared object inputs are sender-only.** A vulnerability probe
found that a transaction could declare another user's owned object as a
`mutable` explicit `AddressOwner` input: the engine validated only that the
declared owner matched the stored owner, and the runtime preloader grants
mutable-borrow authority for cross-owner non-coin objects, which would have
handed the sender a `&mut` alias to foreign app state (client-side CLI
ownership checks are not a security boundary). Now closed at the platform
chokepoint: `engine::validate_transaction_object_access` rejects any mutable
declared input whose stored owner is not the transaction sender. Shared
objects keep Sui-style semantics (any sender may mutably borrow a canonical
shared object). Regression tests live in
`crates/kanari-core/tests/unit/engine_mutable_input_tests.rs`.

**Balance destruction is paired with the supply ledger.** Sui has no
`balance::destroy` because you cannot reach a foreign balance value at all.
Kanari's legacy `balance` exposes its value as a `store` field, which made a
bare `destroy<T>(balance: Balance<T>)` reachable from any package: unpaired
destruction would let value leave circulation without the supply ledger's
paired burn bookkeeping, desyncing every cap-checked invariant downstream.
Now closed the same way Sui closes minting — `friend`-based capability:
`kanari_system::coin` is the only module allowed to destroy, and `coin::burn`
pairs the destruction with `decrease_supply` under `TreasuryCap` before
locking it. `create`/`increase`/`decrease`/`destroy` are all
`public(friend)`; `destroy_zero` is the public zero-value exit that tests
and UX flows may use, `balance::burn(supply, balance)` is the paired public
path for DEX-style LP burning, and the unit-test sink is the test-only
`destroy_for_testing`. The vault audit story (a balance can be emptied but
its slot cannot vanish silently) holds because emptying now goes through
`decrease`, which is friend-gated too.

**Kiosk: personal witnesses and the kiosk rule shipped.** The last
functional Sui-parity items:

- `KioskOwnerCap<phantom K: key>` — the cap carries the personal-kiosk type
  parameter Sui uses; `kiosk::new<K>` brands a cap family with the owner's
  package key type, so an extension package can bind its entry points to
  exactly one kiosk family by naming `K`.
- `kiosk::PersonalWitness` + `issue_personal_witness<K>(cap)` — proves cap
  possession to extension flows without exposing the cap, matching
  `sui::kiosk`.
- `kiosk_rule.move` — `sui::kiosk_rule` parity with the same adversarial
  model: once `add_rule` registers the rule on `T`'s policy,
  `transfer_policy::confirm_request` aborts unless the rule's receipt is on
  the request, and the receipt is mintable **only** inside
  `kiosk::settle` — `kiosk_rule::kiosk_receipt` is `public(friend)` whose
  sole friend is the kiosk module, so a trade that bypasses the kiosk cannot
  confirm. The kiosk is trustworthy here because `purchase`/
  `purchase_with_cap` receive the live `Kiosk` object (`&mut`) whose identity
  the runtime enforces; the receipt is attested by the kiosk module, not by
  the buyer.

## Migration cost, in order

All six items are done. What actually shipped, and what each one cost:

1. **`package.move` + `display.move`** — additive. No existing code changed;
   new modules are boot-compatible.

2. **`transfer_policy.move`** (policy merged in) — additive. The hot potato is
   self-contained: a type without a policy is unaffected.

3. **`royalty_rule.move`** — additive.

4. **james stays a legacy example; the new stack ships as a fresh package.**
   `KariKid` carries `name`, `description`, `image_url`, `attribute_keys`,
   `attribute_values` per token. Under `Display` those become type-level, and
   the options were: keep the fields and add a `Display` (two sources of truth,
   not acceptable), delete the fields (breaks tokens already on chain), or leave
   james alone. The recommended option won — `example_move/kanari_pet` is the
   Sui-shaped showcase, wired to Publisher + Display + policy + 5% royalty in
   one `init`, and its tests are the end-to-end proof. **james has not been
   migrated**; it remains a deliberate legacy reference.

5. **`nft.move` generic `Nft`** — landed as a convenience type, matching Sui's
   "provided mostly for convenience" framing.

6. **`kiosk.move`** — landed with `Kiosk`, `KioskOwnerCap`, `ItemId`, and
   `PurchaseCap`. Items are held as dynamic object fields, `lock` requires a
   configured policy so a locked item can never become unsellable, and
   `purchase` returns the `TransferRequest<T>` hot potato rather than settling
   it — rules belong to the type owner, not the seller.

### Kiosk departures from Sui, on the record

- **No payment escrow.** `purchase` splits the asking price and transfers it to
  the seller inline; the policy holds no balance. `record_profit` / `profit_of`
  exist for marketplaces that batch payouts, but nothing yet moves that ledger.
- **`ItemId.item` is the dynamic-field name, not the child object id.** The
  caller supplies the id at `place` because Move cannot read a generic `T`'s
  fields to extract its own. It is the handle key throughout. `settle` however
  reads the child object's **real** address via `dof::id` (the stored child's
  leading-32-byte UID) and puts _that_ on `TransferRequest.item`, so policy
  rules inspect the actual object id, not the field name —
  `test_transfer_request_carries_the_items_real_object_id` pins it. The only
  residual mismatch with Sui is that Sui's kiosk notifies of _the same_
  address in both places because its items are merged-domain objects.
- **A lockout window on every locked listing.** Sui's kiosk can be deadlocked
  by a malicious `lock_with_policy`: the item is locked forever and the policy
  is never collected, so the seller can never `delist`. Kanari adds
  `list_with_lockout` / `lock_with_lockout(ms)`: after `ms` elapses
  (`epoch_timestamp_ms` clock), `delist` is permitted again
  (`E_LOCKOUT_ACTIVE` until then). Unlike Sui's `time_lock` (a wrapper the
  marketplace must opt into before locking), this is enforced _by the kiosk
  itself_ on the existing `delist` — a guaranteed exit for the seller without
  weakening any lock semantic the buyer relies on before expiry.
- **No `PurchaseCap` for kiosk-wide purchase rights**, only per-item reservation.

## Constraints for whoever implements this

- Legacy Move edition. No `public struct`, no `public use fun`,
  no `public(package)`, no `mut` in let-patterns. `friend` **is** supported
  (verified when the deny-list hardening landed).
- `String` is `copy + drop + store` in this stdlib, so
  `VecMap<String, String>` is usable — that is what `Display` needs.
- `struct Publisher<phantom T>` works; `phantom` is already used by
  `DenyCap<phantom T>`.
- Do not add a `types::claim_*` native for these. The OTW-as-type-parameter
  argument is sufficient and adding host state would make the modules
  non-portable.

## Verification expectations

Each new module needs the same treatment the deny-list work got: a negative
fixture proving the type-ownership gate actually rejects an outside caller.
A `Display<T>` must not be creatable by a module that does not own `T`, and
`TransferPolicy` must not be confirmable without the owner's cap. Tests that
only exercise the happy path proved worthless during this work — the first
`deny_list` guard passed while the capability was still public, because the
child-process harness was misconfigured.

The Kiosk work added 18 tests on that basis: 10 of them are negative cases
(wrong owner, wrong item type, duplicate id, locked withdrawal, policy with no
rules, public purchase against a reserved listing, a cap aimed at another item,
double reservation, underpayment, and an unconfirmed hot potato). The happy-path
tests alone would have missed every one of those.

Two framework facts that only surfaced through test failures, worth recording
because both produce internal VM errors rather than clean aborts:

- **`key` types stored as dynamic object fields must be UID-first.** The
  `record_child` native reads the child's BCS and requires the leading 32 bytes
  to be an object id. A fieldless `struct ITEM has key, store {}` serializes to
  zero bytes and aborts with `INTERNAL_TYPE_ERROR`. Test doubles for stored
  items need a real `UID`.
- **Non-`drop` values must be consumed on every path, including unreachable
  ones.** A `#[expected_failure]` test still has to typecheck the code after the
  abort, so every resource built before it needs a disposal line after it.
