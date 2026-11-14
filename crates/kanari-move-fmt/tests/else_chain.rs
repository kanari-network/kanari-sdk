#[test]
fn preserves_else_if_chain() {
    let src = include_str!("../move_package/else_if_chain.move");
    let out =
        kanari_move_fmt::format_source(src, &kanari_move_fmt::FormatOptions::default()).unwrap();
    eprintln!("OUTPUT:\n{}", out);
    assert!(out.contains("else if"), "else-if chain dropped:\n{out}");
    assert!(out.contains("c - 55"), "second branch dropped:\n{out}");
    assert!(out.contains("abort E"), "final else dropped:\n{out}");
}
