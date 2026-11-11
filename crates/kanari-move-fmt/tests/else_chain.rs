#[test]
fn preserves_else_if_chain() {
    let src = "module m::n {\n    fun f(c: u8): u8 {\n        if (c >= 48 && c <= 57) c - 48\n        // 0-9\n        else if (c >= 65 && c <= 70) c - 55\n        // A-F\n        else abort E\n    }\n}\n";
    let out =
        kanari_move_fmt::format_source(src, &kanari_move_fmt::FormatOptions::default()).unwrap();
    eprintln!("OUTPUT:\n{}", out);
    assert!(out.contains("else if"), "else-if chain dropped:\n{out}");
    assert!(out.contains("c - 55"), "second branch dropped:\n{out}");
    assert!(out.contains("abort E"), "final else dropped:\n{out}");
}
