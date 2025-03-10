module kanari_framework::metadata {
    use std::vector;
    use std::signer;

    /// Metadata stores information about file ownership and content hash
    struct Metadata has key, store, drop {
        owner: address,
        content_hash: vector<u8>,
    }

    /// Create a new empty metadata instance
    public fun new(): Metadata {
        Metadata {
            owner: @0x0,
            content_hash: vector::empty<u8>(),
        }
    }

    /// Set the owner of the metadata
    public fun set_owner(metadata: &mut Metadata, owner: address) {
        metadata.owner = owner;
    }

    /// Get the owner of the metadata
    public fun get_owner(metadata: &Metadata): address {
        metadata.owner
    }

    /// Set the content hash of the metadata
    public fun set_hash(metadata: &mut Metadata, content_hash: vector<u8>) {
        metadata.content_hash = content_hash;
    }

    /// Get the content hash from the metadata
    public fun get_hash(metadata: &Metadata): vector<u8> {
        metadata.content_hash
    }

    /// Store the metadata and return it
    public fun store(metadata: Metadata): Metadata {
        // In a real implementation, this might involve storing the metadata on-chain
        // For now, we just return it
        metadata
    }

    /// Verify if the provided hash matches the one stored in metadata
    public fun verify_hash(metadata: &Metadata, content_hash: vector<u8>): bool {
        metadata.content_hash == content_hash
    }
}