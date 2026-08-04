// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// FIPS 206 FN-DSA-1024 / Falcon-1024 verification.
module kanari_system::falcon1024 {
    const PUBLIC_KEY_LENGTH: u64 = 1793;
    const MAX_SIGNATURE_LENGTH: u64 = 2048;

    public fun public_key_length(): u64 { PUBLIC_KEY_LENGTH }
    public fun max_signature_length(): u64 { MAX_SIGNATURE_LENGTH }

    native public fun verify(signature: &vector<u8>, public_key: &vector<u8>, message: &vector<u8>): bool;

    #[test]
    fun test_sizes() {
        assert!(public_key_length() == 1793, 0);
        assert!(max_signature_length() == 2048, 1);
    }

    #[test]
    fun test_rejects_malformed_input() {
        assert!(!verify(&x"00", &x"00", &x"6b616e617269"), 0);
    }

    #[test]
    fun test_rejects_empty_input() {
        assert!(!verify(&x"", &x"", &x""), 0);
    }

    #[test]
    fun test_rejects_truncated_signature() {
        assert!(!verify(&x"000102", &x"00", &x"6b616e617269"), 0);
    }
}
