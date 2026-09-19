// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// zkLogin (Sui-style) — Phase 3b (linkable JWT path) + Phase 3c (private
/// Groth16 path).
///
/// - `verify` / `verify_session`: ephemeral Ed25519 signature + JWT RS256
///   against a JWK + claim binding (iss/aud/exp/nonce) + `max_epoch` vs the
///   chain epoch. `sub` and `salt` are visible inputs — simple Google-login
///   UX, but linkable.
/// - `verify_pinned_proof` / `verify_private_session`: BN254 Groth16 proof
///   against a PINNED ceremony VK hash + public inputs. Unpinned proof
///   acceptance does not exist at this layer: any other VK aborts with
///   `E_VK_MISMATCH`. When the circuit keeps `sub`/`salt` private, the
///   chain learns nothing linkable beyond the public inputs (e.g. the
///   derived address).
module kanari_system::zklogin {
    use kanari_system::tx_context::{Self, TxContext};
    use std::vector;

    #[allow(unused_const)]
    /// JWT malformed or failed to parse.
    const E_INVALID_JWT: u64 = 1;

    #[allow(unused_const)]
    /// JWK document malformed or `kid` unknown.
    const E_INVALID_JWK: u64 = 2;

    #[allow(unused_const)]
    /// Ephemeral signature invalid (length checks in Move, crypto in native).
    const E_INVALID_EPHEMERAL_SIG: u64 = 3;

    #[allow(unused_const)]
    /// iss/aud/sub mismatch or address derivation failed.
    const E_CLAIM_MISMATCH: u64 = 4;

    #[allow(unused_const)]
    /// JWT expired.
    const E_EXPIRED: u64 = 5;

    #[allow(unused_const)]
    /// Groth16 verifying key / proof malformed.
    const E_INVALID_PROOF: u64 = 6;
    /// Pinned verifying-key hash mismatch.
    const E_VK_MISMATCH: u64 = 7;

    const EPHEMERAL_PUBKEY_LENGTH: u64 = 32;
    const EPHEMERAL_SIG_LENGTH: u64 = 64;
    const SALT_LENGTH: u64 = 32;

    public fun ephemeral_pubkey_length(): u64 {
        EPHEMERAL_PUBKEY_LENGTH
    }

    public fun ephemeral_sig_length(): u64 {
        EPHEMERAL_SIG_LENGTH
    }

    public fun salt_length(): u64 {
        SALT_LENGTH
    }

    /// Verify JWT RS256 against `jwks_json` (`{"keys":[...]}`) and check
    /// `iss`/`aud`/expiry at `now_secs`. Returns true on success.
    /// Aborts with the codes above on malformed input.
    public fun verify(
        jwt: &vector<u8>,
        jwks_json: &vector<u8>,
        iss: &vector<u8>,
        aud: &vector<u8>,
        now_secs: u64
    ): bool {
        assert!(vector::length(jwt) > 0, E_INVALID_JWT);
        native_verify(jwt, jwks_json, iss, aud, now_secs)
    }

    native fun native_verify(
        jwt: &vector<u8>,
        jwks_json: &vector<u8>,
        iss: &vector<u8>,
        aud: &vector<u8>,
        now_secs: u64
    ): bool;

    /// Ed25519 check for the ephemeral session key (non-aborting).
    /// Argument order is (sig, pk, msg): the native pops `msg`, then `pk`,
    /// then `sig` off the call stack, so the first declared parameter binds
    /// to the signature. Verified by `test_verify_ephemeral_accepts_fixture`
    /// with a real vector (swapped order returns false).
    public fun verify_ephemeral(
        sig: &vector<u8>, ephemeral_pubkey: &vector<u8>, msg: &vector<u8>
    ): bool {
        assert!(
            vector::length(ephemeral_pubkey) == EPHEMERAL_PUBKEY_LENGTH,
            E_INVALID_EPHEMERAL_SIG
        );
        assert!(vector::length(sig) == EPHEMERAL_SIG_LENGTH, E_INVALID_EPHEMERAL_SIG);
        native_verify_ephemeral(sig, ephemeral_pubkey, msg)
    }

    native fun native_verify_ephemeral(
        sig: &vector<u8>, ephemeral_pubkey: &vector<u8>, msg: &vector<u8>
    ): bool;

    /// Derive the 32-byte zkLogin address from claims + salt.
    public fun derive_address(
        iss: &vector<u8>, aud: &vector<u8>, sub: &vector<u8>, salt: &vector<u8>
    ): vector<u8> {
        assert!(vector::length(salt) == SALT_LENGTH, E_CLAIM_MISMATCH);
        native_derive_address(iss, aud, sub, salt)
    }

    native fun native_derive_address(
        iss: &vector<u8>, aud: &vector<u8>, sub: &vector<u8>, salt: &vector<u8>
    ): vector<u8>;

    /// True iff the JWT `nonce` claim equals the ephemeral binding.
    public fun check_nonce(
        jwt: &vector<u8>,
        ephemeral_pubkey: &vector<u8>,
        max_epoch: u64,
        randomness: &vector<u8>
    ): bool {
        assert!(
            vector::length(ephemeral_pubkey) == EPHEMERAL_PUBKEY_LENGTH,
            E_INVALID_EPHEMERAL_SIG
        );
        native_check_nonce(jwt, ephemeral_pubkey, max_epoch, randomness)
    }

    native fun native_check_nonce(
        jwt: &vector<u8>,
        ephemeral_pubkey: &vector<u8>,
        max_epoch: u64,
        randomness: &vector<u8>
    ): bool;

    /// BN254 Groth16 proof check (Phase 3c, non-aborting).
    ///
    /// `vk_bytes`: compressed `VerifyingKey<Bn254>`; `public_inputs_bytes`:
    /// concatenated 32-byte big-endian field elements; `proof_bytes`:
    /// compressed `Proof<Bn254>`. No `sub`/`salt`/JWT crosses this boundary,
    /// so a circuit that keeps them private gives real unlinkability.
    /// Aborts `E_INVALID_PROOF` on empty/malformed key or proof.
    public fun verify_proof(
        vk_bytes: &vector<u8>, public_inputs_bytes: &vector<u8>, proof_bytes: &vector<u8>
    ): bool {
        assert!(vector::length(vk_bytes) > 0, E_INVALID_PROOF);
        assert!(vector::length(proof_bytes) > 0, E_INVALID_PROOF);
        native_verify_proof(vk_bytes, public_inputs_bytes, proof_bytes)
    }

    native fun native_verify_proof(
        vk_bytes: &vector<u8>, public_inputs_bytes: &vector<u8>, proof_bytes: &vector<u8>
    ): bool;

    /// Pinned-key proof check: aborts unless `sha2_256(vk_bytes)` equals the
    /// ceremony VK hash the contract trusts, then verifies the proof.
    /// Deployments hardcode their ceremony hash here (or pass it from a
    /// versioned config object); proofs against any other VK abort, so a
    /// locally-generated (toxic-waste) setup can never pass as canonical.
    public fun verify_pinned_proof(
        expected_vk_hash: vector<u8>,
        vk_bytes: vector<u8>,
        public_inputs_bytes: &vector<u8>,
        proof_bytes: &vector<u8>
    ): bool {
        assert!(std::vector::length(&expected_vk_hash) == 32, E_VK_MISMATCH);
        let ok = verify_proof(&vk_bytes, public_inputs_bytes, proof_bytes);
        assert!(std::hash::sha2_256(vk_bytes) == expected_vk_hash, E_VK_MISMATCH);
        ok
    }

    /// Private session check: Groth16 proof valid AND ephemeral sig valid.
    /// The JWT-private counterpart of `verify_session`.
    ///
    /// The VK hash MUST be pinned: proofs against any other VK (including
    /// toxic-waste demo setups) abort with `E_VK_MISMATCH` before any
    /// pairing work is trusted. There is no unpinned entry point on
    /// purpose — use `verify_pinned_proof` + `verify_ephemeral` directly
    /// only if you re-check the pin yourself.
    public fun verify_private_session(
        expected_vk_hash: vector<u8>,
        vk_bytes: vector<u8>,
        public_inputs_bytes: &vector<u8>,
        proof_bytes: &vector<u8>,
        ephemeral_pubkey: &vector<u8>,
        msg: &vector<u8>,
        ephemeral_sig: &vector<u8>
    ): bool {
        verify_pinned_proof(expected_vk_hash, vk_bytes, public_inputs_bytes, proof_bytes)
            && verify_ephemeral(ephemeral_sig, ephemeral_pubkey, msg)
    }

    /// Full session check: JWT valid AND session live AND nonce bound AND
    /// ephemeral sig valid. One call for login gates.
    ///
    /// `max_epoch` is enforced against the chain epoch: sessions from an
    /// older epoch abort with `E_EXPIRED`. This is the on-chain counterpart
    /// of the Rust `check_max_epoch` used at admission — and the ONLY
    /// timeliness bound proof-mode sessions have (they carry no JWT `exp`).
    public fun verify_session(
        jwt: &vector<u8>,
        jwks_json: &vector<u8>,
        iss: &vector<u8>,
        aud: &vector<u8>,
        now_secs: u64,
        ephemeral_pubkey: &vector<u8>,
        max_epoch: u64,
        randomness: &vector<u8>,
        msg: &vector<u8>,
        ephemeral_sig: &vector<u8>,
        ctx: &TxContext
    ): bool {
        assert!(max_epoch >= tx_context::epoch(ctx), E_EXPIRED);
        verify(jwt, jwks_json, iss, aud, now_secs)
            && check_nonce(jwt, ephemeral_pubkey, max_epoch, randomness)
            && verify_ephemeral(ephemeral_sig, ephemeral_pubkey, msg)
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_JWT)]
    fun test_rejects_empty_jwt() {
        // Empty JWT aborts before reaching the native.
        // (Positive vectors need an RSA key; covered in Rust tests.)
        verify(&x"", &b"{}", &b"iss", &b"aud", 0);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_EPHEMERAL_SIG)]
    fun test_rejects_bad_pubkey_length() {
        // 1-byte key aborts in the Move length guard.
        verify_ephemeral(&x"00", &b"m", &x"00");
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_PROOF)]
    fun test_rejects_empty_proof() {
        // Empty VK aborts in the Move length guard before the native.
        verify_proof(&x"", &x"", &x"00");
    }

    #[test]
    fun test_constants() {
        assert!(ephemeral_pubkey_length() == 32, 0);
        assert!(ephemeral_sig_length() == 64, 1);
        assert!(salt_length() == 32, 2);
    }
}

