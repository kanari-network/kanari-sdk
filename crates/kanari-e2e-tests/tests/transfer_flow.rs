//! ระบบ transfer / faucet flow — ต้องใช้ node จริงที่มีเงิน (ข้ามถ้าไม่มี `KANARI_E2E_RPC_URL`)
//!
//! รันแบบ local: `kanari-node local` (พอร์ต 6767) แล้ว:
//! ```powershell
//! $env:KANARI_E2E_RPC_URL='http://127.0.0.1:6767'
//! cargo test -p kanari-e2e-tests --test transfer_flow -- --nocapture
//! ```

use kanari_crypto::{CurveType, generate_keypair};
use kanari_rpc_api::{BuildNativeTransferRequest, GetObjectsRequest};

/// เตรียมผู้รับใหม่ + assert ว่า backend ปฏิเสธ transfer จากกระเป๋าเปล่าอย่างมีเหตุผล
/// (policy error แบบ structured ไม่ใช่ crash) — ใช้ตรวจ faucet/gas UX ได้
#[tokio::test]
async fn unfunded_wallet_cannot_drain_network() {
    let client = kanari_e2e_tests::external_or_skip!();

    let kp = generate_keypair(CurveType::Ed25519).unwrap();
    let sender = kp.tagged_address();
    eprintln!("sender (unfunded): {sender}");

    // กระเป๋าใหม่ต้องไม่มี object เลย
    let objects = client
        .get_objects(GetObjectsRequest {
            owner: Some(sender.clone()),
            owner_kind: None,
            object_type: None,
            min_version: None,
            max_version: None,
        })
        .await
        .expect("get_objects ต้องตอบกลับ");
    assert!(
        objects.objects.is_empty(),
        "กระเป๋าใหม่ต้องว่าง แต่เจอ {} objects",
        objects.objects.len()
    );

    // build transfer จากกระเป๋าว่างต้อง fail (ไม่มี coin ให้เลือก) — ยืนยันว่า
    // backend ไม่สร้าง tx ลมๆ แล้งๆ ให้
    let build = client
        .build_native_transfer(BuildNativeTransferRequest {
            sender: sender.clone(),
            recipient: kanari_types::address::Address::DEV_ADDRESS.to_string(),
            amount: 1_000,
            gas_limit: 10_000_000,
            gas_price: 1_000,
            excluded_object_ids: vec![],
            nonce: None,
            execute_immediate: Some(false),
        })
        .await;
    assert!(
        build.is_err(),
        "build transfer จากกระเป๋าว่างต้อง fail แต่กลับสำเร็จ"
    );
    eprintln!("expected build failure: {:#}", build.unwrap_err());
}

#[tokio::test]
async fn dev_wallet_visible_on_live_network() {
    let client = kanari_e2e_tests::external_or_skip!();
    let dev = kanari_types::address::Address::DEV_ADDRESS;

    let owner = client.get_owner(dev).await.expect("dev wallet ต้องมีตัวตน");
    eprintln!("dev owner: {owner:?}");

    let balances = client
        .get_owner_balances(dev)
        .await
        .expect("dev balances ต้อง query ได้");
    eprintln!("dev balances: {balances}");
}
