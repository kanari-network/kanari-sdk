#![no_main]
use libfuzzer_sys::fuzz_target;
use kanari_core::decode_hex_exact;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        for n in [1usize, 2, 16, 32, 64] {
            let _ = decode_hex_exact("fuzz", s, n);
        }
    }
});