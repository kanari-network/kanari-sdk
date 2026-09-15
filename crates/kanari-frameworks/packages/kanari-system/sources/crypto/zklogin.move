// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// zkLogin (Sui-style) without the ZK proof — Phase 3b.
///
/// Verifies: ephemeral Ed25519 signature + JWT RS256 against a JWK +
/// claim binding (iss/aud/exp/nonce). `sub` and `salt` are visible inputs,
/// so this does NOT hide the user identity from the chain (that needs the
/// Groth16 circuit, Phase 3c). Use it where Google-login UX matters more
/// than unlinkability.
module kanari_system::zklogin {
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

    const EPHEMERAL_PUBKEY_LENGTH: u64 = 32;
    const EPHEMERAL_SIG_LENGTH: u64 = 64;
    const SALT_LENGTH: u64 = 32;

    public fun ephemeral_pubkey_length(): u64 { EPHEMERAL_PUBKEY_LENGTH }
    public fun ephemeral_sig_length(): u64 { EPHEMERAL_SIG_LENGTH }
    public fun salt_length(): u64 { SALT_LENGTH }

    /// Verify JWT RS256 against `jwks_json` (`{"keys":[...]}`) and check
    /// `iss`/`aud`/expiry at `now_secs`. Returns true on success.
    /// Aborts with the codes above on malformed input.
    public fun verify(
        jwt: &vector<u8>,
        jwks_json: &vector<u8>,
        iss: &vector<u8>,
        aud: &vector<u8>,
        now_secs: u64,
    ): bool {
        assert!(vector::length(jwt) > 0, E_INVALID_JWT);
        native_verify(jwt, jwks_json, iss, aud, now_secs)
    }

    native fun native_verify(
        jwt: &vector<u8>,
        jwks_json: &vector<u8>,
        iss: &vector<u8>,
        aud: &vector<u8>,
        now_secs: u64,
    ): bool;

    /// Ed25519 check for the ephemeral session key (non-aborting).
    public fun verify_ephemeral(
        ephemeral_pubkey: &vector<u8>,
        msg: &vector<u8>,
        sig: &vector<u8>,
    ): bool {
        assert!(vector::length(ephemeral_pubkey) == EPHEMERAL_PUBKEY_LENGTH, E_INVALID_EPHEMERAL_SIG);
        assert!(vector::length(sig) == EPHEMERAL_SIG_LENGTH, E_INVALID_EPHEMERAL_SIG);
        native_verify_ephemeral(ephemeral_pubkey, msg, sig)
    }

    native fun native_verify_ephemeral(
        ephemeral_pubkey: &vector<u8>,
        msg: &vector<u8>,
        sig: &vector<u8>,
    ): bool;

    /// Derive the 32-byte zkLogin address from claims + salt.
    public fun derive_address(
        iss: &vector<u8>,
        aud: &vector<u8>,
        sub: &vector<u8>,
        salt: &vector<u8>,
    ): vector<u8> {
        assert!(vector::length(salt) == SALT_LENGTH, E_CLAIM_MISMATCH);
        native_derive_address(iss, aud, sub, salt)
    }

    native fun native_derive_address(
        iss: &vector<u8>,
        aud: &vector<u8>,
        sub: &vector<u8>,
        salt: &vector<u8>,
    ): vector<u8>;

    /// True iff the JWT `nonce` claim equals the ephemeral binding.
    public fun check_nonce(
        jwt: &vector<u8>,
        ephemeral_pubkey: &vector<u8>,
        max_epoch: u64,
        randomness: &vector<u8>,
    ): bool {
        assert!(vector::length(ephemeral_pubkey) == EPHEMERAL_PUBKEY_LENGTH, E_INVALID_EPHEMERAL_SIG);
        native_check_nonce(jwt, ephemeral_pubkey, max_epoch, randomness)
    }

    native fun native_check_nonce(
        jwt: &vector<u8>,
        ephemeral_pubkey: &vector<u8>,
        max_epoch: u64,
        randomness: &vector<u8>,
    ): bool;

    /// Full session check: JWT valid AND nonce bound AND ephemeral sig valid.
    /// One call for login gates.
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
    ): bool {
        verify(jwt, jwks_json, iss, aud, now_secs)
            && check_nonce(jwt, ephemeral_pubkey, max_epoch, randomness)
            && verify_ephemeral(ephemeral_pubkey, msg, ephemeral_sig)
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
    fun test_constants() {
        assert!(ephemeral_pubkey_length() == 32, 0);
        assert!(ephemeral_sig_length() == 64, 1);
        assert!(salt_length() == 32, 2);
    }
}
