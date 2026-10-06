//! Storage + SMT: RocksDB shared handle + Sparse Merkle Tree operations

use kanari_db_common::open_or_get_db;
use smt::{SparseMerkleTree, verify_proof};

#[test]
fn rocksdb_shared_handle_and_read_write() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("kanari_db");

    let db1 = open_or_get_db(Some(path.clone())).unwrap();
    let db2 = open_or_get_db(Some(path.clone())).unwrap();
    assert!(
        std::sync::Arc::ptr_eq(&db1, &db2),
        "reopening same path should return same Arc"
    );

    db1.put(b"e2e:key", b"e2e:value").unwrap();
    let got = db2.get(b"e2e:key").unwrap().unwrap();
    assert_eq!(got, b"e2e:value");
}

#[test]
fn smt_insert_get_prove_verify() {
    let dir = tempfile::tempdir().unwrap();
    let tree = SparseMerkleTree::open(Some(dir.path().join("smt"))).unwrap();

    tree.insert(&[(b"alice".to_vec(), b"100".to_vec())])
        .unwrap();
    assert_eq!(tree.get(b"alice").unwrap(), Some(b"100".to_vec()));

    let (exists, leaf, proof) = tree.proof(b"alice").unwrap();
    assert!(exists, "alice should exist in tree");
    let root = tree.root_hash().unwrap();
    assert!(verify_proof(&root, b"alice", (exists, leaf, proof.clone())));
    assert!(verify_proof(&root, b"alice", tree.proof(b"alice").unwrap()));

    let (exists, leaf, proof) = tree.proof(b"nobody").unwrap();
    assert!(!exists, "nobody should not exist in tree");
    let root = tree.root_hash().unwrap();
    assert!(verify_proof(&root, b"nobody", (exists, leaf, proof)));
}

#[test]
fn smt_delete_removes_key_but_keeps_others() {
    let dir = tempfile::tempdir().unwrap();
    let tree = SparseMerkleTree::open(Some(dir.path().join("smt"))).unwrap();

    tree.insert(&[
        (b"keep".to_vec(), b"1".to_vec()),
        (b"drop".to_vec(), b"2".to_vec()),
    ])
    .unwrap();
    tree.delete(&[b"drop".to_vec()]).unwrap();

    assert_eq!(tree.get(b"keep").unwrap(), Some(b"1".to_vec()));
    assert_eq!(tree.get(b"drop").unwrap(), None);
}

#[test]
fn smt_root_changes_on_write() {
    let dir = tempfile::tempdir().unwrap();
    let tree = SparseMerkleTree::open(Some(dir.path().join("smt"))).unwrap();

    let before = tree.root_hash().unwrap();
    tree.insert(&[(b"k".to_vec(), b"v".to_vec())]).unwrap();
    let after = tree.root_hash().unwrap();
    assert_ne!(before, after, "root should change after insert");
}
