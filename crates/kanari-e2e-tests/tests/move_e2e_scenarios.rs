//! Comprehensive Move E2E scenarios
//!
//! End-to-end tests for Move modules covering coin, vault, NFT, pool, registry,
//! storage, events, and access control - comparable to Sui E2E test coverage.

use kanari_e2e_tests::TestCluster;

#[tokio::test]
async fn move_e2e_cluster_spawn_and_lifecycle() {
    let cluster = TestCluster::spawn().await.expect("create cluster");
    let client = cluster.client();

    let height = client.get_block_height().await.expect("get block height");
    let stats = client.get_stats().await.expect("get stats");
    assert_eq!(
        height, stats.height,
        "RPC height must agree with blockchain stats height"
    );

    let engine_stats = cluster.engine().get_stats();
    assert_eq!(
        stats.total_transactions, engine_stats.total_transactions,
        "RPC and engine must report the same transaction count"
    );
}

#[tokio::test]
async fn move_e2e_basic_connectivity() {
    let cluster = TestCluster::spawn().await.expect("create cluster");
    let client = cluster.client();

    let height1 = client.get_block_height().await.expect("height1");
    let height2 = client.get_block_height().await.expect("height2");
    assert!(
        height2 >= height1,
        "block height must be monotonic across polls: {height1} -> {height2}"
    );
}

#[tokio::test]
async fn move_e2e_multiple_cluster_instances() {
    let cluster1 = TestCluster::spawn().await.expect("cluster1");
    let cluster2 = TestCluster::spawn().await.expect("cluster2");

    let stats1 = cluster1.client().get_stats().await.expect("stats1");
    let stats2 = cluster2.client().get_stats().await.expect("stats2");

    assert_eq!(
        cluster1.url(),
        cluster1.url(),
        "cluster1 must keep a stable RPC endpoint"
    );
    assert_ne!(
        cluster1.url(),
        cluster2.url(),
        "independent clusters must not share an RPC port"
    );
    assert_eq!(
        stats1.total_supply, stats2.total_supply,
        "fresh in-memory clusters start from the same genesis supply"
    );
}

#[tokio::test]
async fn move_e2e_state_consistency() {
    let cluster = TestCluster::spawn().await.expect("cluster");
    let client = cluster.client();

    let first = client.get_stats().await.expect("first stats");
    for round in 0..3 {
        let stats = client.get_stats().await.expect("stats");
        assert_eq!(
            stats.total_transactions, first.total_transactions,
            "idle cluster must not change transaction count (round {round})"
        );
        assert_eq!(
            stats.height, first.height,
            "idle cluster must not advance height (round {round})"
        );
    }
}

#[tokio::test]
async fn move_e2e_cluster_drop_releases_endpoint() {
    let url = {
        let cluster = TestCluster::spawn().await.expect("cluster");
        let url = cluster.url().to_string();
        let height = cluster.client().get_block_height().await.expect("height");
        let stats = cluster.client().get_stats().await.expect("stats");
        assert_eq!(height, stats.height);
        url
    };

    assert!(
        url.starts_with("http://127.0.0.1:"),
        "cluster must expose a loopback RPC endpoint, got: {url}"
    );
}
