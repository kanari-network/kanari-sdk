// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::hash::{digest, hash_leaf, hash_node};
use crate::open_or_get_db;
use anyhow::Result;
use once_cell::sync::Lazy;
use rocksdb::IteratorMode;
use rocksdb::WriteBatch;
use std::path::PathBuf;

/// Precomputed default hashes for SMT levels (256 levels + 1 leaf default)
static DEFAULT_HASHES: Lazy<Vec<[u8; 32]>> = Lazy::new(|| {
    let mut default_hashes = vec![[0u8; 32]; 257];
    default_hashes[256] = hash_leaf(&[0u8; 32], &[0u8; 32]);

    for d in (0..256).rev() {
        default_hashes[d] = hash_node(&default_hashes[d + 1], &default_hashes[d + 1]);
    }
    default_hashes
});

/// Sparse Merkle Tree implementation (256-bit keyspace using BLAKE3).
/// - Leaf hash: H(0x00 || key_hash || value)
/// - Node hash: H(0x01 || left || right)
///   Stores only non-default nodes in RocksDB under keys `smt:node:<depth>:<prefix_bytes>`.
#[derive(Debug)]
pub struct SparseMerkleTree {
    db: std::sync::Arc<rocksdb::DB>,
    default_hashes: &'static [[u8; 32]],
}

fn node_key(depth: usize, prefix: &[u8]) -> Vec<u8> {
    let mut out = b"smt:node:".to_vec();
    let d = (depth as u16).to_be_bytes();
    out.extend(&d);
    out.extend(prefix);
    out
}

fn data_key(key_hash: &[u8; 32]) -> Vec<u8> {
    let mut out = b"smt:data:".to_vec();
    out.extend(key_hash);
    out
}

impl SparseMerkleTree {
    pub fn new(db: std::sync::Arc<rocksdb::DB>) -> Self {
        Self {
            db,
            default_hashes: &DEFAULT_HASHES,
        }
    }

    pub fn open(path_opt: Option<PathBuf>) -> Result<Self> {
        let db = open_or_get_db(path_opt)?;
        Ok(Self::new(db))
    }

    /// Export all stored SMT key/value pairs (node entries and data entries)
    /// as a vector of raw bytes. This is used for creating lightweight
    /// snapshots that can later be used to serve historical proofs.
    pub fn export_snapshot(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut out: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        let iter = self.db.iterator(IteratorMode::Start);
        for item in iter {
            let (k, v) = item?;
            let key = k.to_vec();
            if key.starts_with(b"smt:") {
                out.push((key, v.to_vec()));
            }
        }
        Ok(out)
    }

    pub fn root_hash(&self) -> Result<[u8; 32]> {
        // root is stored at depth 0 with empty prefix
        let key = node_key(0, &[]);
        if let Ok(Some(v)) = self.db.get(key) {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&v);
            Ok(arr)
        } else {
            Ok(self.default_hashes[0])
        }
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let kh = digest(key);
        if let Some(v) = self.db.get(data_key(&kh))? {
            Ok(Some(v.to_vec()))
        } else {
            Ok(None)
        }
    }

    /// Produce a proof for `key`. Returns (is_member, leaf_hash, siblings bottom-up)
    pub fn proof(&self, key: &[u8]) -> Result<(bool, [u8; 32], Vec<[u8; 32]>)> {
        let kh = digest(key);

        // check membership
        let is_member = self.db.get(data_key(&kh))?.is_some();

        // leaf hash
        let leaf_hash = if is_member {
            let val = self.db.get(data_key(&kh))?.unwrap();
            hash_leaf(&kh, &val)
        } else {
            self.default_hashes[256]
        };

        let mut siblings: Vec<[u8; 32]> = Vec::with_capacity(256);

        // traverse from leaf depth down to 1 and collect sibling at each level
        for depth in (1..=256).rev() {
            let prefix_bits: usize = depth;
            let prefix_bytes = prefix_bits.div_ceil(8_usize);
            let mut prefix = vec![0u8; prefix_bytes];
            prefix.copy_from_slice(&kh[..prefix_bytes]);
            let excess = (prefix_bytes * 8) - prefix_bits;
            if excess > 0 {
                let mask = 0xFF << excess;
                let last = prefix_bytes - 1;
                prefix[last] &= mask as u8;
            }

            let last_bit_index = prefix_bits - 1;
            let byte_idx = last_bit_index / 8;
            let bit_in_byte = 7 - (last_bit_index % 8);

            let mut sibling_prefix = prefix.clone();
            sibling_prefix[byte_idx] ^= 1u8 << bit_in_byte;
            let sibling_key = node_key(depth, &sibling_prefix);
            if let Ok(Some(v)) = self.db.get(&sibling_key) {
                let mut a = [0u8; 32];
                a.copy_from_slice(&v);
                siblings.push(a);
            } else {
                siblings.push(self.default_hashes[depth]);
            }
        }

        Ok((is_member, leaf_hash, siblings))
    }

    /// Insert a batch of key/value pairs. Simple implementation that calls
    /// `insert` per entry. This can be optimized later to build a single
    /// write batch updating multiple leaves/parents.
    pub fn insert(&self, kvs: &[(Vec<u8>, Vec<u8>)]) -> Result<()> {
        // Use a single WriteBatch to make the batch operation atomic and
        // reduce RocksDB round-trips. We maintain a small in-memory cache of
        // node hashes we write during this batch so subsequent keys can see
        // sibling updates without additional DB reads.
        let mut batch = WriteBatch::default();
        use std::collections::BTreeMap;
        let mut node_cache: BTreeMap<Vec<u8>, [u8; 32]> = BTreeMap::new();

        for (k, v) in kvs.iter() {
            let key = k.as_slice();
            let value = v.as_slice();
            let kh = digest(key);

            // compute leaf hash
            let leaf_hash = hash_leaf(&kh, value);

            // queue data write
            batch.put(data_key(&kh), value);

            // current hash as array
            let mut cur = leaf_hash;

            for depth in (1..=256).rev() {
                let prefix_bits: usize = depth;
                let prefix_bytes = prefix_bits.div_ceil(8_usize);
                let mut prefix = vec![0u8; prefix_bytes];
                prefix.copy_from_slice(&kh[..prefix_bytes]);
                let excess = (prefix_bytes * 8) - prefix_bits;
                if excess > 0 {
                    let mask = 0xFF << excess;
                    let last = prefix_bytes - 1;
                    prefix[last] &= mask as u8;
                }

                let last_bit_index = prefix_bits - 1;
                let byte_idx = last_bit_index / 8;
                let bit_in_byte = 7 - (last_bit_index % 8);

                let mut sibling_prefix = prefix.clone();
                sibling_prefix[byte_idx] ^= 1u8 << bit_in_byte;
                let sibling_key_vec = node_key(depth, &sibling_prefix);

                // try cache first, then DB
                let sibling_hash = if let Some(h) = node_cache.get(&sibling_key_vec) {
                    *h
                } else if let Ok(Some(vb)) = self.db.get(&sibling_key_vec) {
                    let mut a = [0u8; 32];
                    a.copy_from_slice(&vb);
                    a
                } else {
                    self.default_hashes[depth]
                };

                let bit = ((kh[byte_idx] >> bit_in_byte) & 1u8) == 0;
                let (left, right) = if bit {
                    (cur, sibling_hash)
                } else {
                    (sibling_hash, cur)
                };

                let parent = hash_node(&left, &right);
                let parent_arr = parent;

                let node_k = node_key(depth, &prefix);
                batch.put(node_k.clone(), cur);
                node_cache.insert(node_k, cur);

                cur = parent_arr;
            }

            // root
            batch.put(node_key(0, &[]), cur);
        }

        self.db.write(batch)?;
        Ok(())
    }

    /// Delete a single key from the tree, updating parent nodes. This will
    /// remove the stored data entry and propagate default hashes upward,
    /// deleting node records when they equal the default value.
    /// Delete a batch of keys
    pub fn delete(&self, keys: &[Vec<u8>]) -> Result<()> {
        // Optimize deletes by sorting/deduping keys by their key-hash, and
        // applying all deletions in a single WriteBatch while caching node
        // reads/writes to avoid repeated DB access for shared prefixes.
        if keys.is_empty() {
            return Ok(());
        }

        use std::collections::BTreeMap;
        // prepare (key, key_hash) pairs
        let mut keyed: Vec<(Vec<u8>, [u8; 32])> = Vec::with_capacity(keys.len());
        for k in keys.iter() {
            let khd = digest(k.as_slice());
            keyed.push((k.clone(), khd));
        }

        // sort and dedup by hash to coalesce shared prefixes
        keyed.sort_by(|a, b| a.1.cmp(&b.1));
        keyed.dedup_by(|a, b| a.1 == b.1);

        let mut batch = WriteBatch::default();
        let mut node_cache: BTreeMap<Vec<u8>, [u8; 32]> = BTreeMap::new();

        for (_key, kh) in keyed.into_iter() {
            // delete stored data entry
            batch.delete(data_key(&kh));

            // start with default leaf
            let mut cur = self.default_hashes[256];

            for depth in (1..=256).rev() {
                let prefix_bits: usize = depth;
                let prefix_bytes = prefix_bits.div_ceil(8_usize);
                let mut prefix = vec![0u8; prefix_bytes];
                prefix.copy_from_slice(&kh[..prefix_bytes]);
                let excess = (prefix_bytes * 8) - prefix_bits;
                if excess > 0 {
                    let mask = 0xFF << excess;
                    let last = prefix_bytes - 1;
                    prefix[last] &= mask as u8;
                }

                let last_bit_index = prefix_bits - 1;
                let byte_idx = last_bit_index / 8;
                let bit_in_byte = 7 - (last_bit_index % 8);

                let mut sibling_prefix = prefix.clone();
                sibling_prefix[byte_idx] ^= 1u8 << bit_in_byte;
                let sibling_key = node_key(depth, &sibling_prefix);

                // try cache first, then DB
                let sibling_hash = if let Some(h) = node_cache.get(&sibling_key) {
                    *h
                } else if let Ok(Some(v)) = self.db.get(&sibling_key) {
                    let mut a = [0u8; 32];
                    a.copy_from_slice(&v);
                    a
                } else {
                    self.default_hashes[depth]
                };

                let bit = ((kh[byte_idx] >> bit_in_byte) & 1u8) == 0;
                let (left, right) = if bit {
                    (cur, sibling_hash)
                } else {
                    (sibling_hash, cur)
                };

                let parent_arr = hash_node(&left, &right);

                // if current equals default at this depth, remove stored node, else write
                let node_k = node_key(depth, &prefix);
                if cur == self.default_hashes[depth] {
                    batch.delete(node_k.clone());
                    node_cache.insert(node_k, self.default_hashes[depth]);
                } else {
                    batch.put(node_k.clone(), cur);
                    node_cache.insert(node_k, cur);
                }

                cur = parent_arr;
            }

            // root
            if cur == self.default_hashes[0] {
                batch.delete(node_key(0, &[]));
            } else {
                batch.put(node_key(0, &[]), cur);
            }
        }

        self.db.write(batch)?;
        Ok(())
    }
}

/// Produce the canonical default hash vector used by the SMT.
pub fn default_hashes() -> &'static [[u8; 32]] {
    &DEFAULT_HASHES
}

/// Verify a proof (membership or non-membership) against a given root.
/// `proof` is the tuple returned by `proof()`: `(is_member, leaf_hash, siblings)`.
pub fn verify_proof(root: &[u8; 32], key: &[u8], proof: (bool, [u8; 32], Vec<[u8; 32]>)) -> bool {
    let (_is_member, leaf, siblings) = proof;
    let kh = digest(key);

    let mut cur = leaf;
    for (i, sibling) in siblings.into_iter().enumerate() {
        // siblings vector is bottom-up from leaf (depth=256) upwards
        let depth = 256 - i;
        let bit_index = depth - 1;
        let byte_idx = bit_index / 8;
        let bit_in_byte = 7 - (bit_index % 8);
        let bit = ((kh[byte_idx] >> bit_in_byte) & 1u8) == 0;

        let (left, right) = if bit { (cur, sibling) } else { (sibling, cur) };
        let p_arr = hash_node(&left, &right);
        cur = p_arr;
    }

    // After folding up, `cur` should equal the root. For non-membership proofs
    // `is_member` should be false and the leaf used is the default leaf.
    &cur == root
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocksdb::{DB, Options};
    use std::sync::Arc;
    use tempfile::tempdir;

    fn open_test_db(path: &std::path::Path) -> SparseMerkleTree {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).unwrap();
        SparseMerkleTree::new(Arc::new(db))
    }

    #[test]
    fn test_smt_basic_membership() -> Result<()> {
        let dir = tempdir()?;
        let smt = open_test_db(dir.path());

        let key = b"test-key";
        let value = b"test-value";

        // Insert
        smt.insert(&[(key.to_vec(), value.to_vec())])?;

        // Proof
        let (is_member, leaf_hash, siblings) = smt.proof(key)?;
        assert!(is_member);

        // Root
        let root = smt.root_hash()?;

        // Verify
        assert!(verify_proof(&root, key, (is_member, leaf_hash, siblings)));

        Ok(())
    }

    #[test]
    fn test_smt_non_membership() -> Result<()> {
        let dir = tempdir()?;
        let smt = open_test_db(dir.path());

        let key = b"non-existent";

        // Proof
        let (is_member, leaf_hash, siblings) = smt.proof(key)?;
        assert!(!is_member);
        assert_eq!(leaf_hash, default_hashes()[256]);

        // Root (should be empty tree root)
        let root = smt.root_hash()?;
        assert_eq!(root, default_hashes()[0]);

        // Verify
        assert!(verify_proof(&root, key, (is_member, leaf_hash, siblings)));

        Ok(())
    }

    #[test]
    fn test_smt_multi_keys() -> Result<()> {
        let dir = tempdir()?;
        let smt = open_test_db(dir.path());

        let kvs = vec![
            (b"key1".to_vec(), b"val1".to_vec()),
            (b"key2".to_vec(), b"val2".to_vec()),
            (b"key3".to_vec(), b"val3".to_vec()),
        ];

        smt.insert(&kvs)?;

        let root = smt.root_hash()?;

        for (k, _v) in kvs {
            let (is_member, leaf_hash, siblings) = smt.proof(&k)?;
            assert!(is_member);
            assert!(verify_proof(&root, &k, (is_member, leaf_hash, siblings)));
        }

        // Test non-membership of another key
        let other_key = b"key4";
        let (is_member, leaf_hash, siblings) = smt.proof(other_key)?;
        assert!(!is_member);
        assert!(verify_proof(
            &root,
            other_key,
            (is_member, leaf_hash, siblings)
        ));

        Ok(())
    }

    #[test]
    fn test_smt_update() -> Result<()> {
        let dir = tempdir()?;
        let smt = open_test_db(dir.path());

        let key = b"key";
        let val1 = b"val1";
        let val2 = b"val2";

        smt.insert(&[(key.to_vec(), val1.to_vec())])?;
        let root1 = smt.root_hash()?;

        smt.insert(&[(key.to_vec(), val2.to_vec())])?;
        let root2 = smt.root_hash()?;

        assert_ne!(root1, root2);

        let (is_member, leaf_hash, siblings) = smt.proof(key)?;
        assert!(is_member);
        assert!(verify_proof(&root2, key, (is_member, leaf_hash, siblings)));

        Ok(())
    }

    #[test]
    fn test_smt_delete() -> Result<()> {
        let dir = tempdir()?;
        let smt = open_test_db(dir.path());

        let key = b"delete-me";
        let value = b"val";

        smt.insert(&[(key.to_vec(), value.to_vec())])?;
        let root_after_insert = smt.root_hash()?;
        assert_ne!(root_after_insert, default_hashes()[0]);

        smt.delete(&[key.to_vec()])?;
        let root_after_delete = smt.root_hash()?;
        assert_eq!(root_after_delete, default_hashes()[0]);

        let (is_member, _, _) = smt.proof(key)?;
        assert!(!is_member);

        Ok(())
    }
}
