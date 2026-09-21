// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Calibration harness for crypto verify costs (ignored by default):
//! `cargo test -p kanari-system-natives --release crypto_calibrate -- --ignored --nocapture`
//!
//! Fixtures are generated fresh per run (keygen + sign in setup, never timed)
//! and every fixture is asserted valid before timing, so a passing run proves
//! the numbers measure the real accept-path verification cost.
#![allow(clippy::print_stdout, clippy::type_complexity)]

use std::hint::black_box;
use std::time::Instant;

fn bench<R>(name: &str, iters: u64, mut f: impl FnMut() -> R) {
    for _ in 0..10 {
        black_box(f());
    }
    let t = Instant::now();
    for _ in 0..iters {
        black_box(f());
    }
    println!(
        "{name:32} {:12.1} ns/op",
        t.elapsed().as_nanos() as f64 / iters as f64
    );
}

fn random_32() -> [u8; 32] {
    use rand::{TryRng, rngs::SysRng};
    let mut bytes = [0u8; 32];
    SysRng.try_fill_bytes(&mut bytes).expect("os randomness");
    bytes
}

#[test]
#[ignore]
fn crypto_calibrate() {
    use kanari_crypto::cryptos::{
        NativeEcdsaHash, decompress_secp256k1_pubkey, recover_secp256k1_public_key,
        verify_ed25519_native, verify_p256_sha256_native, verify_rs256_prehash_native,
        verify_secp256k1_ecdsa_native,
    };

    println!("--- crypto verify calibration ---");
    let msg = b"kanari move crypto calibration vector";

    // --- secp256k1 (k1): ecrecover / decompress / verify ---
    let sk = secp256k1::SecretKey::from_secret_bytes(random_32()).unwrap();
    let pk = secp256k1::PublicKey::from_secret_key(&sk);
    let digest: [u8; 32] = {
        use sha2::Digest;
        sha2::Sha256::digest(msg).into()
    };
    let message = secp256k1::Message::from_digest(digest);
    let rec_sig = secp256k1::ecdsa::RecoverableSignature::sign_ecdsa_recoverable(message, &sk);
    let (_, rec_compact) = rec_sig.serialize_compact();
    // `recover_secp256k1_public_key` returns the 33-byte compressed key
    // (`PublicKey::serialize`), so compare against the compressed encoding.
    let expected_c = pk.serialize().to_vec();
    // Find the recovery id by trial (stable across secp256k1 releases).
    let mut sig65 = None;
    for v in 0..=3u8 {
        let mut cand = [0u8; 65];
        cand[..64].copy_from_slice(&rec_compact);
        cand[64] = v;
        if recover_secp256k1_public_key(&cand, &digest)
            .map(|p| p == expected_c)
            .unwrap_or(false)
        {
            sig65 = Some(cand);
            break;
        }
    }
    let sig65 = sig65.expect("k1 recovery id");
    let k1_pub_c = pk.serialize().to_vec();
    let k1_sig64 = rec_compact.to_vec();
    assert!(recover_secp256k1_public_key(&sig65, &digest).is_ok());
    bench("k1_ecrecover", 2_000, || {
        recover_secp256k1_public_key(black_box(&sig65), black_box(&digest)).unwrap()
    });
    bench("k1_decompress", 2_000, || {
        decompress_secp256k1_pubkey(black_box(&k1_pub_c)).unwrap()
    });
    assert!(
        verify_secp256k1_ecdsa_native(&k1_pub_c, &k1_sig64, msg, NativeEcdsaHash::Sha256).unwrap()
    );
    bench("k1_verify", 2_000, || {
        verify_secp256k1_ecdsa_native(
            black_box(&k1_pub_c),
            black_box(&k1_sig64),
            black_box(msg),
            NativeEcdsaHash::Sha256,
        )
        .unwrap()
    });

    // --- P-256 (r1) ---
    let r1_sk = p256::ecdsa::SigningKey::from_slice(&random_32())
        .unwrap_or_else(|_| p256::ecdsa::SigningKey::from_slice(&[0x42u8; 32]).unwrap());
    let r1_pub = r1_sk
        .verifying_key()
        .to_sec1_point(false)
        .as_bytes()
        .to_vec();
    let r1_sig = {
        use p256::ecdsa::signature::Signer;
        let sig: p256::ecdsa::Signature = r1_sk.sign(msg);
        sig.to_der().as_bytes().to_vec()
    };
    assert!(verify_p256_sha256_native(&r1_pub, &r1_sig, msg).unwrap());
    bench("r1_verify", 2_000, || {
        verify_p256_sha256_native(black_box(&r1_pub), black_box(&r1_sig), black_box(msg)).unwrap()
    });

    // --- Ed25519 (raw native path) ---
    let ed_sk = ed25519_dalek::SigningKey::from_bytes(&random_32());
    let ed_pub = ed_sk.verifying_key().to_bytes().to_vec();
    let ed_sig = {
        use ed25519_dalek::Signer;
        ed_sk.sign(msg).to_bytes().to_vec()
    };
    assert!(verify_ed25519_native(&ed_pub, &ed_sig, msg));
    bench("ed25519_verify", 5_000, || {
        verify_ed25519_native(black_box(&ed_pub), black_box(&ed_sig), black_box(msg))
    });

    // --- Post-quantum + hybrids via wallet-compatible KeyPairs ---
    use kanari_crypto::keys::{CurveType, generate_keypair};
    // (gas field, curve, sign fn, verify fn, iters)
    let pqc_cases: &[(
        &str,
        CurveType,
        fn(&str, &[u8]) -> Result<Vec<u8>, kanari_crypto::signatures::SignatureError>,
        fn(&str, &[u8], &[u8]) -> Result<bool, kanari_crypto::signatures::SignatureError>,
        u64,
    )] = &[
        (
            "dilithium2_verify",
            CurveType::Dilithium2,
            kanari_crypto::signatures::dilithium2::sign_message_dilithium2,
            kanari_crypto::signatures::dilithium2::verify_signature_dilithium2,
            200,
        ),
        (
            "dilithium3_verify",
            CurveType::Dilithium3,
            kanari_crypto::signatures::dilithium3::sign_message_dilithium3,
            kanari_crypto::signatures::dilithium3::verify_signature_dilithium3,
            200,
        ),
        (
            "dilithium5_verify",
            CurveType::Dilithium5,
            kanari_crypto::signatures::dilithium5::sign_message_dilithium5,
            kanari_crypto::signatures::dilithium5::verify_signature_dilithium5,
            100,
        ),
        (
            "sphincs_verify",
            CurveType::SphincsPlusSha256Robust,
            kanari_crypto::signatures::sphincs::sign_message_sphincs,
            kanari_crypto::signatures::sphincs::verify_signature_sphincs,
            10,
        ),
        (
            "falcon512_verify",
            CurveType::Falcon512,
            kanari_crypto::signatures::falcon::sign_message_falcon512,
            kanari_crypto::signatures::falcon::verify_signature_falcon512,
            100,
        ),
        (
            "falcon1024_verify",
            CurveType::Falcon1024,
            kanari_crypto::signatures::falcon::sign_message_falcon1024,
            kanari_crypto::signatures::falcon::verify_signature_falcon1024,
            50,
        ),
    ];
    for (name, curve, sign, verify, iters) in pqc_cases {
        let kp = generate_keypair(*curve).expect("pqc keygen");
        let sig = sign(&kp.private_key, msg).expect("pqc sign");
        // verify fns take the (prefixed) public key, not the hashed `address`.
        assert!(verify(&kp.public_key, msg, &sig).unwrap(), "{name} fixture");
        bench(name, *iters, || {
            verify(black_box(&kp.public_key), black_box(msg), black_box(&sig)).unwrap()
        });
    }
    let hybrid_cases: &[(
        &str,
        CurveType,
        fn(&str, &[u8]) -> Result<Vec<u8>, kanari_crypto::signatures::SignatureError>,
        fn(&str, &[u8], &[u8]) -> Result<bool, kanari_crypto::signatures::SignatureError>,
    )] = &[
        (
            "hybrid_ed25519_dilithium3",
            CurveType::Ed25519Dilithium3,
            kanari_crypto::signatures::hybrid::sign_message_hybrid_ed25519,
            kanari_crypto::signatures::hybrid::verify_ed25519dilithium3,
        ),
        (
            "hybrid_k256_dilithium3",
            CurveType::K256Dilithium3,
            kanari_crypto::signatures::hybrid::sign_message_hybrid_k256,
            kanari_crypto::signatures::hybrid::verify_k256dilithium3,
        ),
    ];
    for (name, curve, sign, verify) in hybrid_cases {
        let kp = generate_keypair(*curve).expect("hybrid keygen");
        let sig = sign(&kp.private_key, msg).expect("hybrid sign");
        // Hybrid verifiers expect "classical_pub:pqc_pub", not the hashed address.
        assert!(verify(&kp.public_key, msg, &sig).unwrap(), "{name} fixture");
        bench(name, 100, || {
            verify(black_box(&kp.public_key), black_box(msg), black_box(&sig)).unwrap()
        });
    }

    // --- RSASSA-PKCS1-v1_5 / SHA-256, RFC 7515 Appendix A vector ---
    // (keygen would need an RSA-compatible RNG; the repo's own Move tests
    // already pin this public vector, so reuse it directly.)
    {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        use sha2::Digest;
        let n = URL_SAFE_NO_PAD.decode("ofgWCuLjybRlzo0tZWJjNiuSfb4p4fAkd_wWJcyQoTbji9k0l8W26mPddxHmfHQp-Vaw-4qPCJrcS2mJPMEzP1Pt0Bm4d4QlL-yRT-SFd2lZS-pCgNMsD1W_YpRPEwOWvG6b32690r2jZ47soMZo9wGzjb_7OMg0LOL-bSf63kpaSHSXndS5z5rexMdbBYUsLA9e-KXBdQOS-UTo7WTBEMa2R2CapHg665xsmtdVMTBQY4uDZlxvb3qCo5ZwKh9kG4LT6_I5IhlJH7aGhyxXFvUK-DWNmoudF8NAco9_h9iaGNj8q2ethFkMLs91kzk2PAcDTW9gb54h4FRWyuXpoQ").unwrap();
        let e = URL_SAFE_NO_PAD.decode("AQAB").unwrap();
        let sig = URL_SAFE_NO_PAD.decode("cC4hiUPoj9Eetdgtv3hF80EGrhuB__dzERat0XF9g2VtQgr9PJbu3XOiZj5RZmh7AAuHIm4Bh-0Qc_lF5YKt_O8W2Fp5jujGbds9uJdbF9CUAr7t1dnZcAcQjbKBYNX4BAynRFdiuB--f_nZLgrnbyTyWzO75vRK5h6xBArLIARNPvkSjtQBMHlb1L07Qe7K0GarZRmB_eSN9383LcOLn6_dO--xi12jzDwusC-eOkHWEsqtFZESc6BfI7noOPqvhJ1phCnvWh6IeYI2w9QOYEUipUTI8np6LbgGY9Fs98rqVt5AXLIhWkWywlVmtVrBp0igcN_IoypGlUPQGe77Rw").unwrap();
        // RFC 7515 signing input (ASCII).
        let rfc_msg = b"eyJhbGciOiJSUzI1NiJ9.eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ";
        let rfc_hash = sha2::Sha256::digest(rfc_msg).to_vec();
        assert!(verify_rs256_prehash_native(&n, &e, &rfc_hash, &sig));
        bench("rs256_verify_rfc", 500, || {
            verify_rs256_prehash_native(
                black_box(&n),
                black_box(&e),
                black_box(&rfc_hash),
                black_box(&sig),
            )
        });
        // Hash slope (the native hashes the full message, up to 1MB).
        let msg_1k = vec![9u8; 1024];
        let msg_1m = vec![9u8; 1_048_576];
        bench("sha256_1KB", 2_000, || {
            sha2::Sha256::digest(black_box(&msg_1k)).to_vec()
        });
        bench("sha256_1MB", 20, || {
            sha2::Sha256::digest(black_box(&msg_1m)).to_vec()
        });
        // PQC message-size slope check (1MB input): does verify time move?
        let big = vec![3u8; 1_048_576];
        let kp_d3 = generate_keypair(CurveType::Dilithium3).expect("d3 keygen");
        let sig_big = kanari_crypto::signatures::dilithium3::sign_message_dilithium3(
            &kp_d3.private_key,
            &big,
        )
        .expect("d3 big sign");
        assert!(
            kanari_crypto::signatures::dilithium3::verify_signature_dilithium3(
                &kp_d3.public_key,
                &big,
                &sig_big
            )
            .unwrap()
        );
        bench("dilithium3_verify_1MB", 20, || {
            kanari_crypto::signatures::dilithium3::verify_signature_dilithium3(
                black_box(&kp_d3.public_key),
                black_box(&big),
                black_box(&sig_big),
            )
            .unwrap()
        });
    }

    // --- zkLogin JWT full path (mint once with deterministic RSA key) ---
    {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        use kanari_crypto::signatures::zklogin::{
            ISS_GOOGLE, JwksDocument, compute_nonce, derive_zklogin_address_v2,
            verify_jwt_with_jwks,
        };

        struct Xor(u64);
        impl rsa::rand_core::RngCore for Xor {
            fn next_u32(&mut self) -> u32 {
                self.next_u64() as u32
            }
            fn next_u64(&mut self) -> u64 {
                let mut x = self.0 | 1;
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                self.0 = x;
                x
            }
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                let mut i = 0;
                while i < dest.len() {
                    let b = self.next_u64().to_le_bytes();
                    let n = core::cmp::min(8, dest.len() - i);
                    dest[i..i + n].copy_from_slice(&b[..n]);
                    i += n;
                }
            }
            fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rsa::rand_core::Error> {
                self.fill_bytes(dest);
                Ok(())
            }
        }
        impl rsa::rand_core::CryptoRng for Xor {}
        use rsa::pkcs1::EncodeRsaPrivateKey as _;
        use rsa::traits::PublicKeyParts as _;

        let mut rng = Xor(0xBEEF);
        let sk = rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let pk = rsa::RsaPublicKey::from(&sk);
        let n_b64 = URL_SAFE_NO_PAD.encode(pk.n().to_bytes_be());
        let e_b64 = URL_SAFE_NO_PAD.encode(pk.e().to_bytes_be());
        let der = sk.to_pkcs1_der().unwrap().as_bytes().to_vec();

        // Nonce binds a fixed ephemeral session (same shape as login flow).
        let eph_pub = [42u8; 32];
        let nonce = compute_nonce(&eph_pub, 1000, &[7u8; 32]);
        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some("bench-1".to_string());
        let claims = serde_json::json!({
            "iss": ISS_GOOGLE,
            "aud": "kanari-test-client",
            "sub": "1234",
            "exp": 2000000000u64,
            "nonce": nonce,
        });
        let key = jsonwebtoken::EncodingKey::from_rsa_der(&der);
        let jwt = jsonwebtoken::encode(&header, &claims, &key).unwrap();
        let jwks: JwksDocument = serde_json::from_str(&format!(
            "{{\"keys\":[{{\"kty\":\"RSA\",\"kid\":\"bench-1\",\"alg\":\"RS256\",\"n\":\"{n_b64}\",\"e\":\"{e_b64}\"}}]}}"
        ))
        .unwrap();
        assert!(
            verify_jwt_with_jwks(
                &jwt,
                &jwks,
                ISS_GOOGLE,
                "kanari-test-client",
                Some(&nonce),
                1_700_000_000
            )
            .is_ok()
        );
        // Sanity: address derivation agrees (proves fixture coherence).
        let _addr = derive_zklogin_address_v2(ISS_GOOGLE, "kanari-test-client", "1234", &[9u8; 32])
            .unwrap();
        bench("zklogin_jwt_verify", 200, || {
            verify_jwt_with_jwks(
                black_box(&jwt),
                black_box(&jwks),
                black_box(ISS_GOOGLE),
                black_box("kanari-test-client"),
                Some(black_box(&nonce)),
                black_box(1_700_000_000),
            )
            .unwrap()
        });
    }

    // --- Groth16 binding-circuit verify (setup once, time verify only) ---
    {
        use ark_std::rand::SeedableRng;
        use kanari_crypto::signatures::groth16::{
            proof_to_bytes, verify_groth16_proof, vk_to_bytes,
        };
        use kanari_crypto::signatures::zklogin::{compute_nonce, derive_zklogin_address_v2};
        use kanari_crypto::signatures::zklogin_circuit::{
            prove_binding, public_inputs_to_be_bytes, setup_binding_circuit,
        };

        let iss = b"https://accounts.google.com";
        let aud = b"kanari-test-client";
        let salt = [9u8; 32];
        let eph_pub = [42u8; 32];
        let mut rng = ark_std::rand::rngs::StdRng::seed_from_u64(0xB10C5EED);
        let addr_hex = derive_zklogin_address_v2(
            "https://accounts.google.com",
            "kanari-test-client",
            "1234",
            &salt,
        )
        .unwrap();
        let address: [u8; 32] = hex::decode(&addr_hex[2..]).unwrap().try_into().unwrap();
        let nonce_hex = compute_nonce(&eph_pub, 1000, &[7u8; 32]);
        let nonce: [u8; 32] = hex::decode(&nonce_hex).unwrap().try_into().unwrap();
        let (pk, vk) = setup_binding_circuit(iss, aud, &mut rng).unwrap();
        let proof = prove_binding(
            &pk, iss, aud, salt, b"1234", [7u8; 32], eph_pub, 1000, address, nonce, &mut rng,
        )
        .unwrap();
        let vk_bytes = vk_to_bytes(&vk).unwrap();
        let inputs = public_inputs_to_be_bytes(&address, &nonce);
        let proof_bytes = proof_to_bytes(&proof).unwrap();
        assert!(verify_groth16_proof(&vk_bytes, &inputs, &proof_bytes).unwrap());
        bench("groth16_verify_binding", 20, || {
            verify_groth16_proof(
                black_box(&vk_bytes),
                black_box(&inputs),
                black_box(&proof_bytes),
            )
            .unwrap()
        });
    }
}
