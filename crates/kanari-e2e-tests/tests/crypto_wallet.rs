//! ระบบ crypto + wallet: keygen / sign / verify ครบทุก classical curve

use kanari_crypto::signatures::sign_message;
use kanari_crypto::{CurveType, generate_keypair, verify_signature};

fn sign_verify_roundtrip(curve: CurveType) {
    let kp = generate_keypair(curve).expect("generate_keypair ต้องสำเร็จ");
    let tagged = kp.tagged_address();
    assert!(
        tagged.starts_with(&format!("{curve}:")),
        "tagged_address ต้องขึ้นต้นด้วย curve: {tagged}"
    );

    let msg = b"kanari e2e: sign me";
    let sk = kp.export_private_key_secure();
    let sig = sign_message(sk.as_str(), msg, curve).expect("sign ต้องสำเร็จ");

    let ok = verify_signature(&tagged, msg, &sig).expect("verify ต้องไม่ error");
    assert!(ok, "{curve} verify ข้อความเดิมต้องผ่าน");

    let tampered = verify_signature(&tagged, b"tampered", &sig).expect("verify ต้องไม่ error");
    assert!(!tampered, "{curve} verify ข้อความปลอมต้องไม่ผ่าน");
}

#[test]
fn ed25519_sign_verify() {
    sign_verify_roundtrip(CurveType::Ed25519);
}

#[test]
fn k256_sign_verify() {
    sign_verify_roundtrip(CurveType::K256);
}

#[test]
fn p256_sign_verify() {
    sign_verify_roundtrip(CurveType::P256);
}

#[test]
fn tagged_address_parses_back_to_curve() {
    let kp = generate_keypair(CurveType::Ed25519).unwrap();
    let tagged = kp.tagged_address();
    let (curve, _addr) =
        kanari_crypto::KeyPair::parse_tagged_address(&tagged).expect("parse ต้องสำเร็จ");
    assert!(matches!(curve, CurveType::Ed25519));
}

#[test]
fn invalid_mnemonic_is_rejected() {
    let bad =
        kanari_crypto::keypair_from_mnemonic("not a real mnemonic phrase", CurveType::Ed25519);
    assert!(bad.is_err(), "mnemonic มั่วต้องถูก reject");
}
