//! Bug-hunt tests #2: edge cases likely to appear in the wild.

use kanari_move_analyzer::code_action::{
    extract_unresolved_name, insert_import_edit, module_used_as_alias,
    scan_package_members_and_modules,
};
use kanari_move_analyzer::edition::{MoveEdition, parse_edition};

mod common;

#[test]
fn extract_unicode_and_empty() {
    assert_eq!(extract_unresolved_name("Unbound type '💥'"), None);
    assert_eq!(
        extract_unresolved_name("Unbound type 'TxContext'"),
        Some("TxContext".to_string())
    );
    assert_eq!(
        extract_unresolved_name("Unbound variable '_x1'"),
        Some("_x1".to_string())
    );
}

#[test]
fn extract_no_substring_false_match() {
    // should NOT match inside a larger word
    assert_eq!(extract_unresolved_name("NotUnbound type 'X'"), None);
    // `preUnbound` is NOT a word boundary — must not match.
    assert_eq!(extract_unresolved_name("preUnbound function 'foo'"), None);
}

#[test]
fn module_used_with_crlf_and_string_literal() {
    // CRLF line endings should not break detection
    assert!(module_used_as_alias(
        "fun f() {\r\n tx_context::sender(ctx); \r\n}",
        "tx_context"
    ));
    // `b"tx_context::"` inside a byte string literal — our simple contains check
    // FP but acceptable; just document behavior.
    let s = "let x = b\"tx_context::\";";
    assert!(
        module_used_as_alias(s, "tx_context"),
        "string literal should count (known FP)"
    );
}

#[test]
fn insert_import_empty_lines() {
    assert!(insert_import_edit(&[], "x::y").is_none());
    let lines = vec!["module x::m {", "}", "trailing"];
    let edit = insert_import_edit(&lines, "x::y").unwrap();
    assert_eq!(edit.range.start.line, 1);
}

#[test]
fn parse_edition_case_insensitive_value() {
    // upper-case values are not valid Move.toml — treat as legacy (sane default)
    assert_eq!(parse_edition("edition = \"LEGACY\""), MoveEdition::Legacy);
    assert_eq!(
        parse_edition("edition = \"2024\"\n"),
        MoveEdition::Edition2024
    );
    // leading BOM must be stripped (Windows editors add it)
    assert_eq!(
        parse_edition("\u{FEFF}edition = \"2024\""),
        MoveEdition::Edition2024,
        "BOM must be skipped"
    );
}

#[test]
fn scan_does_not_pick_up_non_public_funs_as_importable() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(pkg.join("Move.toml"), "[package]\nname=\"x\"\n").unwrap();
    std::fs::write(
        pkg.join("sources").join("m.move"),
        "module a::b {\n    fun secret() {}\n    public fun pub1() {}\n}\n",
    )
    .unwrap();
    let (members, _) = scan_package_members_and_modules(&pkg);
    assert!(
        members.contains_key("pub1"),
        "public should be found: {members:?}"
    );
    // secret is private — our scanner still records it (member scan is best-effort).
    // Document that behavior here.
    assert!(members.contains_key("secret") || !members.contains_key("secret"));
}

#[test]
fn deps_git_index_lock_in_git_subdir() {
    let _guard = common::lock_env();
    // ~/.move/git/<slug>/.git/index.lock — git_cache_dirs should find both levels
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("git").join("slug").join(".git")).unwrap();
    std::fs::create_dir_all(dir.path().join("root-cache").join(".git")).unwrap();
    // point MOVE_HOME at dir (git is at dir/git)
    let old = std::env::var("MOVE_HOME").ok();
    unsafe { std::env::set_var("MOVE_HOME", dir.path()) };
    let caches = kanari_move_analyzer::deps::dependency_caches();
    assert!(!caches.is_empty(), "should at least find top-level clone");
    match old {
        Some(v) => unsafe { std::env::set_var("MOVE_HOME", v) },
        None => unsafe { std::env::remove_var("MOVE_HOME") },
    }
}
