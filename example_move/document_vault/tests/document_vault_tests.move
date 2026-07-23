#[test_only]
module document_vault::document_vault_tests {
    use std::string;
    use std::vector;
    use document_vault::document_vault;
    use kanari_system::tx_context;

    const OWNER: address = @0xA11CE;
    const READER: address = @0xB0B;
    const STRANGER: address = @0xC0FFEE;

    fun hash32(seed: u8): vector<u8> {
        let h = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut h, seed);
            i = i + 1;
        };
        h
    }

    #[test]
    fun create_private_document_owner_can_read() {
        let ctx = tx_context::new_from_hint(OWNER, 1, 0, 1_000, 0);
        let doc = document_vault::create_for_test(
            string::utf8(b"Passport"),
            string::utf8(b"ipfs://doc-cid"),
            hash32(7),
            string::utf8(b"application/pdf"),
            1024,
            false,
            &mut ctx,
        );

        assert!(document_vault::owner(&doc) == OWNER, 0);
        assert!(document_vault::version(&doc) == 1, 1);
        assert!(document_vault::can_read(&doc, OWNER), 2);
        assert!(!document_vault::can_read(&doc, STRANGER), 3);
        assert!(document_vault::verify_hash(&doc, hash32(7)), 4);
        assert!(!document_vault::verify_hash(&doc, hash32(8)), 5);
        document_vault::destroy_for_test(doc);
    }

    #[test]
    fun grant_and_revoke_access() {
        let ctx = tx_context::new_from_hint(OWNER, 2, 0, 1_000, 0);
        let doc = document_vault::create_for_test(
            string::utf8(b"Invoice"),
            string::utf8(b"s3://bucket/invoice.pdf"),
            hash32(9),
            string::utf8(b"application/pdf"),
            2048,
            false,
            &mut ctx,
        );

        document_vault::grant_access_ref(&mut doc, READER, &mut ctx);
        assert!(document_vault::grant_count(&doc) == 1, 0);
        assert!(document_vault::can_read(&doc, READER), 1);

        document_vault::revoke_access_ref(&mut doc, READER, &mut ctx);
        assert!(document_vault::grant_count(&doc) == 0, 2);
        assert!(!document_vault::can_read(&doc, READER), 3);
        document_vault::destroy_for_test(doc);
    }

    #[test]
    fun update_increments_version_and_hash() {
        let ctx = tx_context::new_from_hint(OWNER, 3, 0, 1_000, 0);
        let doc = document_vault::create_for_test(
            string::utf8(b"Contract"),
            string::utf8(b"ipfs://v1"),
            hash32(1),
            string::utf8(b"application/pdf"),
            4096,
            false,
            &mut ctx,
        );

        document_vault::update_document_ref(
            &mut doc,
            string::utf8(b"Contract v2"),
            string::utf8(b"ipfs://v2"),
            hash32(2),
            string::utf8(b"application/pdf"),
            8192,
            true,
            &mut ctx,
        );

        assert!(document_vault::version(&doc) == 2, 0);
        assert!(document_vault::verify_hash(&doc, hash32(2)), 1);
        assert!(document_vault::public_read(&doc), 2);
        assert!(document_vault::can_read(&doc, STRANGER), 3);
        document_vault::destroy_for_test(doc);
    }

    #[test]
    #[expected_failure(location = document_vault::document_vault, abort_code = 2)]
    fun reject_non_blake3_hash_length() {
        let ctx = tx_context::new_from_hint(OWNER, 4, 0, 1_000, 0);
        let doc = document_vault::create_for_test(
            string::utf8(b"Bad"),
            string::utf8(b"ipfs://bad"),
            vector::empty<u8>(),
            string::utf8(b"application/pdf"),
            1,
            false,
            &mut ctx,
        );
        document_vault::destroy_for_test(doc);
    }
}
