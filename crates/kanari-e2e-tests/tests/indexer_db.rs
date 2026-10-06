//! Indexer DB: SQLite storage and query operations

use kanari_indexer::IndexerDB;
use kanari_types::block::Block;

#[test]
fn index_genesis_block_and_read_back() {
    let db = IndexerDB::new_in_memory().unwrap();
    let genesis = Block::genesis();

    db.insert_block(&genesis).unwrap();

    let back = db
        .get_block_by_height(0)
        .unwrap()
        .expect("genesis block should be retrieved");
    assert_eq!(back.height, 0);
    assert_eq!(back.tx_count, 0);
}

#[test]
fn missing_height_returns_none() {
    let db = IndexerDB::new_in_memory().unwrap();
    assert!(
        db.get_block_by_height(999_999).unwrap().is_none(),
        "missing height should return None"
    );
}

#[test]
fn latest_height_tracks_inserts() {
    let db = IndexerDB::new_in_memory().unwrap();
    let genesis = Block::genesis();
    db.insert_block(&genesis).unwrap();
    let latest = db.get_latest_height().unwrap();
    assert_eq!(latest, 0);
}

#[test]
fn insert_is_idempotent_for_same_height() {
    let db = IndexerDB::new_in_memory().unwrap();
    let genesis = Block::genesis();
    db.insert_block(&genesis).unwrap();
    db.insert_block(&genesis).unwrap();
    assert!(db.get_block_by_height(0).unwrap().is_some());
}
