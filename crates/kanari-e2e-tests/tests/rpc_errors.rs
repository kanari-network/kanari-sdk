//! ระบบ RPC error handling: hash มั่ว / object ไม่มี / method ผิด ต้องได้ structured error ไม่ใช่ hang

use kanari_e2e_tests::TestCluster;

#[tokio::test]
async fn unknown_transaction_hash_returns_error_not_hang() -> anyhow::Result<()> {
    let cluster = TestCluster::spawn().await?;
    let res = cluster.client().get_transaction("0xdeadbeef").await;
    assert!(res.is_err(), "hash มั่วต้อง error กลับมา");
    Ok(())
}

#[tokio::test]
async fn unknown_object_returns_error() -> anyhow::Result<()> {
    let cluster = TestCluster::spawn().await?;
    let res = cluster
        .client()
        .get_object("0x00000000000000000000000000000001")
        .await;
    assert!(res.is_err(), "object ที่ไม่มีต้อง error กลับมา");
    Ok(())
}

#[tokio::test]
async fn unknown_block_height_returns_error_or_empty() -> anyhow::Result<()> {
    let cluster = TestCluster::spawn().await?;
    // height สูงลิ่วยังไงก็ต้องตอบกลับ (error หรือ block ว่างก็ได้ แต่ห้าม hang)
    let res = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        cluster.client().get_block(u64::MAX),
    )
    .await;
    assert!(res.is_ok(), "RPC ต้องตอบภายใน timeout ไม่ hang");
    Ok(())
}

#[tokio::test]
async fn malformed_url_fails_fast() {
    let client = kanari_rpc_client::RpcClient::new("http://127.0.0.1:1");
    let res = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        client.get_block_height(),
    )
    .await;
    // timeout ผ่าน = ได้ response (ซึ่งควรเป็น error เชื่อมต่อ) — สำคัญคือไม่ hang เกิน 10s
    assert!(res.is_ok());
    assert!(res.unwrap().is_err());
}
