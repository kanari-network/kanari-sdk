module test::cross_module {
    use test::wrapped;

    public fun answer(): u64 {
        wrapped::value() + 41
    }
}

module test::wrapped {
    public fun value(): u64 {
        1
    }
}
