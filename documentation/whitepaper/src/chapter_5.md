## 5. Runtime, Storage, and Proofs Implementation

This chapter explains how Kanari's Rust runtime implements storage, object history, and compact proofs used for efficient verification by off-chain clients.

5.1 Runtime components (repo locations)

- `crates/kanari-move-runtime/src/lib.rs` — top-level runtime integration.
- `crates/kanari-move-runtime/src/move_runtime/mod.rs` and `helpers.rs` — runtime helpers and native extension registration.
- `crates/kanari-move-runtime/src/move_runtime/object_ops.rs` — object-level operations (head updates, fetches).
- `crates/kanari-move-runtime/src/storage/object_storage.rs` — persistent storage layer for object state.
- `crates/smt/src/sparse_merkle.rs` and `hash.rs` — sparse Merkle tree implementation and hashing primitives used to build compact inclusion proofs.

5.2 Object history and compact proofs

- Each object's head is anchored on-chain (small `RecordHash`). The full record chain may be stored off-chain or reconstructed by indexers. To verify inclusion without fetching the full chain, the runtime can construct a sparse-merkle-like proof proving that a `RecordHash` equals the head at a particular sequence.
- Proof verification uses `crates/smt` helpers; ensure clients use the same canonical hashing scheme found in `crates/smt/src/hash.rs`.

5.3 Native functions and determinism

- Natives implemented under `move_runtime_extensions.rs` must be deterministic across validator nodes. They should avoid filesystem or RNG usage that would diverge execution.

5.4 Persistence and shared DB

- Persistent disk-backed stores are implemented under `crates/kanari-move-runtime/src/storage/persistent_store.rs` (and related shared DB files). These modules manage serialization formats and recovery semantics.

5.5 Tests & examples

- See `crates/kanari-move-runtime/examples/` for e2e examples (`e2e_nft_publish.rs`, `e2e_token_publish.rs`) demonstrating publishing and verifying metadata flows.
