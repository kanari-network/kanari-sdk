module img::img {
        use std::vector;
        use kanari_framework::metadata::{Self, Metadata};
    
        fun store_file_metadata(
            content_hash: vector<u8>, 
            owner: address
        ): Metadata {
            let metadata = metadata::new();
            metadata::set_owner(&mut metadata, owner);
            metadata::set_hash(&mut metadata, content_hash); 
            metadata::store(metadata)
        }
    
        fun verify_file(
            metadata: &Metadata,
            content_hash: vector<u8>
        ): bool {
            metadata::verify_hash(metadata, content_hash)
        }
    
        #[test]
        fun test_metadata_flow() {
            let hash = vector::empty<u8>();
            let owner = @0x1;
            let metadata = store_file_metadata(hash, owner);
            assert!(verify_file(&metadata, hash), 0);
        }
}