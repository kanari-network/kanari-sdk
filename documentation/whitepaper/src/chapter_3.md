### 3. System Architecture and Components

This chapter outlines Kanari's runtime architecture, the role of Move contracts and native extensions, and the interactions between clients, indexers, and storage providers.

3.1 Principled Layers

- Consensus & Execution: The underlying blockchain provides finality and ordering for metadata updates. Move contracts encode protocol state and enforce capability checks.
- On-chain Registry: Lightweight Move modules store object heads, capability tables, and pointers to payload storage.
- Native Extensions: Performance-sensitive operations (compact proof generation, advanced hashing) can be implemented as native functions exposed to Move while preserving deterministic behavior.
- Indexing Layer: Off-chain indexers subscribe to events and maintain searchable views, offering APIs for queries and bulk export.
- Storage Layer: Large payloads live off-chain with integrity anchored on-chain via content hashes and URIs.

3.2 Move Modules

Key Move modules include:

- `kanari_registry`: core types and primitives for object creation, head updates, and capability management.
- `kanari_access`: higher-level policies for delegation, roles, and revocation semantics.
- `kanari_proofs`: utilities for constructing and verifying compact inclusion proofs.

3.3 Eventing and Observability

Updates emit structured events containing `ObjectId`, new `RecordHash`, author, and minimal metadata useful to indexers. Events are the canonical change-stream for off-chain consumers.

3.4 Client Workflows

- Create: client generates an `ObjectId`, stores payload off-chain, calls `create_object` with payload hash and initial capability assignment.
- Update: authorized client calls `publish_metadata` with a `PayloadDiff`; Move module validates capability and appends a new record.
- Transfer: owner calls `transfer_ownership`, which atomically hands off capabilities and emits transfer events.
- Verify: any verifier fetches the object's head, retrieves the record chain (or a proof), and validates record hashes and signatures.

3.5 Indexers and Search

Indexers consume events and materialize views optimized for query types: by owner, by schema type, or by temporal range. Indexers may optionally hold `IndexerCapability` to annotate records with derived metadata (e.g., text extraction), but annotations do not change canonical on-chain records.

3.6 Security Considerations

- Minimize on-chain payloads: keep authoritative data small and verifiable; store bulk data off-chain.
- Capability hygiene: prefer short-lived or narrow-scope capabilities to reduce blast radius.
- Deterministic natives: ensure any native functions used for proof generation are deterministic across validator nodes.
- Auditing: comprehensive event logs and compact proofs enable external auditors to reconstruct histories without trusting indexers.

3.7 Scalability and Performance

Kanari offloads heavy payload storage and full-text indexing to off-chain systems while keeping a compact, cryptographically secure on-chain state. This hybrid approach scales to millions of objects while ensuring a single canonical source of truth for ownership and versioning.

Summary

Kanari's architecture combines Move's strong safety semantics with targeted native extensions and off-chain services to deliver a practical, auditable metadata management system suitable for both open ecosystems and enterprise deployments.

Code mapping (where to find implementation in the repo)

- Move sources and on-chain modules: `crates/kanari-frameworks/packages/kanari-system/sources/` (files of interest: `kanari.move`, `object.move`, `tx_context.move`, `event.move`, `transfer.move`, `coin.move`, `balance.move`). These Move files implement the on-chain record heads, capability resources and events.
- Runtime & native helpers: `crates/kanari-move-runtime/src/move_runtime/` (e.g., `helpers.rs`, `object_ops.rs`, `move_runtime_extensions.rs`) and storage under `crates/kanari-move-runtime/src/storage/` — runtime enforcement, proof helpers, and native functions live here.
- Sparse Merkle and hashing primitives used for compact proofs: `crates/smt/src/` (notably `sparse_merkle.rs` and `hash.rs`).
- Canonical Rust types for objects, coins, balances and transactions: `crates/kanari-types/src/` (files: `object.rs`, `coin.rs`, `balance.rs`, `tx_context.rs`, `kanari.rs`).
- RPC surface used by indexers / clients: `crates/kanari-rpc-server/src/` (modules for `block`, `transaction`, `module`, `balance` expose server endpoints and event streaming patterns).
- Cryptography & signatures used by clients and attestations: `crates/kanari-crypto/src/` (keys, keystore, signatures, and wallet utilities).
- Tests and examples demonstrating end-to-end flows: `crates/kanari-move-runtime/examples/` (e2e token/nft examples) and Move tests under `crates/kanari-frameworks/packages/kanari-system/tests/`.

Use these files as the ground truth when updating or extending the system; the chapter text maps directly to the modules and crates above.

