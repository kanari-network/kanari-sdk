#[test]
fn preserves_else_branch() {
    let src = "module m::n {\n    public fun f(x: u64) {\n        if (x > 0) {\n            y = 1;\n        } else {\n            y = 2;\n        }\n    }\n}\n";
    let out =
        kanari_move_fmt::format_source(src, &kanari_move_fmt::FormatOptions::default()).unwrap();
    eprintln!("OUTPUT:\n{}", out);
    assert!(out.contains("} else"), "else branch dropped:\n{out}");
    assert!(out.contains("y = 2;"), "else body dropped:\n{out}");
}

#[test]
fn indents_if_body() {
    let src = "module m::n {\n    public fun f() {\n        if (x > 0) {\n            y = 1;\n        }\n    }\n}\n";
    let out =
        kanari_move_fmt::format_source(src, &kanari_move_fmt::FormatOptions::default()).unwrap();
    assert!(
        out.contains("            y = 1;"),
        "missing if-body indent:\n{out}"
    );
}
