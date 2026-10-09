module test::abilities {
    struct NoAbilities {}

    struct DropOnly has drop {}

    struct KeyStore has key, store {
        id: UID,
    }

    struct AllAbilities has copy, drop, store, key {
        id: UID,
        value: u64,
    }

    public fun uses_abilities<T: key + store>(item: T): T {
        item
    }
}
