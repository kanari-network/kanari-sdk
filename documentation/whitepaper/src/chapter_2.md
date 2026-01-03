### 2. Data Model and Primitives

This chapter defines the canonical data structures and low-level primitives Kanari exposes to clients and on-chain Move modules. The model is intentionally minimal to enable composability while providing strong cryptographic guarantees.

2.1 Metadata Object

- Identifier: a globally unique `ObjectId` computed as a hash of an origin address, a local sequence number, and a namespace tag.
- Owner: an on-chain account address that holds the canonical ownership capability for the object.
- Payload: an opaque, application-defined JSON or binary blob containing descriptive attributes, URIs, and schema pointers.
- Head: a reference to the latest metadata record (by record hash) in the object's append-only history.

2.2 Metadata Record

- RecordHash: the cryptographic hash of the record contents.
- Prev: the previous `RecordHash` (or null for genesis), forming an append-only chain.
- Author: the signer address that produced the update.
- Timestamp: logical block time when the update was committed.
- PayloadDiff: either a full payload or a minimal delta describing the change.
- Signatures: optional external signatures for off-chain attestations.

2.3 Capabilities and Permissions

Kanari models access control using capability objects that are transferrable and revocable:

- OwnerCapability: grants full rights to publish updates and transfer ownership.
- UpdateCapability: grants rights to submit updates but not to transfer ownership.
- IndexerCapability: read-only capability for trusted indexers to annotate metadata with derived indices.

Capabilities are first-class Move resources; transferring and revoking capabilities is a protocol primitive enforced by Move modules.

2.4 Primitives

- publish_metadata(object_id, payload_diff, capability): appends a new record to the object's history when the caller holds a valid capability.
- transfer_ownership(object_id, new_owner, owner_capability): atomically transfers `OwnerCapability`.
- revoke_capability(object_id, cap_id, owner_capability): invalidates a delegated capability.
- get_record(record_hash): returns the full record for verification and proof construction.

2.5 Compact Proofs and External Verification

To enable lightweight verification by off-chain clients, Kanari supports compact Merkle-like proofs that prove inclusion of a `RecordHash` in an object's history head. Proofs are produced by on-chain routines and can be validated with the object's head and the protocol's canonical hashing scheme.

2.6 Storage and Off-chain Indexing

Metadata payloads can be stored off-chain (e.g., IPFS, S3) with on-chain records anchoring URIs and content hashes. Indexers maintain materialized views and provide APIs for search, but canonical authority remains the on-chain record chain.

Summary

Kanari's data model balances expressiveness and verifiability: small, strongly-typed on-chain records anchor flexible off-chain payloads, while capability-based access control enables secure delegation patterns required by enterprise workflows.

Code mapping (where to find these concepts in the repository)

- `crates/kanari-types/src/object.rs`: canonical `ObjectId` and object-related types used by Rust code and the runtime.
- `crates/kanari-types/src/tx_context.rs` and `src/kanari.rs`: transaction and ownership-related types referenced by client code.
- `crates/kanari-frameworks/packages/kanari-system/sources/object.move`: Move-side object model and head-management primitives.
- `crates/kanari-frameworks/packages/kanari-system/sources/tx_context.move`: Move-side transaction/context primitives and capability semantics.
- `crates/kanari-move-runtime/src/storage/object_storage.rs` and `src/move_runtime/object_ops.rs`: runtime storage handling and object history organization.
- `crates/smt/src/sparse_merkle.rs`: sparse Merkle / compact proof utilities used by the runtime for inclusion proofs.
- `crates/kanari-frameworks/packages/kanari-system/sources/event.move` and `crates/kanari-system/tests/event_tests.move`: event formats and examples consumed by indexers.

These files contain the authoritative implementation of the types and primitives described above; use them as primary references when implementing clients, indexers, or Move modules that interoperate with Kanari.

