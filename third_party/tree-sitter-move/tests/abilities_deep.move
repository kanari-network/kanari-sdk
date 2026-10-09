module test::abilities_deep {
    struct NoAbilities {
        a: u64,
        b: address,
    }

    public struct Keyed<phantom T> has key, store {
        id: UID,
        inner: NoAbilities,
    }

    public fun wrap<T>(id: UID, inner: NoAbilities): Keyed<T> {
        Keyed { id, inner }
    }

    public fun unwrap<T>(wrapped: Keyed<T>): NoAbilities {
        let Keyed { id: _, inner } = wrapped;
        inner
    }
}
