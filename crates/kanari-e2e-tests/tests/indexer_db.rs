//! Indexer DB: SQLite storage and query operations

use kanari_indexer::IndexerDB;
use kanari_types::block::Block;
use kanari_types::event::Event;

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

#[test]
fn indexes_object_locked_supply_changed_event_payload() {
    let indexer = kanari_indexer::Indexer::new(kanari_indexer::IndexerConfig {
        in_memory: true,
        ..Default::default()
    })
    .unwrap();
    let type_tag = "0x2::kanari_system::ObjectLockedSupplyChanged";
    let payload = serde_json::json!({
        "object_id": "0xdead",
        "object_type": "0x2::escrow::EscrowDeal<0x2::test::TEST>",
        "token_type": "0x2::test::TEST",
        "amount": 100,
        "delta": 100,
        "action": "locked"
    });
    let event = Event {
        key: b"0xdead".to_vec(),
        sequence_number: 0,
        type_tag: type_tag.to_string(),
        event_data: serde_json::to_vec(&payload).unwrap(),
    };
    let block = Block::new(1, vec![0; 32], vec![0; 32], vec![], vec![event], 1);

    indexer.index_block(&block).unwrap();
    let events = indexer.db().get_events_by_type_tag(type_tag, 10).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].block_height, 1);
    assert_eq!(events[0].type_tag, type_tag);
    let decoded: serde_json::Value =
        serde_json::from_slice(events[0].event_data.as_ref().unwrap()).unwrap();
    assert_eq!(decoded["object_id"], "0xdead");
    assert_eq!(decoded["delta"], 100);
    assert_eq!(decoded["action"], "locked");
}
