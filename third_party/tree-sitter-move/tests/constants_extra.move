module test::constants {
    /// Error codes
    const E_NOT_FOUND: u64 = 0;
    const E_INVALID: u64 = 1;
    const E_EMPTY: u64 = 2;

    const MAX_U64: u64 = 18446744073709551615;
    const NAME: vector<u8> = b"kanari";
    const FLAG: bool = true;
    const ZERO_ADDRESS: address = @0x0;
}
