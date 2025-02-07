module kanari_framework::metadata {
        use std::vector;
        use std::error;
        use std::signer;
        use kanari_framework::clock::{Self, Clock};  // Add clock module

        /// Error codes
        const EINVALID_HASH: u64 = 1;
        const EINVALID_OWNER: u64 = 2;
    
        /// Represents file metadata stored on chain
        struct Metad ata has key, store {
            owner: address,
            content_hash: vector<u8>,
            created_at: u64,
        }
    
        /// Create new metadata
        public fun new(): Metadata {
            Metadata {
                owner: @0x0,
                content_hash: vector::empty(),
                created_at: 0,
            }
        }
    
        /// Set the owner of the metadata
        public fun set_owner(metadata: &mut Metadata, owner: address) {
            assert!(owner != @0x0, error::invalid_argument(EINVALID_OWNER));
            metadata.owner = owner;
        }
    
        /// Set the content hash
        public fun set_hash(metadata: &mut Metadata, hash: vector<u8>) {
            assert!(!vector::is_empty(&hash), error::invalid_argument(EINVALID_HASH));
            metadata.content_hash = hash;
        }
    
        /// Store metadata on chain
        public fun store(metadata: Metadata, clock: &Clock): Metadata {
            // Validate metadata
            assert!(metadata.owner != @0x0, error::invalid_argument(EINVALID_OWNER));
            assert!(!vector::is_empty(&metadata.content_hash), error::invalid_argument(EINVALID_HASH));
            
            // Set creation timestamp using Clock
            let metadata_mut = &mut metadata;
            metadata_mut.created_at = clock::timestamp_ms(clock);
            
            metadata
        }
    
        /// Get metadata owner
        public fun get_owner(metadata: &Metadata): address {
            metadata.owner
        }
    
        /// Get content hash
        public fun get_hash(metadata: &Metadata): vector<u8> {
            *&metadata.content_hash
        }
    
        /// Verify file hash matches metadata
        public fun verify_hash(metadata: &Metadata, hash: vector<u8>): bool {
            metadata.content_hash == hash
        }
    
        /// Check if account is the owner
        public fun is_owner(metadata: &Metadata, account: &signer): bool {
            metadata.owner == signer::address_of(account)
        }

}