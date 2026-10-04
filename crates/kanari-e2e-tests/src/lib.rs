//! E2E test harness สำหรับระบบต่างๆ ของ kanari-sdk
//!
//! - `TestCluster`: สปิน engine in-memory + RPC server บนพอร์ตสุ่ม แล้วคุยผ่าน `RpcClient`
//! - `external_client()`: คุยกับ node จริงผ่าน env `KANARI_E2E_RPC_URL` (เช่น local/testnet)
//!
//! ตัวอย่าง:
//! ```no_run
//! use kanari_e2e_tests::TestCluster;
//! # #[tokio::main] async fn main() -> anyhow::Result<()> {
//! let cluster = TestCluster::spawn().await?;
//! let height = cluster.client().get_block_height().await?;
//! assert_eq!(height, cluster.client().get_stats().await?.height);
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use std::time::Duration;

use kanari_core::BlockchainEngine;
use kanari_rpc_client::RpcClient;

/// E2E cluster แบบ hermetic: engine in-memory + RPC server จริง
pub struct TestCluster {
    engine: Arc<BlockchainEngine>,
    url: String,
}

impl TestCluster {
    /// สปิน cluster ใหม่ รอจน RPC พร้อม (poll `get_block_height`)
    pub async fn spawn() -> anyhow::Result<Self> {
        let engine = Arc::new(BlockchainEngine::new_in_memory()?);
        Self::spawn_with_engine(engine).await
    }

    /// สปิน cluster จาก engine ที่เตรียมมาแล้ว (เช่น ใส่ genesis / authorities เอง)
    pub async fn spawn_with_engine(engine: Arc<BlockchainEngine>) -> anyhow::Result<Self> {
        let port = free_port().await?;
        let url = format!("http://127.0.0.1:{port}");
        let addr = format!("127.0.0.1:{port}");

        let server_engine = engine.clone();
        tokio::spawn(async move {
            if let Err(e) = kanari_rpc_server::start_server(server_engine, &addr).await {
                tracing::warn!(%e, "e2e rpc server exited");
            }
        });

        let client = RpcClient::new(url.clone());
        wait_for_rpc(&client, Duration::from_secs(15)).await?;

        Ok(Self { engine, url })
    }

    /// RPC endpoint เช่น `http://127.0.0.1:PORT`
    pub fn url(&self) -> &str {
        &self.url
    }

    /// engine ตรงๆ สำหรับ assert ฝั่ง backend เทียบกับ RPC
    pub fn engine(&self) -> &Arc<BlockchainEngine> {
        &self.engine
    }

    /// client พร้อมใช้
    pub fn client(&self) -> RpcClient {
        RpcClient::new(self.url.clone())
    }
}

/// หาพอร์ตว่างแบบ ephemeral (bind แล้วปล่อย ปล่อยให้ server bind ต่อ)
async fn free_port() -> anyhow::Result<u16> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    Ok(listener.local_addr()?.port())
}

/// poll `get_block_height` จนกว่าจะติด หรือ timeout
pub async fn wait_for_rpc(client: &RpcClient, timeout: Duration) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        match client.get_block_height().await {
            Ok(_) => return Ok(()),
            Err(e) => {
                if tokio::time::Instant::now() >= deadline {
                    anyhow::bail!("RPC not ready within {timeout:?}: {e:#}");
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

/// client ไปยัง node จริง ถ้าตั้ง `KANARI_E2E_RPC_URL` ไว้ ไม่งั้นคืน `None`
/// ใช้กับ flow ที่ต้องใช้เงินจริง/faucet (transfer, publish, call function)
pub fn external_client() -> Option<RpcClient> {
    std::env::var("KANARI_E2E_RPC_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(RpcClient::new)
}

/// ข้าม test แบบนุ่มนวลเมื่อไม่มี node จริง (return `None` ให้ caller `return` ทันที)
/// ตัวอย่าง: `let client = kanari_e2e_tests::external_or_skip!();`
#[macro_export]
macro_rules! external_or_skip {
    () => {
        match $crate::external_client() {
            Some(c) => c,
            None => {
                eprintln!("SKIP: set KANARI_E2E_RPC_URL to run this test against a live node");
                return;
            }
        }
    };
}
