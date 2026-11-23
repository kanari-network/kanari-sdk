#![no_main]
use libfuzzer_sys::fuzz_target;
use kanari_types::address::Address as KanariAddress;
use std::str::FromStr;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Must not panic on arbitrary input.
        let _ = KanariAddress::from_str(s);
        let _ = KanariAddress::parse_to_account_address(s);
    }
});