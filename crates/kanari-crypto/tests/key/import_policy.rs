use kanari_crypto::keys::{
    CurveType, KANAHYBRID_PREFIX, KANAPQC_PREFIX, KeyError, extract_raw_key, generate_keypair,
    keypair_from_private_key,
};
use proptest::prelude::*;

#[test]
fn import_rejects_pqc_secret_without_public_key_for_all_pqc_curves() {
    for curve in [
        CurveType::Dilithium2,
        CurveType::Dilithium3,
        CurveType::Dilithium5,
        CurveType::SphincsPlusSha256Robust,
    ] {
        let secret_only = format!("{}deadbeef", KANAPQC_PREFIX);
        assert!(matches!(
            keypair_from_private_key(&secret_only, curve),
            Err(KeyError::InvalidPrivateKey)
        ));
    }
}

#[test]
fn import_preserves_generated_pqc_and_hybrid_keypairs() {
    for curve in [
        CurveType::Dilithium2,
        CurveType::Dilithium3,
        CurveType::Dilithium5,
        CurveType::SphincsPlusSha256Robust,
        CurveType::Ed25519Dilithium3,
        CurveType::K256Dilithium3,
    ] {
        let original = generate_keypair(curve).unwrap();
        let imported = keypair_from_private_key(&original.private_key, curve).unwrap();

        assert_eq!(imported.public_key, original.public_key);
        assert_eq!(imported.address, original.address);
        assert_eq!(imported.pqc_public_key, original.pqc_public_key);
    }
}

proptest! {
    #[test]
    fn malformed_classical_private_keys_never_panic(input in ".{0,256}") {
        for curve in [CurveType::K256, CurveType::P256, CurveType::Ed25519] {
            let result = keypair_from_private_key(&input, curve);
            if result.is_ok() {
                prop_assert!(input.starts_with("kanari") || hex::decode(extract_raw_key(&input)).is_ok());
            }
        }
    }

    #[test]
    fn malformed_pqc_imports_fail_closed(secret in "[0-9a-f]{0,128}", public in ".{0,128}") {
        let key = format!("{}{}:{}", KANAPQC_PREFIX, secret, public);
        let result = keypair_from_private_key(&key, CurveType::Dilithium3);

        if result.is_err() {
            prop_assert!(matches!(result, Err(KeyError::InvalidPrivateKey)));
        }
    }

    #[test]
    fn malformed_hybrid_imports_fail_closed(classical in ".{0,128}", pqc in ".{0,128}") {
        let key = format!("{}{}:{}", KANAHYBRID_PREFIX, classical, pqc);
        let result = keypair_from_private_key(&key, CurveType::Ed25519Dilithium3);

        if result.is_err() {
            prop_assert!(matches!(result, Err(KeyError::InvalidPrivateKey)));
        }
    }
}
