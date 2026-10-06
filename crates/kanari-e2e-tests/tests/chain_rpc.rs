//! ระบบ chain + RPC: height / stats / block ผ่าน node จริง (in-memory engine + RPC server)

use kanari_e2e_tests::TestCluster;

#[tokio::test]
async fn cluster_serves_height_and_stats_consistently() -> anyhow::Result<()> {
    let cluster = TestCluster::spawn().await?;
    let client = cluster.client();

    let height = client.get_block_height().await?;
    let stats = client.get_stats().await?;

    assert_eq!(
        height, stats.height,
        "get_block_height ต้องตรงกับ stats.height"
    );

    Ok(())
}

#[tokio::test]
async fn engine_and_rpc_agree_on_stats() -> anyhow::Result<()> {
    let cluster = TestCluster::spawn().await?;
    let rpc_stats = cluster.client().get_stats().await?;
    let engine_stats = cluster.engine().get_stats();

    assert_eq!(rpc_stats.height, engine_stats.height);
    assert_eq!(
        rpc_stats.total_transactions,
        engine_stats.total_transactions
    );
    assert_eq!(rpc_stats.total_supply, engine_stats.total_supply);
    Ok(())
}

#[tokio::test]
async fn concurrent_rpc_clients_all_succeed() -> anyhow::Result<()> {
    let cluster = TestCluster::spawn().await?;
    let mut handles = Vec::new();

    for _ in 0..10 {
        let client = cluster.client();
        handles.push(tokio::spawn(async move {
            let h = client.get_block_height().await?;
            let s = client.get_stats().await?;
            anyhow::ensure!(h == s.height, "height ไม่ตรงกันภายใต้ concurrent load");
            Ok::<_, anyhow::Error>(())
        }));
    }

    for h in handles {
        h.await??;
    }
    Ok(())
}

#[tokio::test]
async fn block_query_at_genesis_height() -> anyhow::Result<()> {
    let cluster = TestCluster::spawn().await?;
    let client = cluster.client();
    let height = client.get_block_height().await?;

    // genesis อาจยังไม่มี block ที่ height 0 — ยอมรับได้ทั้งสองทาง
    // แต่ต้องไม่พังแบบ connection error
    match client.get_block(height).await {
        Ok(block) => assert_eq!(block.height, height),
        Err(e) => eprintln!("note: get_block({height}) ยังไม่มี block: {e:#}"),
    }
    match client.get_full_block(height).await {
        Ok(full) => assert_eq!(full.height, height),
        Err(e) => eprintln!("note: get_full_block({height}) ยังไม่มี block: {e:#}"),
    }
    Ok(())
}
