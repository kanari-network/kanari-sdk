//! Additional Move source files test

#[test]
fn extra_move_sources_exist() {
    let sources_dir = std::path::Path::new("move_package/sources");
    assert!(sources_dir.exists());

    let files = [
        "move_e2e_storage.move",
        "move_e2e_time.move",
        "move_e2e_nft.move",
        "move_e2e_pool.move",
        "move_e2e_registry.move",
    ];

    for f in &files {
        let path = sources_dir.join(f);
        assert!(
            path.exists(),
            "Move source should exist: {}",
            path.display()
        );
    }
}

#[test]
fn extra_move_modules_have_correct_names() {
    let storage = std::fs::read_to_string("move_package/sources/move_e2e_storage.move").unwrap();
    assert!(storage.contains("module kanari_e2e_tests::move_e2e_storage"));

    let time = std::fs::read_to_string("move_package/sources/move_e2e_time.move").unwrap();
    assert!(time.contains("module kanari_e2e_tests::move_e2e_time"));

    let nft = std::fs::read_to_string("move_package/sources/move_e2e_nft.move").unwrap();
    assert!(nft.contains("module kanari_e2e_tests::move_e2e_nft"));

    let pool = std::fs::read_to_string("move_package/sources/move_e2e_pool.move").unwrap();
    assert!(pool.contains("module kanari_e2e_tests::move_e2e_pool"));

    let registry = std::fs::read_to_string("move_package/sources/move_e2e_registry.move").unwrap();
    assert!(registry.contains("module kanari_e2e_tests::move_e2e_registry"));
}

#[test]
fn move_files_contain_expected_logic() {
    let pool = std::fs::read_to_string("move_package/sources/move_e2e_pool.move").unwrap();
    assert!(pool.contains("create_pool"));
    assert!(pool.contains("add_liquidity"));
    assert!(pool.contains("remove_liquidity"));

    let nft = std::fs::read_to_string("move_package/sources/move_e2e_nft.move").unwrap();
    assert!(nft.contains("create_collection"));
    assert!(nft.contains("mint_nft"));
    assert!(nft.contains("NFT"));

    let storage = std::fs::read_to_string("move_package/sources/move_e2e_storage.move").unwrap();
    assert!(storage.contains("KeyValueStore"));
    assert!(storage.contains("set_value"));
    assert!(storage.contains("get_value"));

    let registry = std::fs::read_to_string("move_package/sources/move_e2e_registry.move").unwrap();
    assert!(registry.contains("create_registry"));
    assert!(registry.contains("lookup"));

    let time = std::fs::read_to_string("move_package/sources/move_e2e_time.move").unwrap();
    assert!(time.contains("create_record"));
    assert!(time.contains("Clock"));
}
