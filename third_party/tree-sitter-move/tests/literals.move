module test::literals {
    const BYTES: vector<u8> = b"hello";
    const HEX: vector<u8> = x"00ff";
    const URL: vector<u8> = b"https://example.com/icon.png";

    public fun make(): vector<u64> {
        let empty = vector::empty<u64>();
        vector::push_back(&mut empty, 1);
        vector::push_back(&mut empty, 2);
        empty
    }
}
