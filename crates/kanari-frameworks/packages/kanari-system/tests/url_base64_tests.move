// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[test_only]
module kanari_system::url_base64_tests {
    use kanari_system::base64;
    use kanari_system::url;
    use std::ascii;

    #[test]
    fun test_url_lifecycle() {
        let u = url::new_unsafe(ascii::string(b"https://kanari.network"));
        assert!(url::inner_url(&u) == ascii::string(b"https://kanari.network"), 0);
        url::update(&mut u, ascii::string(b"https://x.example"));
        assert!(url::inner_url(&u) == ascii::string(b"https://x.example"), 1);

        let v = url::new_unsafe_from_bytes(b"https://y.example/icon.png");
        assert!(url::inner_url(&v) == ascii::string(b"https://y.example/icon.png"), 2);
    }

    #[test]
    fun test_base64_roundtrip() {
        // RFC 4648 vectors.
        let e0 = b"";
        assert!(base64::encode(&e0) == b"", 0);
        let e1 = b"f";
        assert!(base64::encode(&e1) == b"Zg==", 1);
        let e2 = b"fo";
        assert!(base64::encode(&e2) == b"Zm8=", 2);
        let e3 = b"foo";
        assert!(base64::encode(&e3) == b"Zm9v", 3);
        let d0 = b"Zm9v";
        assert!(base64::decode(&d0) == b"foo", 4);

        // Roundtrip on binary data (all byte values).
        let raw = x"00112233445566778899aabbccddeeff";
        let enc = base64::encode(&raw);
        assert!(base64::decode(&enc) == raw, 5);
        // Distinct inputs give distinct outputs (not a stub).
        let ia = b"a";
        let ib = b"b";
        assert!(base64::encode(&ia) != base64::encode(&ib), 6);
    }
}
