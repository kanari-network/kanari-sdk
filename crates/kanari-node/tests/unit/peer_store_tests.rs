use super::*;
use kanari_types::error::KanariUnwrapExt;
use tempfile::TempDir;

#[test]
fn test_peer_store_save_load() {
    let temp_dir = TempDir::new().invariant("failed to create temp dir");
    let file_path = temp_dir.path().join("peers.json");

    let mut store = PeerStore::new(file_path.clone());
    let peer_id = PeerId::random();
    store.add_peer(peer_id, vec![]);
    store.save().invariant("failed to save peer store");

    let loaded = PeerStore::load(file_path).invariant("failed to load peer store");
    assert_eq!(loaded.peers.len(), 1);
}
