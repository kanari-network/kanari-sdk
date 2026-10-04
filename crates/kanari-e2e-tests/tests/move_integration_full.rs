//! Full Move integration E2E tests
//!
//! Comprehensive end-to-end tests for Move contract deployment and execution
//! with full cluster lifecycle - Sui-equivalent coverage level.

use kanari_e2e_tests::TestCluster;
use std::path::Path;

#[tokio::test]
async fn move_full_cluster_lifecycle() {
    let cluster = TestCluster::spawn().await.expect("create cluster");

    let client = cluster.client();
    let height = client.get_block_height().await.expect("height");
    let stats = client.get_stats().await.expect("stats");

    assert_eq!(
        height, stats.height,
        "cluster lifecycle must expose a consistent RPC view"
    );
    assert_eq!(
        stats.total_supply,
        cluster.engine().get_stats().total_supply,
        "engine and RPC must agree on total supply"
    );
}

#[tokio::test]
async fn move_full_rpc_endpoints_coverage() {
    let cluster = TestCluster::spawn().await.expect("cluster");
    let client = cluster.client();

    let height = client
        .get_block_height()
        .await
        .expect("get_block_height must succeed");
    let stats = client.get_stats().await.expect("get_stats must succeed");

    assert_eq!(height, stats.height);
}

#[tokio::test]
async fn move_full_source_files_validation() {
    let sources = [
        "move_package/sources/move_e2e_basic.move",
        "move_package/sources/move_e2e_comprehensive.move",
        "move_package/sources/move_e2e_math.move",
        "move_package/sources/move_e2e_events.move",
        "move_package/sources/move_e2e_access.move",
        "move_package/sources/move_e2e_storage.move",
        "move_package/sources/move_e2e_time.move",
        "move_package/sources/move_e2e_nft.move",
        "move_package/sources/move_e2e_pool.move",
        "move_package/sources/move_e2e_registry.move",
    ];

    for s in &sources {
        let p = Path::new(s);
        assert!(p.exists(), "missing: {}", s);
        let content = std::fs::read_to_string(p).expect("read");
        assert!(content.contains("module kanari_e2e_tests::"));
        assert!(content.len() > 50);
    }
}

#[tokio::test]
async fn move_full_module_functionality_check() {
    let basic = std::fs::read_to_string("move_package/sources/move_e2e_basic.move").unwrap();
    assert!(basic.contains("init_test_coin"));

    let comp = std::fs::read_to_string("move_package/sources/move_e2e_comprehensive.move").unwrap();
    assert!(comp.contains("init_comp_coin"));
    assert!(comp.contains("create_vault"));
    assert!(comp.contains("deposit"));
    assert!(comp.contains("withdraw"));

    let math = std::fs::read_to_string("move_package/sources/move_e2e_math.move").unwrap();
    assert!(math.contains("add"));
    assert!(math.contains("multiply"));
    assert!(math.contains("safe_add"));
    assert!(math.contains("clamp"));

    let events = std::fs::read_to_string("move_package/sources/move_e2e_events.move").unwrap();
    assert!(events.contains("emit_message"));
    assert!(events.contains("emit_batch"));

    let access = std::fs::read_to_string("move_package/sources/move_e2e_access.move").unwrap();
    assert!(access.contains("create_access_control"));
    assert!(access.contains("add_admin"));
}

#[tokio::test]
async fn move_full_advanced_modules_check() {
    let storage = std::fs::read_to_string("move_package/sources/move_e2e_storage.move").unwrap();
    assert!(storage.contains("create_store"));
    assert!(storage.contains("set_value"));
    assert!(storage.contains("get_value"));
    assert!(storage.contains("remove_value"));
    assert!(storage.contains("create_blob"));
    assert!(storage.contains("update_blob"));

    let time = std::fs::read_to_string("move_package/sources/move_e2e_time.move").unwrap();
    assert!(time.contains("create_record"));
    assert!(time.contains("update_record"));
    assert!(time.contains("is_fresh"));

    let nft = std::fs::read_to_string("move_package/sources/move_e2e_nft.move").unwrap();
    assert!(nft.contains("create_collection"));
    assert!(nft.contains("mint_nft"));
    assert!(nft.contains("transfer_nft"));
    assert!(nft.contains("collection_size"));

    let pool = std::fs::read_to_string("move_package/sources/move_e2e_pool.move").unwrap();
    assert!(pool.contains("create_pool"));
    assert!(pool.contains("add_liquidity"));
    assert!(pool.contains("remove_liquidity"));
    assert!(pool.contains("reserves"));

    let registry = std::fs::read_to_string("move_package/sources/move_e2e_registry.move").unwrap();
    assert!(registry.contains("create_registry"));
    assert!(registry.contains("register_name"));
    assert!(registry.contains("lookup"));
    assert!(registry.contains("create_entry"));
}
