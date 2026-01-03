## 6. Client APIs, RPCs, and Indexers

This chapter documents the client-facing APIs, RPC surface, and the role of indexers in the Kanari ecosystem, including where to find server and RPC code in the repository.

6.1 RPC server and endpoints

- `crates/kanari-rpc-server/src/` implements the RPC server used by indexers and clients. Notable submodules:
  - `block/mod.rs` — block and chain queries
  - `transaction/mod.rs` — transaction submission and status
  - `module/mod.rs` — module queries and metadata
  - `balance/mod.rs` — coin/balance related endpoints

6.2 Client flows

- Create object: client computes `ObjectId`, stores payload off-chain, calls Move transaction via RPC that invokes `create_object` Move entrypoint.
- Update object: client signs and submits a transaction that calls `publish_metadata` with a `payload_hash` or `payload_diff`. RPC returns committed `tx_hash` and emitted `Event` (including `record_hash`).
- Verify object: client fetches object's head via RPC (`get_object_head` or similar), requests a compact proof from an indexer or runtime, and validates the proof using `crates/smt` hashing utilities.

6.3 Indexers and event consumption

- Indexers subscribe to on-chain events (emitted by Move modules) and materialize views optimized for queries. The repository does not include a single indexer service; however the RPC server and `event.move` describe the event shapes indexers must consume.
- Best practice: indexers optionally hold `IndexerCapability` to perform derived annotations, but should never alter canonical on-chain records.

6.4 Example clients & utilities

- Client-side cryptography and keystore helpers are in `crates/kanari-crypto/src/` (keys, signatures, wallet). Re-use these utilities to ensure compatible signature formats and address derivations.
- Integration examples may be found under `packages/kanari_flutter` and `crates/kanari-move-runtime/examples/`.
