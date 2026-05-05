// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

module kanari_system::rs256 {
    use std::vector;

    /// Minimum modulus (n) length (bits) for RSASSA-PKCS1-V1_5 with SHA-256 (RS256)
    const RSASSA_PKCS1_V1_5_MINIMUM_MODULUS_LENGTH: u64 = 2048;
    /// Minimum exponent (e) length (bytes) for RSASSA-PKCS1-V1_5 with SHA-256 (RS256)
    const RSASSA_PKCS1_V1_5_MINIMUM_EXPONENT_LENGTH: u64 = 1;
    /// Maximum exponent (e) length (bytes) for RSASSA-PKCS1-V1_5 with SHA-256 (RS256)
    const RSASSA_PKCS1_V1_5_MAXIMUM_EXPONENT_LENGTH: u64 = 512;
    /// Message length for the Sha2-256 hash function
    const SHA256_MESSAGE_LENGTH: u64 = 32;

    // Error codes
    const ErrorInvalidSignature: u64 = 1;
    const ErrorInvalidPubKey: u64 = 2;
    const ErrorInvalidHashType: u64 = 3;
    const ErrorInvalidMessageLength: u64 = 4;

    // Hash type
    const SHA256: u8 = 0;

    // functions to the constants
    public fun sha256(): u8 {
        SHA256
    }

    /// Verifies a RSA signature from public modulus (n) and public exponent (e) over RSASSA-PKCS1-V1_5 with SHA-256 (RS256).
    /// The message will be the original message with hashing in-function.
    public fun verify(
        signature: &vector<u8>,
        n: &vector<u8>,
        e: &vector<u8>,
        msg: &vector<u8>
    ): bool {
        // check conditions for verify function
        check_conditions_verify(signature, n, e);
        // call native_verify
        native_verify(signature, n, e, msg)
    }

    native fun native_verify(
        signature: &vector<u8>,
        n: &vector<u8>,
        e: &vector<u8>,
        msg: &vector<u8>,
    ): bool;

    /// Verifies a RSA signature from public modulus (n) and public exponent (e) over RSASSA-PKCS1-V1_5 with SHA-256 (RS256).
    /// The message will be the hashed using SHA256 before the verification.
    public fun verify_prehash(
        signature: &vector<u8>,
        n: &vector<u8>,
        e: &vector<u8>,
        msg: &vector<u8>,
        hash_type: u8
    ): bool {
        // check conditions for verify prehash function
        check_conditions_verify_prehash(signature, n, e, msg, hash_type);
        // call native_verify
        native_verify_prehash(signature, n, e, msg, hash_type)
    }

    native fun native_verify_prehash(
        signature: &vector<u8>,
        n: &vector<u8>,
        e: &vector<u8>,
        msg: &vector<u8>,
        hash_type: u8
    ): bool;

    fun check_conditions_verify(signature: &vector<u8>, n: &vector<u8>, e: &vector<u8>) {
        // 1. Signature length must match modulus (n) length (both in bytes)
        // This ensures the signature was generated with the same key size
        assert!(vector::length(signature) == vector::length(n), ErrorInvalidSignature);
        
        // 2. Ensure modulus (n) length meets minimum security requirements (2048 bits = 256 bytes)
        // RSA keys smaller than 2048 bits are considered insecure
        assert!(vector::length(n) >= RSASSA_PKCS1_V1_5_MINIMUM_MODULUS_LENGTH / 8, ErrorInvalidPubKey);
        
        // 3. Ensure modulus (n) length does not exceed maximum (4096 bits = 512 bytes)
        // Larger keys provide diminishing returns and impact performance
        assert!(vector::length(n) <= (RSASSA_PKCS1_V1_5_MINIMUM_MODULUS_LENGTH / 8) * 2, ErrorInvalidPubKey);
        
        // 4. Ensure exponent (e) length meets minimum requirements (at least 1 byte)
        assert!(vector::length(e) >= RSASSA_PKCS1_V1_5_MINIMUM_EXPONENT_LENGTH, ErrorInvalidPubKey);
        
        // 5. Ensure exponent (e) length does not exceed maximum (512 bytes)
        assert!(vector::length(e) <= RSASSA_PKCS1_V1_5_MAXIMUM_EXPONENT_LENGTH, ErrorInvalidPubKey);
        
        // 6. Validate exponent (e) value for security
        let e_len = vector::length(e);
        
        // For single-byte exponents, ensure value is >= 3
        // Values 1 and 2 are cryptographically weak for RSA
        if (e_len == 1) {
            let first_byte = *vector::borrow(e, 0);
            assert!(first_byte >= 3, ErrorInvalidPubKey);
        };
        
        // For multi-byte exponents, perform additional validation
        if (e_len > 1) {
            // a) Verify exponent is not all zeros (would be invalid)
            let has_non_zero = check_exponent_not_all_zeros(e, e_len, 0);
            assert!(has_non_zero, ErrorInvalidPubKey);
            
            // b) Check for excessive leading zeros (>75% of length is suspicious)
            // This catches malformed or intentionally obfuscated keys
            let leading_zeros = count_leading_zeros(e, e_len, 0);
            assert!(leading_zeros < (e_len * 3 / 4), ErrorInvalidPubKey);
            
            // c) Limit actual exponent size to prevent abnormally large values
            // After removing leading zeros, exponent should be ≤ 8 bytes (64 bits)
            // This allows common values like 65537 (0x010001) while rejecting extreme cases
            let actual_bytes = e_len - leading_zeros;
            assert!(actual_bytes <= 8, ErrorInvalidPubKey);
        };
    }

    /// Helper function to check that exponent is not all zeros
    /// Returns true if at least one non-zero byte is found
    fun check_exponent_not_all_zeros(e: &vector<u8>, len: u64, _index: u64): bool {
        let index = 0;
        while (index < len) {
            if (*vector::borrow(e, index) != 0) {
                return true
            };
            index = index + 1;
        };
        false
    }

    /// Count leading zero bytes in exponent
    /// Returns the number of consecutive zero bytes from the start
    fun count_leading_zeros(e: &vector<u8>, len: u64, _index: u64): u64 {
        let index = 0;
        while (index < len) {
            if (*vector::borrow(e, index) != 0) {
                return index
            };
            index = index + 1;
        };
        len
    }

    fun check_conditions_verify_prehash(signature: &vector<u8>, n: &vector<u8>, e: &vector<u8>, msg: &vector<u8>, hash_type: u8) {
        // include all verify conditions
        check_conditions_verify(signature, n, e);
        assert!(hash_type == SHA256, ErrorInvalidHashType);
        assert!(vector::length(msg) == SHA256_MESSAGE_LENGTH, ErrorInvalidMessageLength);
    }
}