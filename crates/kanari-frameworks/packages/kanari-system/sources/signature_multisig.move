// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Threshold multisig over raw signature verification.
///
/// Unlike `multisig` (which custodies `Coin<T>` and executes proposals),
/// this module verifies that at least `threshold` of `n` DISTINCT public
/// keys signed the same message — for ed25519, secp256k1 ECDSA, and P-256
/// ECDSA.
///
/// Callers bind the result to their own authorization logic (e.g. minting,
/// admin actions, cross-chain messages). No state, no funds custody.
///
/// # Security contract (callers MUST obey)
/// - Pass a FIXED committee: the `public_keys` vector must come from
///   on-chain state the caller controls, never from transaction input an
///   attacker can choose. This module rejects duplicate keys, but it cannot
///   tell whether the committee itself is the right one.
/// - Replay protection is the caller's job: build `msg` with
///   [`build_message`] (domain separator + unique nonce) so a signature
///   for one action can never be replayed as another action.
module kanari_system::signature_multisig {
    use kanari_system::ecdsa_k1;
    use kanari_system::ecdsa_r1;
    use kanari_system::ed25519;
    use std::bcs;
    use std::vector;

    /// Not enough valid signatures to meet the threshold.
    const E_THRESHOLD_NOT_MET: u64 = 1;
    /// Threshold is zero, or exceeds the number of keys.
    const E_INVALID_THRESHOLD: u64 = 2;
    /// Keys and signatures vectors have different lengths.
    const E_LENGTH_MISMATCH: u64 = 3;
    /// Unsupported scheme selector.
    const E_INVALID_SCHEME: u64 = 4;
    /// The same public key appears twice. Without this check one signer
    /// could satisfy an n-of-n threshold alone by replaying their own
    /// key/signature pair.
    const E_DUPLICATE_KEY: u64 = 5;

    /// Signature scheme selectors.
    const SCHEME_ED25519: u8 = 0;
    const SCHEME_ECDSA_K1: u8 = 1;
    const SCHEME_ECDSA_R1: u8 = 2;

    public fun scheme_ed25519(): u8 {
        SCHEME_ED25519
    }

    public fun scheme_ecdsa_k1(): u8 {
        SCHEME_ECDSA_K1
    }

    public fun scheme_ecdsa_r1(): u8 {
        SCHEME_ECDSA_R1
    }

    /// Count how many `(public_key, signature)` pairs verify for `msg`.
    /// Entries that fail to verify count as zero — the call only aborts on
    /// malformed input (length mismatch, bad threshold, bad scheme).
    public fun count_valid_ed25519(
        public_keys: &vector<vector<u8>>,
        signatures: &vector<vector<u8>>,
        msg: &vector<u8>
    ): u64 {
        let n = vector::length(public_keys);
        assert!(n == vector::length(signatures), E_LENGTH_MISMATCH);
        assert_no_duplicate_keys(public_keys);
        let (count, i) = (0u64, 0u64);
        while (i < n) {
            if (ed25519::verify(
                vector::borrow(signatures, i),
                vector::borrow(public_keys, i),
                msg
            )) {
                count = count + 1;
            };
            i = i + 1;
        };
        count
    }

    /// Count valid secp256k1 ECDSA signatures (`hash`: 0 = Keccak256, 1 = Sha256).
    public fun count_valid_ecdsa_k1(
        public_keys: &vector<vector<u8>>,
        signatures: &vector<vector<u8>>,
        msg: &vector<u8>,
        hash: u8
    ): u64 {
        let n = vector::length(public_keys);
        assert!(n == vector::length(signatures), E_LENGTH_MISMATCH);
        assert_no_duplicate_keys(public_keys);
        let (count, i) = (0u64, 0u64);
        while (i < n) {
            if (ecdsa_k1::verify(
                vector::borrow(signatures, i),
                vector::borrow(public_keys, i),
                msg,
                hash
            )) {
                count = count + 1;
            };
            i = i + 1;
        };
        count
    }

    /// Count valid P-256 ECDSA signatures (SHA-256, 64-byte raw `(r, s)`).
    public fun count_valid_ecdsa_r1(
        public_keys: &vector<vector<u8>>,
        signatures: &vector<vector<u8>>,
        msg: &vector<u8>
    ): u64 {
        let n = vector::length(public_keys);
        assert!(n == vector::length(signatures), E_LENGTH_MISMATCH);
        assert_no_duplicate_keys(public_keys);
        let (count, i) = (0u64, 0u64);
        while (i < n) {
            if (ecdsa_r1::verify(
                vector::borrow(signatures, i),
                vector::borrow(public_keys, i),
                msg
            )) {
                count = count + 1;
            };
            i = i + 1;
        };
        count
    }

    /// Verify `threshold`-of-`n` ed25519 signatures. Aborts
    /// `E_THRESHOLD_NOT_MET` when fewer than `threshold` verify.
    public fun verify_threshold_ed25519(
        public_keys: &vector<vector<u8>>,
        signatures: &vector<vector<u8>>,
        msg: &vector<u8>,
        threshold: u64
    ) {
        assert_valid_threshold(threshold, vector::length(public_keys));
        assert!(
            count_valid_ed25519(public_keys, signatures, msg) >= threshold,
            E_THRESHOLD_NOT_MET
        );
    }

    /// Verify `threshold`-of-`n` secp256k1 ECDSA signatures.
    public fun verify_threshold_ecdsa_k1(
        public_keys: &vector<vector<u8>>,
        signatures: &vector<vector<u8>>,
        msg: &vector<u8>,
        hash: u8,
        threshold: u64
    ) {
        assert_valid_threshold(threshold, vector::length(public_keys));
        assert!(
            count_valid_ecdsa_k1(public_keys, signatures, msg, hash) >= threshold,
            E_THRESHOLD_NOT_MET
        );
    }

    /// Verify `threshold`-of-`n` P-256 ECDSA signatures.
    public fun verify_threshold_ecdsa_r1(
        public_keys: &vector<vector<u8>>,
        signatures: &vector<vector<u8>>,
        msg: &vector<u8>,
        threshold: u64
    ) {
        assert_valid_threshold(threshold, vector::length(public_keys));
        assert!(
            count_valid_ecdsa_r1(public_keys, signatures, msg) >= threshold,
            E_THRESHOLD_NOT_MET
        );
    }

    /// Mixed-scheme threshold: each entry picks its scheme via `schemes[i]`
    /// (`0` = ed25519, `1` = ecdsa_k1 with `hash`, `2` = ecdsa_r1).
    /// Aborts `E_INVALID_SCHEME` on unknown selectors.
    public fun verify_threshold_mixed(
        schemes: &vector<u8>,
        public_keys: &vector<vector<u8>>,
        signatures: &vector<vector<u8>>,
        msg: &vector<u8>,
        hash: u8,
        threshold: u64
    ) {
        let n = vector::length(public_keys);
        assert!(n == vector::length(signatures), E_LENGTH_MISMATCH);
        assert!(n == vector::length(schemes), E_LENGTH_MISMATCH);
        assert_no_duplicate_keys(public_keys);
        assert_valid_threshold(threshold, n);
        let (count, i) = (0u64, 0u64);
        while (i < n) {
            let scheme = *vector::borrow(schemes, i);
            let ok =
                if (scheme == SCHEME_ED25519) {
                    ed25519::verify(
                        vector::borrow(signatures, i),
                        vector::borrow(public_keys, i),
                        msg
                    )
                } else if (scheme == SCHEME_ECDSA_K1) {
                    ecdsa_k1::verify(
                        vector::borrow(signatures, i),
                        vector::borrow(public_keys, i),
                        msg,
                        hash
                    )
                } else if (scheme == SCHEME_ECDSA_R1) {
                    ecdsa_r1::verify(
                        vector::borrow(signatures, i),
                        vector::borrow(public_keys, i),
                        msg
                    )
                } else {
                    abort E_INVALID_SCHEME
                };
            if (ok) {
                count = count + 1;
            };
            i = i + 1;
        };
        assert!(count >= threshold, E_THRESHOLD_NOT_MET);
    }

    fun assert_valid_threshold(threshold: u64, n: u64) {
        assert!(
            threshold > 0 && threshold <= n, E_INVALID_THRESHOLD
        );
    }

    /// Abort `E_DUPLICATE_KEY` if any public key appears twice.
    /// O(n^2) byte comparison — committees are small (typically <= 32),
    /// so a hash set is not worth the extra dependency.
    fun assert_no_duplicate_keys(public_keys: &vector<vector<u8>>) {
        let n = vector::length(public_keys);
        let i = 0u64;
        while (i < n) {
            let j = i + 1;
            while (j < n) {
                assert!(
                    vector::borrow(public_keys, i) != vector::borrow(public_keys, j),
                    E_DUPLICATE_KEY
                );
                j = j + 1;
            };
            i = i + 1;
        };
    }

    /// Build the message signers must sign: domain separator + length-prefixed
    /// action id + unique nonce. Signers and verifiers MUST use this (or an
    /// equivalent construction) so a signature cannot be replayed across
    /// actions, modules, or chains.
    ///
    /// Layout: `domain || u64 LE(action_id_len) || action_id || u64 LE(nonce)`.
    public fun build_message(
        domain: &vector<u8>, action_id: &vector<u8>, nonce: u64
    ): vector<u8> {
        let msg = *domain;
        vector::append(&mut msg, bcs::to_bytes(&vector::length(action_id)));
        vector::append(&mut msg, *action_id);
        vector::append(&mut msg, bcs::to_bytes(&nonce));
        msg
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_THRESHOLD)]
    fun test_threshold_zero_rejected() {
        let keys = vector[x"00"];
        let sigs = vector[x"00"];
        verify_threshold_ed25519(&keys, &sigs, &b"m", 0);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_THRESHOLD)]
    fun test_threshold_over_n_rejected() {
        let keys = vector[x"00"];
        let sigs = vector[x"00"];
        verify_threshold_ed25519(&keys, &sigs, &b"m", 2);
    }

    #[test]
    #[expected_failure(abort_code = E_LENGTH_MISMATCH)]
    fun test_length_mismatch_rejected() {
        let keys = vector[x"00", x"01"];
        let sigs = vector[x"00"];
        verify_threshold_ed25519(&keys, &sigs, &b"m", 1);
    }

    #[test]
    #[expected_failure(abort_code = E_THRESHOLD_NOT_MET)]
    fun test_unmet_threshold_rejected() {
        // Garbage key/sig: ed25519::verify returns false, so 0 < 1 aborts.
        let keys = vector[x"00"];
        let sigs = vector[x"00"];
        verify_threshold_ed25519(&keys, &sigs, &b"m", 1);
    }

    #[test]
    #[expected_failure(abort_code = E_DUPLICATE_KEY)]
    fun test_duplicate_keys_rejected() {
        // One signer replaying their own key/sig pair must never satisfy 2-of-2.
        let keys = vector[x"00", x"00"];
        let sigs = vector[x"00", x"00"];
        verify_threshold_ed25519(&keys, &sigs, &b"m", 2);
    }

    #[test]
    #[expected_failure(abort_code = E_DUPLICATE_KEY)]
    fun test_duplicate_keys_rejected_mixed() {
        let schemes = vector[SCHEME_ED25519, SCHEME_ED25519];
        let keys = vector[x"00", x"00"];
        let sigs = vector[x"00", x"00"];
        verify_threshold_mixed(&schemes, &keys, &sigs, &b"m", 1, 2);
    }

    #[test]
    fun test_build_message_differs_per_nonce_and_action() {
        let domain = b"kanari.bridge.v1";
        let m1 = build_message(&domain, &b"send", 1);
        let m2 = build_message(&domain, &b"send", 2);
        let m3 = build_message(&domain, &b"mint", 1);
        let m4 = build_message(&b"other.domain", &b"send", 1);
        assert!(m1 != m2, 0);
        assert!(m1 != m3, 1);
        assert!(m1 != m4, 2);
        // Layout is deterministic: rebuilding yields identical bytes.
        assert!(m1 == build_message(&domain, &b"send", 1), 3);
    }
}

