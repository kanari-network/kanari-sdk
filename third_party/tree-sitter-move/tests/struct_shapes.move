module test::struct_shapes {
    struct Empty {}

    struct Named {
        id: UID,
        owner: address,
    }

    struct Nested {
        inner: Empty,
        named: Named,
    }

    public fun build(owner: address, ctx: &mut TxContext): Nested {
        Nested {
            inner: Empty {},
            named: Named { id: object::new(ctx), owner },
        }
    }
}
