module kanari_pet::nft {
    use kanari_system::event;

    struct MintLog has copy, drop {
        object_id: address,
        creator: address,
        collection_id: address,
    }

    public fun emit_mint_log(
        object_id: address,
        creator: address,
        collection_id: address,
    ) {
        event::emit(MintLog { object_id, creator, collection_id });
    }
}
