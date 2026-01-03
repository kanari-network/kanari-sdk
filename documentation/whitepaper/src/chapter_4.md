## 4. Move Modules and On-Chain Schemas

This chapter maps the high-level data model and primitives to the actual Move modules in the repository and explains the on-chain schema shapes used by Kanari's Move packages.

4.1 Key Move modules (repo locations)

- `crates/kanari-frameworks/packages/kanari-system/sources/kanari.move` — protocol-level types and common constants used across modules.
- `crates/kanari-frameworks/packages/kanari-system/sources/object.move` — object creation, `head` storage, and history anchoring primitives.
- `crates/kanari-frameworks/packages/kanari-system/sources/tx_context.move` — transaction-context types and helpers for capability checks.
- `crates/kanari-frameworks/packages/kanari-system/sources/event.move` — event shapes emitted by protocol actions (used by indexers).
- `crates/kanari-frameworks/packages/kanari-system/sources/transfer.move` and `coin.move` — examples of asset transfer semantics and token interactions that coexist with metadata operations.

4.2 On-chain resources and types

Common Move resource shapes used by Kanari:

- `ObjectHead` (or similar): stores the current `RecordHash` for an object and possibly a `sequence` counter.
- `MetadataRecord` (stored off-chain or emitted as events): small on-chain footprint referencing an off-chain `payload_hash` and `prev_record_hash`.
- `Capability` resources: typed resources representing `OwnerCapability`, `UpdateCapability`, and `IndexerCapability` attached to accounts or object tables.

4.3 Storage tables and indexing in Move

Move modules keep small tables keyed by `ObjectId` or account address. Look for `Table` or `vector`-backed storages inside the Move sources; these ensure efficient lookup of heads and capability assignments.

4.4 Events emitted by Move

Move modules publish events for authoritative changes. Event fields commonly include `object_id`, `record_hash`, `author`, and `timestamp`. Indexers rely on these events to build materialized views.

4.5 How Move and the Rust runtime interoperate

- Move modules define the protocol invariants and resource types; the Rust runtime (`crates/kanari-move-runtime`) executes Move bytecode, enforces caps, and implements low-level storage and proof helpers.
- Native functions exposed to Move (in `move_runtime_extensions.rs`) provide performance-sensitive primitives, such as hashing or proof construction, that remain deterministic across validators.

References

- Move sources: `crates/kanari-frameworks/packages/kanari-system/sources/`
- Runtime: `crates/kanari-move-runtime/src/move_runtime/` and `crates/kanari-move-runtime/src/storage/`
