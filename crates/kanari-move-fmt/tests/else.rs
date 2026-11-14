#[test]
fn preserves_else_branch() {
    let src = include_str!("../move_package/else_branch.move");
    let out =
        kanari_move_fmt::format_source(src, &kanari_move_fmt::FormatOptions::default()).unwrap();
    eprintln!("OUTPUT:\n{}", out);
    assert!(out.contains("} else"), "else branch dropped:\n{out}");
    assert!(out.contains("y = 2;"), "else body dropped:\n{out}");
}

#[test]
fn indents_if_body() {
    let src = include_str!("../move_package/indents_if_body.move");
    let out =
        kanari_move_fmt::format_source(src, &kanari_move_fmt::FormatOptions::default()).unwrap();
    assert!(out.contains("            y = 1;"), "missing if-body indent:\n{out}");
}
