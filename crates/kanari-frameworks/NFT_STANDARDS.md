# NFT Standards: ERC-721 / ERC-1155 analogues for `kanari_system`

Design note. This defines the Kanari-native equivalents of the two Ethereum
NFT standards, expressed in the object model this runtime actually implements
(owned objects with `UID`, explicit capabilities, runtime-dispatched entry
functions).

The ERC names are _conceptual_ references only; Kanari's unique-token module is
named `krc_721`. Kanari has
no contract ABI, no `balanceOf(address, tokenId)` mapping, and no `transferFrom`
reentrancy surface, so these modules deliberately do **not** clone the Solidity
surface. They port the semantics those standards exist to express:

| Ethereum concept            | Ethereum surface             | Kanari equivalent                                  |
| --------------------------- | ---------------------------- | -------------------------------------------------- |
| ERC-721 unique token        | one token per `tokenId`      | one object per mint, unique `UID`                  |
| ERC-721 supply cap          | `_MAX_SUPPLY` in constructor | `NftCap.remaining` (already in `collection.move`)  |
| ERC-721 operator / approved | per-token `getApproved`      | **removed** — object ownership _is_ the approval   |
| ERC-1155 semi-fungible      | one `tokenId`, N copies      | one `Token` type, `u64` balance in a shared object |
| ERC-1155 batch transfer     | `safeBatchTransferFrom`      | split/merge loop over one object per token type    |
| ERC-1155 per-token URI      | `uri(uint256 id)`            | metadata stored on the token type, not per copy    |
| Pausable / denylist         | owner hooks                  | `deny_list` (framework) / issuer cap (family)      |

## Why the mapping looks like this

Ethereum models ownership as a `mapping(address => uint256 balance)` inside one
contract. Kanari models it as _object ownership_: an NFT is an object with a
`UID`, and holding the object is holding the token. Three consequences:

1. **No approve/operator needed.** "Allowing" someone to move an ERC-721 means
   transferring the object, or wrapping it. Adding an operator registry would
   reintroduce exactly the delegated-transfer surface that object ownership
   removes, and it would need its own revocation path.
2. **No `tokenId` integer.** The object's id _is_ the token id
   (`object::uid_to_inner(&uid)`). Two mints can never collide.
3. **Batch transfer is N object inputs.** A user paying for 1155 batch transfers
   already funds one transaction; the runtime charges per object touched, so
   the batching is already the optimal shape. An in-contract loop would be
   strictly worse.

## Module layout

The KRC-721 module lives alongside the existing `collection.move`; KRC-1155 is
the planned semi-fungible module:

```
kanari_system::krc_721  -- unique tokens, one object each
kanari_system::erc1155  -- semi-fungible, shared object + u64 balances
```

`collection.move` stays as-is: it already provides `Collection`, `NftCap`,
`create_collection`, `consume_for_mint`, `return_from_burn`. Both standards
build on it rather than duplicating the supply accounting. **This is the one
piece of the existing framework that already matches both standards** and
should not be redesigned.

The current `krc_721` module provides generic NFT and attribute primitives,
plus collection-scoped deny-list helpers backed by the framework
`deny_list::DenyList`. The collection-bound mint/burn lifecycle described
below remains the target design.

---

## `kanari_system::krc_721`

### Types

```move
/// One unique token. Each mint produces a distinct object with its own UID,
/// which is the token id.
struct Token has key, store {
    id: UID,
    collection: address,
    /// 0-based mint ordinal within the collection, for off-chain indexing
    /// and the "which one is this" query. Not an identity: `uid_to_inner`
    /// is the identity.
    number: u64,
    name: String,
    description: String,
    image_url: Url,
    attributes: VecMap<String, String>,
}
```

Deliberately _not_ stored: `owner` (the runtime derives it), `collection` id is
cached but must match the cap at mint time (see below), `royalties` are not in
scope.

### Lifecycle

```move
/// Mint one token. Requires `&mut NftCap` so supply is bounded by the
/// collection's `max_supply`, and requires `&Collection` so the cap's
/// `collection_id` is checked against the actual collection object.
public fun mint(
    cap: &mut NftCap,
    collection: &Collection,
    name: vector<u8>, description: vector<u8>, image_url: vector<u8>,
    attributes: (vector<vector<u8>>, vector<vector<u8>>),
    recipient: address,
    ctx: &mut TxContext,
): Token
```

The `(cap, collection)` pair is the security-relevant part of this signature.
`collection.move::return_from_burn` already requires the pair for exactly this
reason (see its `E_CAP_COLLECTION_MISMATCH`): a `NftCap` is transferable, so a
cap whose `collection_id` points at a _different_ collection than the one
passed in would let the holder mint against the wrong supply pool. `mint` must
apply the same check rather than trusting `cap.collection_id` alone.

```move
public fun burn(cap: &mut NftCap, collection: &Collection, token: Token, ctx: &mut TxContext)
```

Returns the supply unit via `collection::return_from_burn`, which re-checks the
pair. `issued_counter` is intentionally not decremented (existing behaviour:
it is a historical counter, not a live balance).

### Transfer

```move
/// Ownership transfer is `transfer::public_transfer` — no framework function
/// needed. Provided only so wallets have one obvious call site.
public fun transfer(token: Token, recipient: address, ctx: &mut TxContext)
```

### Compliance surface

ERC-721 defines `supportsInterface` / `ERC721Metadata`. The Kanari analogue is
**structural, not nominal**: an object is a 721 iff it has a `UID` and lives in
a `Collection`. Wallet-side detection is by type shape, not by an interface id.

So the framework provides read helpers and _no_ registry of "which types are
721":

```move
public fun token_id(token: &Token): ID            // object::uid_to_inner(&token.id)
public fun number(token: &Token): u64
public fun collection_of(token: &Token): address
public fun attribute(token: &Token, key: &String): Option<String>
```

Registration would be self-attested by the issuer anyway, so a registry would
carry no authority it could enforce.

---

## `kanari_system::erc1155`

### Types

```move
/// One semi-fungible token *type*. The shared object below holds balances for
/// this type; there is one object per (collection, token type) pair, so
/// token types never interfere.
struct TokenType has key, store {
    id: UID,
    collection: address,
    name: String,
    description: String,
    image_url: Url,
    decimals: u8,
}

/// Holder registry for one `TokenType`, shared so anyone can mint into it and
/// anyone can read their own balance.
struct HolderRegistry has key {
    id: UID,
    token_type: address,
    /// address -> balance. `VecMap` because addresses are `copy + drop`; this
    /// runtime keys dynamic fields by name *bytes*, so two distinct key types
    /// with the same layout would collide (see the note in `deny_list`).
    balances: VecMap<address, u64>,
    total_supply: u64,
    cap: u64,
}

/// Per-owner share of one `TokenType`, transferred instead of mutated.
struct Balance has key, store {
    id: UID,
    token_type: address,
    amount: u64,
}
```

The `HolderRegistry` + `Balance` split is the whole design. Balances live in a
shared object (so mint is permissionless within the cap, like ERC-1155's
permissionless mint) but ownership of a balance is a `Balance` object the holder
owns. Moving tokens is therefore an object transfer, not a registry mutation —
same property that makes 721 work.

### Lifecycle

```move
public fun create_token_type(
    cap: &mut NftCap,
    collection: &Collection,
    name: vector<u8>, description: vector<u8>, image_url: vector<u8>, decimals: u8,
    ctx: &mut TxContext,
): (TokenType, HolderRegistry)

public fun mint(
    registry: &mut HolderRegistry,
    amount: u64,
    recipient: address,
    ctx: &mut TxContext,
): Balance
```

`mint` aborts if `total_supply + amount > registry.cap`. **The cap lives on the
registry, not on `NftCap`.** `NftCap.remaining` counts _object_ supply units,
which is the 721 notion; a 1155 mint creates one object but can mint `amount`
copies. Sharing the counter would make "max supply" ambiguous (200 NFTs or
200 tokens?). Two separate counters, both issuer-controlled, is the honest
reading.

`burn` is inverse and requires the `Balance`:

```move
public fun burn(registry: &mut HolderRegistry, balance: Balance, ctx: &mut TxContext)
```

### Transfer + batch

```move
public fun transfer(balance: Balance, recipient: address, ctx: &mut TxContext)

/// Split a `Balance` into two `Balance` objects so one transaction can pay
/// several recipients. Each output is a normal object, so the batch needs no
/// framework support beyond `split`.
public fun split(balance: &Balance, amount: u64, ctx: &mut TxContext): Balance

/// Recombine two `Balance` objects of the same `TokenType`.
public fun join(a: Balance, b: Balance, ctx: &mut TxContext): Balance
```

`split`/`join` abort unless both sides agree on `token_type`, mirroring how
`coin::join`/`coin::split` work in this framework.

### Read surface

```move
public fun balance_of(registry: &HolderRegistry, owner: address): u64
public fun total_supply(registry: &HolderRegistry): u64
public fun balance_of_object(balance: &Balance): u64
public fun token_type_of(balance: &Balance): address
```

---

## Shared concerns

### Metadata is per collection, not per token

ERC-1155's `uri(id)` is a function; ERC-721's `tokenURI(id)` likewise. Here the
URI is data on the object. Consequences worth accepting deliberately:

- **Off-chain indexers** read `image_url` / `attributes` from the object. The
  runtime already parses `Collection` (`crates/kanari-types/src/collection.rs`
  has `name_str`, `description_str`), so an analogous `NftToken` parser goes
  there and not in the Move code.
- **Metadata is immutable after mint** except through `collection::update_metadata`,
  which is gated on `E_NOT_COLLECTION_CREATOR`. That covers collection-level
  fields; per-token fields stay fixed, which matches ERC-721's typical behaviour.

### Supply accounting differs by design

721 and 1155 expose the same `max_supply` concept but count different things
(objects vs. tokens). This is not a bug in the port; it is what the two
standards actually mean. Document it in the RPC layer rather than papering over
it with one shared counter.

### Censorship: use `deny_list`, do not reimplement it

Both standards need a pause/deny path. Neither module implements one.

- **Coin types** already have the framework `deny_list`, and the
  `public(friend)` hardening just completed means only `coin::create_regulated_currency`
  can mint the `DenyCap`. That path is closed to NFTs by construction.
- **Object families** cannot call the coin-facing deny-list API, whose
  capability is minted only for regulated coins. `krc_721` provides the
  framework-only bridge for NFT collections: it gates writes on that
  collection's `&collection::NftCap` and keys entries by collection id.

The `krc_721` pause hook uses the same framework deny-list data and is scoped
to one collection. Adding a second family-owned list would duplicate storage
and enforcement.

### Events

Both modules emit on every state transition, matching the existing
`CollectionCreated` shape so the indexer has one decoder:

```move
struct TokenMinted has copy, drop { collection: address, token: address, number: u64, recipient: address }
struct TokenBurned has copy, drop { collection: address, token: address }
struct TokenTypeCreated has copy, drop { collection: address, token_type: address }
struct BalanceMinted has copy, drop { token_type: address, owner: address, amount: u64 }
struct BalanceBurned has copy, drop { token_type: address, owner: address, amount: u64 }
```

`james::nft` already emits a `MintLog` that
`kanari_types::collection::MINT_LOG_EVENT` parses at fixed offsets
(`MINT_LOG_EVENT_SIZE = 96`). These new events are additional, not a
replacement; the existing `MintLog` decoding keeps working.

---

## Gaps in the existing framework this design must not step on

Found while reading `collection.move`. These are real and predate this design:

1. **`update_metadata` is not a mutation.** It takes `c: &mut Collection`, writes
   the two URL fields, then calls `object::save_object(c)` and returns. A
   `&mut` reference to a value that was passed by value cannot persist a write
   in Move — the caller's copy is unchanged. Either the signature should take
   `Collection` by value and transfer it back, or it needs
   `borrow_global_mut<T>(addr)` (this runtime has that native). Until fixed,
   collection metadata is effectively immutable after creation.

2. **`consume_for_mint` / `return_from_burn` persist via `object::save_object`.**
   Same mechanism, same concern. They work in the unit tests because tests hold
   the value locally; whether they persist correctly in a real transaction is
   what `save_object` is for, and it is worth an integration test that mints,
   commits, and re-reads `remaining` from a fresh runtime.

3. **`NftCap` has `drop`.** A `drop` capability can be silently discarded,
   which permanently freezes the collection's supply. `NftCap` arguably should
   be `store`-only (it already is `key + store + drop`); making it
   non-`drop` would force an explicit transfer, which is the safer default for
   an authority object. Changing this is a breaking change to existing
   collections, so it is a follow-up, not part of this design.

4. **`create_collection` takes `max_supply` unbounded.** There is no upper
   bound on `issued_counter`, so a 1155 registry cap and a 721 cap are both
   whatever the issuer typed. Acceptable for now; worth a sanity ceiling later.

Items 1 and 2 are correctness bugs in existing code, independent of these
standards, and 2 should have an integration test before either standard ships.

---

## Suggested order of work

1. Fix `update_metadata` (gap 1) and add the commit/re-read integration test
   for supply accounting (gap 2). Both are prerequisites — `krc_721::mint` and
   `erc1155::mint` build directly on `consume_for_mint`.
2. `krc_721` — it is the smaller surface and validates the
   `(cap, collection)` pairing pattern.
3. `erc1155` — `HolderRegistry` + `Balance` split, `split`/`join`.
4. Parsers in `crates/kanari-types/src/collection.rs` for off-chain display,
   alongside the existing `Collection` parser.
5. RPC/indexer surfacing of the new events.

Nothing here changes the runtime, the host natives, or the consensus rules — it
is all framework Move plus a Rust-side parser, so it does not require the
`KANARI_FRAMEWORK_ALLOW_INCOMPATIBLE` dance that the deny-list hardening did.
