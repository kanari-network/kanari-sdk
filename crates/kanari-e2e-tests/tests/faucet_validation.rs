//! ระบบ Faucet: ทดสอบ validation ก่อนส่ง request
//!
//! ตรวจสอบ:
//! 1. ตรวจ amount ที่มีรูปแบบผิดพลาด (ทศนิยมเกิน, ค่าติดลบ, สตริงมั่ว)
//! 2. ตรวจ recipient address ที่ผิด spec (ต้องเป็น 0x ตามด้วย 64 hex chars)
//! 3. ตรวจกรณีไม่มี password ป้องกันการเรียกแบบ unauthorized

use kanari_faucet::request_from_dev;

#[tokio::test]
async fn faucet_rejects_invalid_amount_before_network() {
    let dummy_rpc = "http://127.0.0.1:9"; // port ที่ไม่มี server อยู่

    // ทดสอบสตริง amount มั่ว
    let res = request_from_dev(None, Some("pwd"), Some("0x123"), "not_a_number", dummy_rpc).await;
    assert!(res.is_err());
    let err_str = res.unwrap_err().to_string();
    assert!(
        err_str.contains("Invalid faucet amount") || err_str.contains("invalid KANARI amount"),
        "error message ควรบอกเรื่อง amount: {err_str}"
    );

    // ทดสอบ amount ติดลบ
    let res = request_from_dev(None, Some("pwd"), Some("0x123"), "-10.5", dummy_rpc).await;
    assert!(res.is_err());

    // ทดสอบทศนิยมเกิน 9 ตำแหน่ง (เกิน mist precision)
    let res = request_from_dev(None, Some("pwd"), Some("0x123"), "1.0000000001", dummy_rpc).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn faucet_rejects_malformed_recipient_address() {
    let dummy_rpc = "http://127.0.0.1:9";

    // address สั้นเกินไป
    let res = request_from_dev(None, Some("pwd"), Some("0xabc"), "1.0", dummy_rpc).await;
    assert!(res.is_err());
    let err_str = res.unwrap_err().to_string();
    assert!(
        err_str.contains("Invalid recipient address format"),
        "ควรแจ้ง recipient format: {err_str}"
    );

    // address ไม่มี 0x นำหน้า
    let non_hex_addr = "3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b314600";
    let res = request_from_dev(None, Some("pwd"), Some(non_hex_addr), "1.0", dummy_rpc).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn faucet_rejects_missing_password() {
    let dummy_rpc = "http://127.0.0.1:9";
    let valid_len_addr = "0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146";

    // เอา env KANARI_PASSWORD ออกถ้ามี
    let prev = std::env::var("KANARI_PASSWORD").ok();
    unsafe {
        std::env::remove_var("KANARI_PASSWORD");
    }

    let res = request_from_dev(None, None, Some(valid_len_addr), "1.0", dummy_rpc).await;

    // คืนค่า env กลับถ้ามี
    if let Some(p) = prev {
        unsafe {
            std::env::set_var("KANARI_PASSWORD", p);
        }
    }

    assert!(res.is_err());
    let err_str = res.unwrap_err().to_string();
    assert!(
        err_str.contains("Dev password not provided") || err_str.contains("KANARI_PASSWORD"),
        "ควรแจ้ง password ขาด: {err_str}"
    );
}
