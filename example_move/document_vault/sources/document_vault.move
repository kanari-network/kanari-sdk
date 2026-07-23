module document_vault::document_vault {
    use std::bcs;
    use std::hash;
    use std::signer;
    use std::string::String;
    use std::vector;
    use kanari_system::ed25519;
    use kanari_system::event;
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    const E_NOT_OWNER: u64 = 1;
    const E_BAD_HASH_LENGTH: u64 = 2;
    const E_BAD_SIZE: u64 = 3;
    const E_ALREADY_GRANTED: u64 = 4;
    const E_GRANT_NOT_FOUND: u64 = 5;
    const E_BAD_PUBLIC_KEY_LENGTH: u64 = 6;
    const E_BAD_SIGNATURE_LENGTH: u64 = 7;

    const HASH_LEN_BLAKE3_256: u64 = 32;

    /// On-chain proof/metadata for one off-chain document.
    ///
    /// Keep the file bytes in object storage, IPFS, Arweave, local enterprise
    /// storage, or another storage provider. Keep only immutable verification
    /// data and access metadata here.
    struct Document has key, store {
        id: UID,
        owner: address,
        title: String,
        uri: String,
        content_hash: vector<u8>,
        mime_type: String,
        size_bytes: u64,
        version: u64,
        public_read: bool,
        created_at_ms: u64,
        updated_at_ms: u64,
        grants: vector<AccessGrant>,
    }

    struct AccessGrant has copy, drop, store {
        reader: address,
        granted_at_ms: u64,
    }

    struct DocumentCreated has copy, drop, store {
        document_id: address,
        owner: address,
        content_hash: vector<u8>,
        version: u64,
        public_read: bool,
        timestamp_ms: u64,
    }

    struct DocumentUpdated has copy, drop, store {
        document_id: address,
        owner: address,
        old_version: u64,
        new_version: u64,
        content_hash: vector<u8>,
        timestamp_ms: u64,
    }

    struct DocumentAccessChanged has copy, drop, store {
        document_id: address,
        owner: address,
        reader: address,
        granted: bool,
        timestamp_ms: u64,
    }

    public entry fun create_document(
        title: String,
        uri: String,
        content_hash: vector<u8>,
        mime_type: String,
        size_bytes: u64,
        public_read: bool,
        ctx: &mut TxContext,
    ) {
        assert_valid_document_input(&content_hash, size_bytes);

        let owner = tx_context::sender(ctx);
        let now = tx_context::epoch_timestamp_ms(ctx);
        let id = object::new(ctx);
        let document_id = object::uid_address(&id);

        let doc = Document {
            id,
            owner,
            title,
            uri,
            content_hash: copy content_hash,
            mime_type,
            size_bytes,
            version: 1,
            public_read,
            created_at_ms: now,
            updated_at_ms: now,
            grants: vector::empty<AccessGrant>(),
        };

        transfer::public_transfer(doc, owner);

        event::emit(DocumentCreated {
            document_id,
            owner,
            content_hash,
            version: 1,
            public_read,
            timestamp_ms: now,
        });
    }

    /// API-first update path. The runtime authenticates the mutable object input.
    public entry fun update_document_ref(
        doc: &mut Document,
        title: String,
        uri: String,
        content_hash: vector<u8>,
        mime_type: String,
        size_bytes: u64,
        public_read: bool,
        ctx: &mut TxContext,
    ) {
        assert_owner(doc, ctx);
        assert_valid_document_input(&content_hash, size_bytes);

        let now = tx_context::epoch_timestamp_ms(ctx);
        let old_version = doc.version;
        let new_version = old_version + 1;
        let document_id = object::uid_address(&doc.id);

        doc.title = title;
        doc.uri = uri;
        doc.content_hash = copy content_hash;
        doc.mime_type = mime_type;
        doc.size_bytes = size_bytes;
        doc.version = new_version;
        doc.public_read = public_read;
        doc.updated_at_ms = now;

        object::save_object(doc);

        event::emit(DocumentUpdated {
            document_id,
            owner: doc.owner,
            old_version,
            new_version,
            content_hash,
            timestamp_ms: now,
        });
    }

    /// CLI-compatible update path for clients that pass an object id.
    public entry fun update_document(
        document_id: address,
        title: String,
        uri: String,
        content_hash: vector<u8>,
        mime_type: String,
        size_bytes: u64,
        public_read: bool,
        ctx: &mut TxContext,
    ) {
        let doc: &mut Document = object::borrow_global_mut<Document>(document_id);
        update_document_ref(doc, title, uri, content_hash, mime_type, size_bytes, public_read, ctx);
    }

    public entry fun grant_access_ref(
        doc: &mut Document,
        reader: address,
        ctx: &mut TxContext,
    ) {
        assert_owner(doc, ctx);
        assert!(!has_grant(doc, reader), E_ALREADY_GRANTED);

        let now = tx_context::epoch_timestamp_ms(ctx);
        vector::push_back(&mut doc.grants, AccessGrant { reader, granted_at_ms: now });
        object::save_object(doc);

        event::emit(DocumentAccessChanged {
            document_id: object::uid_address(&doc.id),
            owner: doc.owner,
            reader,
            granted: true,
            timestamp_ms: now,
        });
    }

    public entry fun grant_access(
        document_id: address,
        reader: address,
        ctx: &mut TxContext,
    ) {
        let doc: &mut Document = object::borrow_global_mut<Document>(document_id);
        grant_access_ref(doc, reader, ctx);
    }

    public entry fun revoke_access_ref(
        doc: &mut Document,
        reader: address,
        ctx: &mut TxContext,
    ) {
        assert_owner(doc, ctx);

        let i = find_grant_index(doc, reader);
        assert!(i < vector::length(&doc.grants), E_GRANT_NOT_FOUND);

        vector::swap_remove(&mut doc.grants, i);
        let now = tx_context::epoch_timestamp_ms(ctx);
        object::save_object(doc);

        event::emit(DocumentAccessChanged {
            document_id: object::uid_address(&doc.id),
            owner: doc.owner,
            reader,
            granted: false,
            timestamp_ms: now,
        });
    }

    public entry fun revoke_access(
        document_id: address,
        reader: address,
        ctx: &mut TxContext,
    ) {
        let doc: &mut Document = object::borrow_global_mut<Document>(document_id);
        revoke_access_ref(doc, reader, ctx);
    }

    public fun can_read(doc: &Document, reader: address): bool {
        doc.public_read || doc.owner == reader || has_grant(doc, reader)
    }

    public fun verify_hash(doc: &Document, content_hash: vector<u8>): bool {
        doc.content_hash == content_hash
    }

    /// Hash arbitrary document bytes with Kanari's canonical BLAKE3-256 native.
    ///
    /// Prefer calculating this off-chain for large files. This helper is useful
    /// for small payload tests or clients that need identical on-chain semantics.
    public fun blake3_256_document_bytes(bytes: vector<u8>): vector<u8> {
        hash::blake3_256(&bytes)
    }

    /// Compatibility helper for systems that still import SHA-256 document hashes.
    /// New Kanari-native document proofs should prefer BLAKE3-256.
    public fun sha256_document_bytes(bytes: vector<u8>): vector<u8> {
        hash::sha2_256(bytes)
    }

    /// Canonical message signed by an external document authority.
    ///
    /// Message format:
    /// `b"KANARI_DOCUMENT_VAULT_V1" || document_id || version || content_hash`
    public fun signing_message(
        document_id: address,
        version: u64,
        content_hash: vector<u8>,
    ): vector<u8> {
        assert!(vector::length(&content_hash) == HASH_LEN_BLAKE3_256, E_BAD_HASH_LENGTH);

        let msg = b"KANARI_DOCUMENT_VAULT_V1";
        vector::append(&mut msg, signer::address_to_bytes(document_id));
        vector::append(&mut msg, bcs::to_bytes(&version));
        vector::append(&mut msg, content_hash);
        msg
    }

    /// Verify an Ed25519 signature over the current document proof.
    ///
    /// This uses `kanari_system::ed25519` from the framework crypto package.
    /// It does not prove who controls `doc.owner`; it proves that the supplied
    /// public key signed the canonical document proof message.
    public fun verify_ed25519_document_signature(
        doc: &Document,
        public_key: vector<u8>,
        signature: vector<u8>,
    ): bool {
        assert!(vector::length(&public_key) == ed25519::public_key_length(), E_BAD_PUBLIC_KEY_LENGTH);
        assert!(vector::length(&signature) == ed25519::signature_length(), E_BAD_SIGNATURE_LENGTH);

        let msg = signing_message(
            object::uid_address(&doc.id),
            doc.version,
            *&doc.content_hash,
        );
        ed25519::verify(&signature, &public_key, &msg)
    }

    public fun id(doc: &Document): address {
        object::uid_address(&doc.id)
    }

    public fun owner(doc: &Document): address {
        doc.owner
    }

    public fun version(doc: &Document): u64 {
        doc.version
    }

    public fun size_bytes(doc: &Document): u64 {
        doc.size_bytes
    }

    public fun public_read(doc: &Document): bool {
        doc.public_read
    }

    public fun grant_count(doc: &Document): u64 {
        vector::length(&doc.grants)
    }

    fun assert_owner(doc: &Document, ctx: &TxContext) {
        assert!(doc.owner == tx_context::sender(ctx), E_NOT_OWNER);
    }

    fun assert_valid_document_input(content_hash: &vector<u8>, size_bytes: u64) {
        assert!(vector::length(content_hash) == HASH_LEN_BLAKE3_256, E_BAD_HASH_LENGTH);
        assert!(size_bytes > 0, E_BAD_SIZE);
    }

    fun has_grant(doc: &Document, reader: address): bool {
        find_grant_index(doc, reader) < vector::length(&doc.grants)
    }

    fun find_grant_index(doc: &Document, reader: address): u64 {
        let i = 0;
        let len = vector::length(&doc.grants);
        while (i < len) {
            let grant = vector::borrow(&doc.grants, i);
            if (grant.reader == reader) {
                return i
            };
            i = i + 1;
        };
        len
    }

    #[test_only]
    public fun create_for_test(
        title: String,
        uri: String,
        content_hash: vector<u8>,
        mime_type: String,
        size_bytes: u64,
        public_read: bool,
        ctx: &mut TxContext,
    ): Document {
        assert_valid_document_input(&content_hash, size_bytes);

        let now = tx_context::epoch_timestamp_ms(ctx);
        Document {
            id: object::new(ctx),
            owner: tx_context::sender(ctx),
            title,
            uri,
            content_hash,
            mime_type,
            size_bytes,
            version: 1,
            public_read,
            created_at_ms: now,
            updated_at_ms: now,
            grants: vector::empty<AccessGrant>(),
        }
    }

    #[test_only]
    public fun destroy_for_test(doc: Document) {
        let Document {
            id,
            owner: _,
            title: _,
            uri: _,
            content_hash: _,
            mime_type: _,
            size_bytes: _,
            version: _,
            public_read: _,
            created_at_ms: _,
            updated_at_ms: _,
            grants: _,
        } = doc;
        object::delete(id);
    }
}
