module test::visibility_extra {
    public(package) fun package_only(): u64 {
        7
    }

    fun private_helper(): u64 {
        package_only()
    }
}
