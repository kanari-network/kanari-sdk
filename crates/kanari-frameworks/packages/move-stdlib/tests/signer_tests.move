// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[test_only]
module std::signer_tests {
    use std::signer;
    use std::vector;

    #[test]
    fun test_address_to_bytes_big_endian() {
        // @0x1 => 31 zero bytes followed by 0x01.
        let b = signer::address_to_bytes(@0x1);
        assert!(vector::length(&b) == 32, 0);
        assert!(*vector::borrow(&b, 0) == 0, 1);
        assert!(*vector::borrow(&b, 30) == 0, 2);
        assert!(*vector::borrow(&b, 31) == 1, 3);

        // Full-width value round-trips byte by byte.
        let addr = @0x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20;
        let full = signer::address_to_bytes(addr);
        assert!(vector::length(&full) == 32, 4);
        assert!(*vector::borrow(&full, 0) == 1, 5);
        assert!(*vector::borrow(&full, 1) == 2, 6);
        assert!(*vector::borrow(&full, 31) == 32, 7);

        // Distinct addresses give distinct encodings (not a zero stub).
        assert!(signer::address_to_bytes(@0x1234) != signer::address_to_bytes(@0x1235), 8);
    }

    #[test]
    fun test_address_to_u64_low_bytes() {
        // Low 8 bytes of @0x0102030405060708, big-endian.
        assert!(signer::address_to_u64(@0x0102030405060708) == 72623859790382856, 0);
        // High bytes are ignored.
        assert!(
            signer::address_to_u64(@0xff000000000000000000000000000000000000000000000102030405060708)
                == 72623859790382856,
            1,
        );
        assert!(signer::address_to_u64(@0x0) == 0, 2);
        assert!(signer::address_to_u64(@0x1) == 1, 3);
        // Deterministic.
        assert!(
            signer::address_to_u64(@0xabcdef) == signer::address_to_u64(@0xabcdef),
            4,
        );
    }
}
