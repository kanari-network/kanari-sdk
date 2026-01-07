# Merkle Tree Implementation

## Overview

Kanari blockchain uses **two types of Merkle trees** for different purposes:

### 1. Sparse Merkle Tree (SMT) - State Storage

- **Location**: `crates/smt/`
- **Purpose**: Account state verification and proofs
- **Used for**:
  - Account balances
  - Module storage
  - Object ownership
  - State root computation
- **Implementation**: Full SMT with 256-bit keyspace
- **Storage**: Persistent in RocksDB

### 2. Transaction Merkle Tree - Block Verification

- **Location**: `crates/kanari-core/src/blockchain/merkle.rs`
- **Purpose**: Light client transaction verification
- **Used for**:
  - Block header merkle root
  - Transaction inclusion proofs
  - Light client sync
- **Implementation**: Binary merkle tree (like Bitcoin/Ethereum)
- **Storage**: In-memory, reconstructed per block

## Why Two Different Trees?

| Feature              | SMT (State)                 | Binary Tree (Transactions)|
|----------------------|-----------------------------|---------------------------|
| **Persistence**      | Persisted in DB             | Computed on-demand        |
| **Proof Type**       | Membership + Non-membership | Inclusion only            |
| **Size**             | Sparse (256-bit keyspace)   | Dense (transaction count) |
| **Update Frequency** | Per transaction execution   | Per block production      |
| **Primary Use**      | State verification          | Light client sync         |

## Hash Function Consistency

Both trees use **Blake3** for consistency:

- SMT: `smt::digest()`
- Transaction Tree: `smt::digest()` (same function!)

This ensures:

- Consistent security properties
- Fast hashing performance
- Simple verification logic

## Transaction Merkle Tree API

### Compute Merkle Root

```rust
use kanari_core::blockchain::compute_merkle_root;

let tx_hashes: Vec<Vec<u8>> = transactions
    .iter()
    .map(|tx| tx.hash())
    .collect();

let merkle_root = compute_merkle_root(&tx_hashes);
```

### Generate Proof

```rust
use kanari_core::blockchain::generate_merkle_proof;

// Get proof for transaction at index 2
let proof = generate_merkle_proof(&tx_hashes, 2);
```

### Verify Proof

```rust
use kanari_core::blockchain::verify_merkle_proof;

let is_valid = verify_merkle_proof(
    &tx_hash,
    2,           // index
    &proof,
    &merkle_root
);
```

### Batch Proof Verification

Verify multiple proofs efficiently in O(n) time:

```rust
use kanari_core::blockchain::batch_verify_merkle_proofs;

let proofs = vec![
    (tx_hash_1, 0, proof_1.clone()),
    (tx_hash_2, 2, proof_2.clone()),
    (tx_hash_3, 5, proof_3.clone()),
];

let all_valid = batch_verify_merkle_proofs(&proofs, &merkle_root);
```

### Merkle Multiproof

Generate a single deduplicated proof for multiple transactions:

```rust
use kanari_core::blockchain::generate_merkle_multiproof;

// Prove transactions at indices [1, 3, 5]
let indices = vec![1, 3, 5];
let multiproof = generate_merkle_multiproof(&tx_hashes, &indices);

// Multiproof shares common siblings, reducing bandwidth
```

### Compressed Proof Format

Reduce proof size by ~50% using bit flags:

```rust
use kanari_core::blockchain::CompressedMerkleProof;

// Compress a standard proof
let compressed = CompressedMerkleProof::from_proof(&tx_hash, index, &proof);

// Serialize to compact binary format
let bytes = compressed.to_bytes();

// Deserialize and verify
let decompressed = CompressedMerkleProof::from_bytes(&bytes).unwrap();
let is_valid = decompressed.verify(&tx_hash, index, &merkle_root);
```

## RPC Endpoint

Light clients can request transaction proofs via RPC:

### Standard Proof

```json
{
  "jsonrpc": "2.0",
  "method": "kanari_getTransactionMerkleProof",
  "params": {
    "block_height": 100,
    "tx_index": 2
  },
  "id": 1
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "tx_hash": "abc123...",
    "tx_index": 2,
    "merkle_root": "def456...",
    "proof": ["hash1", "hash2", "hash3"]
  },
  "id": 1
}
```

### Batch Proof Request

Request multiple proofs in a single call:

```json
{
  "jsonrpc": "2.0",
  "method": "kanari_getBatchMerkleProofs",
  "params": {
    "block_height": 100,
    "tx_indices": [1, 3, 5]
  },
  "id": 1
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "block_height": 100,
    "merkle_root": "def456...",
    "proofs": [
      {
        "tx_hash": "abc123...",
        "tx_index": 1,
        "merkle_root": "def456...",
        "proof": ["hash1", "hash2"]
      },
      {
        "tx_hash": "def789...",
        "tx_index": 3,
        "merkle_root": "def456...",
        "proof": ["hash3", "hash4"]
      }
    ]
  },
  "id": 1
}
```

### Compressed Proof Request

Get bandwidth-optimized proof (~50% smaller):

```json
{
  "jsonrpc": "2.0",
  "method": "kanari_getCompressedMerkleProof",
  "params": {
    "block_height": 100,
    "tx_index": 2
  },
  "id": 1
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "tx_hash": "abc123...",
    "tx_index": 2,
    "merkle_root": "def456...",
    "compressed_proof": "base64encodeddata...",
    "original_size": 256,
    "compressed_size": 128
  },
  "id": 1
}
```

## RPC Endpoints

## Block Structure

Each block contains:

```rust
pub struct BlockHeader {
    pub height: u64,
    pub timestamp: u64,
    pub prev_hash: Vec<u8>,
    pub state_root: Vec<u8>,      // SMT root (account state)
    pub merkle_root: Vec<u8>,     // Transaction merkle root
    pub tx_count: usize,
}
```

## Verification Flow

### Full Node

1. Execute all transactions
2. Compute state changes
3. Update SMT → new state_root
4. Collect transaction hashes
5. Compute merkle_root
6. Create block with both roots

### Light Client

1. Download block header only
2. Trust merkle_root (or verify via consensus)
3. Request specific transaction proof
4. Verify proof against merkle_root
5. Trust transaction is in block ✅

## Testing

Run merkle tree tests:

```bash
cargo test --package kanari-core blockchain::merkle
```

All tests should pass:

- ✅ Empty tree
- ✅ Single transaction
- ✅ Two transactions
- ✅ Three transactions (odd number)
- ✅ Proof generation and verification
- ✅ Invalid proof detection
- ✅ Batch proof verification
- ✅ Merkle multiproof generation
- ✅ Compressed proof format
- ✅ Compressed proof serialization

## Security Properties

1. **Collision Resistance**: Blake3 provides strong collision resistance
2. **Second Preimage Resistance**: Cannot create different transaction set with same root
3. **Deterministic**: Same transactions always produce same merkle root
4. **Efficient**: O(log n) proof size and verification time

## Performance Optimizations

### Proof Caching

Frequently requested merkle proofs are cached using an LRU cache (1000 entries) to reduce computation:

- **Cache Hit**: O(1) retrieval from memory
- **Cache Miss**: O(log n) computation, then cached for future requests
- **Cache Eviction**: Least recently used proofs are evicted when cache is full

This is particularly useful for:

- Light clients repeatedly verifying recent transactions
- Block explorers serving popular blocks
- API endpoints with high traffic

### Bandwidth Optimization

- **Standard Proof**: ~256 bytes for typical block (log₂(1000) ≈ 10 siblings * 32 bytes)
- **Compressed Proof**: ~128 bytes (50% reduction using bit flags)
- **Batch Proofs**: Deduplicated sibling nodes reduce total bandwidth

## Implemented Enhancements

- ✅ **Batch proof verification** - `batch_verify_merkle_proofs()` verifies multiple proofs in O(n) time
- ✅ **Compressed proof format** - `CompressedMerkleProof` reduces bandwidth by ~50% using bit flags
- ✅ **Merkle multiproofs** - `generate_merkle_multiproof()` deduplicates sibling nodes across multiple indices
- ✅ **Proof caching** - LRU cache (1000 entries) for frequently requested merkle proofs
- ✅ **RPC endpoints** - `kanari_getBatchMerkleProofs` and `kanari_getCompressedMerkleProof`

## Future Enhancements

- [ ] Integration with consensus for light client security

## References

- Bitcoin merkle trees: [Bitcoin Wiki](https://en.bitcoin.it/wiki/Protocol_documentation#Merkle_Trees)
- Ethereum merkle trees: [Ethereum Yellow Paper](https://ethereum.github.io/yellowpaper/paper.pdf)
- Sparse Merkle Trees: [Jellyfish Merkle Tree](https://developers.diem.com/papers/jellyfish-merkle-tree/2021-01-14.pdf)
- Blake3: [BLAKE3 Specification](https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf)
