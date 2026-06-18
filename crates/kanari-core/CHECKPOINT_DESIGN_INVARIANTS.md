# Checkpoint and Height Design Invariants

## Status

This document defines intentional product behavior for Kanari Core and the
multi-node runtime. These rules are not temporary workarounds.

Do not change these invariants without an explicit product decision from the
Kanari maintainers and new multi-node tests that prove the replacement design.

## Product Requirement

Kanari uses transaction-driven checkpoints.

A node must create a new blockchain checkpoint and increment blockchain height
only when it has one or more accepted transactions to commit.

When the mempool is empty:

- blockchain height must remain unchanged;
- no empty checkpoint may be created;
- no empty DAG vertex may be used to advance blockchain height;
- a DAG synchronization request must not manufacture a checkpoint;
- receiving a network DAG vertex must not directly create a checkpoint.

This behavior is intentional. The developer wants height to represent committed
transaction work, not elapsed time, polling activity, peer traffic, or an idle
consensus heartbeat.

## Required Separation

The system has two related but different kinds of state:

1. Mysticeti DAG state
2. Kanari blockchain checkpoint state

A received DAG vertex may update local consensus metadata and may be persisted
for DAG synchronization. It must not directly mutate Move state, append a
blockchain checkpoint, increment height, or increment the transaction count.

Blockchain state changes occur through a committed checkpoint containing real
transactions, or through checkpoint synchronization from a valid peer.

## Core Enforcement

The invariant is intentionally enforced at more than one layer.

### BlockchainEngine

`BlockchainEngine::produce_checkpoint()` must reject an empty mempool with:

```text
No new transactions to checkpoint
```

This is the public core guard. Callers must not bypass it to create idle
checkpoints.

### DagEngine

`DagEngine::produce_vertex()` must also reject a zero-transaction batch.

This second guard is defense in depth. It protects the invariant if a future
caller invokes the DAG engine directly.

### Network DAG Vertices

`DagEngine::add_network_vertex()` may validate, deduplicate, store, and persist
the vertex as consensus metadata.

It must not call checkpoint finalization or apply the vertex transactions to
blockchain state.

## Node Runtime Rules

The node production loop may call `produce_checkpoint()` only when:

- the mempool contains at least one transaction; and
- the short transaction gossip delay has completed.

Do not restore an idle `should_produce_progress` path. Consensus progress must
not create empty blockchain history.

The gossip delay exists so other authorities can receive the transaction batch
before local checkpoint production. Removing it requires a replacement test
showing deterministic multi-node transaction propagation.

## Synchronization Rules

A DAG vertex request is a request for existing DAG data. It is not permission to
create new empty work.

The DAG sync responder must return existing vertices only. It must not call
`produce_checkpoint()` to create a catch-up vertex.

A node that is behind in blockchain height must recover through checkpoint
synchronization. Checkpoint sync must preserve sequence validation, transaction
signature verification, replay protection, and state-root verification.

## Transaction Counting

The transaction count represents unique committed signed transactions.

It must not include:

- Move system prologue execution;
- clock updates;
- DAG vertices;
- peer messages;
- failed duplicate submissions;
- empty checkpoints;
- duplicated transaction-index entries.

Explorer and RPC statistics must be derived from queryable committed transaction
history. A count that differs from the unique transaction hashes is a bug.

## State Root Requirement

For the same ordered transaction history and starting state, all authorities
must compute the same state root.

Differences in local DAG vertex IDs or checkpoint hashes may occur because of
authority metadata or timestamps. Differences in the state root at the same
committed height are not normal and must be investigated.

Do not hide state-root divergence by selecting one node's value in the RPC or
explorer layer.

## Changes That Require Explicit Review

The following changes require an explicit architecture review:

- allowing checkpoints with zero transactions;
- incrementing height on a timer;
- incrementing height after receiving a DAG vertex;
- producing checkpoints inside DAG request handlers;
- applying transaction payloads directly from an uncommitted network vertex;
- removing transaction replay checks;
- replacing unique transaction counting with a mutable accumulated counter;
- treating cached transaction hashes as authoritative without immutable signed
  transactions;
- weakening checkpoint sequence, signature, or state-root verification.

## Required Regression Tests

Any change to checkpoint production or synchronization must retain tests for:

1. Empty mempool does not create a checkpoint.
2. Empty mempool does not increment height after restart.
3. Receiving a network DAG vertex does not increment height.
4. A real transaction creates exactly one committed transaction.
5. Duplicate gossip does not create another committed transaction.
6. Four nodes converge on transaction count and state root.
7. A lagging node catches up through checkpoint sync.
8. Restarting all nodes does not create new transactions or heights.

## Operational Verification

After changing core consensus, node runtime, or sync code:

```powershell
cargo check -p kanari-core -p kanari-node -p kanari-rpc-server
cargo test -p kanari-core
cargo test -p kanari-node sync::tests
```

For a four-node local network, poll `kanari_getStats` on ports `19001`, `19011`,
`19021`, and `19031`.

With all mempools empty, poll twice several seconds apart. Every node must retain
the same height. After submitting one transaction, every node must eventually
report the same unique transaction count and state root.

## Maintainer Note

This design deliberately favors meaningful blockchain height and deterministic
multi-node state over empty consensus heartbeat checkpoints.

If Mysticeti requires additional liveness messages, implement those as
consensus-layer messages or DAG metadata. Do not represent them as user
transactions or Kanari blockchain checkpoints.
