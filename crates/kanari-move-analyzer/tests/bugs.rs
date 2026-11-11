//! Adversarial bug-hunting tests for kanari-move-analyzer.

use kanari_move_analyzer::code_action::{
    extract_unresolved_name, insert_import_edit, local_dep_dirs, module_used_as_alias,
    scan_package_members, scan_package_members_and_modules,
};
use kanari_move_analyzer::deps::{dependency_caches, move_home};
use kanari_move_analyzer::diagnostics::is_kanari_false_positive;
use kanari_move_analyzer::edition::{MoveEdition, parse_edition};

mod common;

// ---------------------------------------------------------------------------
// edition

#[test]
fn parse_edition_weird_spacing() {
    assert_eq!(
        parse_edition("  edition   =   \"2024\"  "),
        MoveEdition::Edition2024
    );
    assert_eq!(parse_edition("edition=\"2024\""), MoveEdition::Edition2024);
    assert_eq!(parse_edition("edition\t=\t\"legacy\""), MoveEdition::Legacy);
    // single quotes are not valid TOML basic strings → Legacy per our impl
    assert_eq!(parse_edition("edition = '2024'"), MoveEdition::Legacy);
    // comment lines should be ignored
    assert_eq!(
        parse_edition("# edition = \"2024\"\n[package]\n"),
        MoveEdition::Legacy
    );
    assert_eq!(
        parse_edition("// edition = \"2024\"\n"),
        MoveEdition::Legacy
    );
    // unknown value → Legacy
    assert_eq!(parse_edition("edition = \"2099\""), MoveEdition::Legacy);
    // empty / whitespace
    assert_eq!(parse_edition(""), MoveEdition::Legacy);
    assert_eq!(parse_edition("   \n  \n"), MoveEdition::Legacy);
}

// ---------------------------------------------------------------------------
// extract_unresolved_name — edge cases found by fuzzing-style inputs

#[test]
fn extract_earliest_pattern_wins() {
    // whichever pattern appears first in the message wins
    assert_eq!(
        extract_unresolved_name("Unbound variable 'idx' and Could not resolve the name 'UID'"),
        Some("idx".to_string())
    );
    assert_eq!(
        extract_unresolved_name("Could not resolve the name 'UID' then Unbound type 'TxContext'"),
        Some("UID".to_string())
    );
}

#[test]
fn extract_bad_inputs() {
    assert_eq!(extract_unresolved_name(""), None);
    assert_eq!(extract_unresolved_name("Unbound"), None);
    assert_eq!(extract_unresolved_name("Unbound type ''"), None);
    assert_eq!(extract_unresolved_name("Unbound type '123'"), None);
    // backtick variant
    assert_eq!(
        extract_unresolved_name("Unresolved reference `TxContext`"),
        Some("TxContext".to_string())
    );
    // long trailing text after name should stop at non-word char
    assert_eq!(
        extract_unresolved_name("Unbound module alias 'tx_context' in current scope"),
        Some("tx_context".to_string())
    );
}

// ---------------------------------------------------------------------------
// module_used_as_alias

#[test]
fn module_used_ignores_comments() {
    // comment containing tx_context:: must not count
    assert!(!module_used_as_alias(
        "// tx_context::sender should not count\nlet x = 1;",
        "tx_context"
    ));
    assert!(!module_used_as_alias(
        "/// tx_context::\nlet x = 1;",
        "tx_context"
    ));
    // real usage after comment counts
    assert!(module_used_as_alias(
        "// comment\nlet y = tx_context::sender(ctx);",
        "tx_context"
    ));
}

#[test]
fn module_used_prefix_collision() {
    // `coin2::` must not count as `coin::`
    assert!(!module_used_as_alias("fun f() { coin2::value(); }", "coin"));
    assert!(module_used_as_alias("fun f() { coin::value(); }", "coin"));
    // module line itself must not count
    assert!(!module_used_as_alias(
        "module deepbook::registry { use x; }",
        "deepbook"
    ));
}

#[test]
fn module_used_inline_after_module_brace() {
    // `module x::m { fun f() { tx_context::sender(); } }` on one line
    assert!(module_used_as_alias(
        "module x::m { fun f() { tx_context::sender(); } }",
        "tx_context"
    ));
}

// ---------------------------------------------------------------------------
// insert_import_edit

#[test]
fn insert_import_no_module_is_none() {
    let lines = vec!["pub fun f() {}"];
    assert!(insert_import_edit(&lines, "x::y").is_none());
}

#[test]
fn insert_import_indent_preserved() {
    let lines = vec![
        "module x::m {",
        "    use std::bcs;",
        "    use kanari_system::event;",
        "    public fun f() {}",
        "}",
    ];
    let edit = insert_import_edit(&lines, "kanari_system::transfer").unwrap();
    assert_eq!(edit.range.start.line, 3, "after last use line");
    assert!(
        edit.new_text.starts_with("    use"),
        "indent lost: {:?}",
        edit.new_text
    );
}

#[test]
fn insert_import_after_module_when_no_use() {
    let lines = vec!["module x::m {", "    public fun f() {}", "}"];
    let edit = insert_import_edit(&lines, "x::y").unwrap();
    assert_eq!(edit.range.start.line, 1);
}

// ---------------------------------------------------------------------------
// scan

#[test]
fn scan_ignores_non_public_fun_for_members_but_captures_struct() {
    // TxContext is `struct TxContext` (no `public`) — must still be found
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(pkg.join("Move.toml"), "[package]\nname=\"x\"\n").unwrap();
    std::fs::write(
        pkg.join("sources").join("m.move"),
        "module kanari_system::tx_context { struct TxContext has drop {} }\n",
    )
    .unwrap();
    let (members, _) = scan_package_members_and_modules(&pkg);
    assert!(members.contains_key("TxContext"), "members: {members:?}");
}

#[test]
fn scan_empty_package_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(pkg.join("Move.toml"), "[package]\nname=\"x\"\n").unwrap();
    let (members, modules) = scan_package_members_and_modules(&pkg);
    assert!(members.is_empty());
    assert!(modules.is_empty());
}

#[test]
fn scan_ignores_non_move_files() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(pkg.join("Move.toml"), "[package]\nname=\"x\"\n").unwrap();
    std::fs::write(
        pkg.join("sources").join("README.md"),
        "module fake::x { public fun nope() {} }",
    )
    .unwrap();
    let (members, _) = scan_package_members_and_modules(&pkg);
    assert!(!members.contains_key("nope"));
}

#[test]
fn scan_package_members_pub_struct_enum_const() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(pkg.join("sources")).unwrap();
    std::fs::write(pkg.join("Move.toml"), "[package]\nname=\"x\"\n").unwrap();
    std::fs::write(
        pkg.join("sources").join("m.move"),
        "module a::b {\n public struct S {}\n public enum E { A }\n public const C: u64 = 1;\n}\n",
    )
    .unwrap();
    let map = scan_package_members(&pkg);
    assert!(map.contains_key("S"));
    assert!(map.contains_key("E"));
    assert!(map.contains_key("C"));
}

// ---------------------------------------------------------------------------
// local_dep_dirs

#[test]
fn local_dep_dirs_parses_spaces_and_comments() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("pkg");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("Move.toml"),
        "[dependencies]\nA = { local = \"./a\" }\nB = { local=\"../b\" } # comment\nC = { git = \"https://example.com\" }\n",
    )
    .unwrap();
    let dirs = local_dep_dirs(&pkg);
    assert_eq!(
        dirs.len(),
        2,
        "git dep must not be treated as local: {dirs:?}"
    );
    assert!(dirs[0].ends_with("a"));
    assert!(dirs[1].ends_with("b"));
}

#[test]
fn local_dep_dirs_missing_toml_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    assert!(local_dep_dirs(dir.path()).is_empty());
}

// ---------------------------------------------------------------------------
// deps helpers

#[test]
fn move_home_empty_env_ignored() {
    let _guard = common::lock_env();
    let old = std::env::var("MOVE_HOME").ok();
    unsafe { std::env::set_var("MOVE_HOME", "   ") };
    // empty/whitespace MOVE_HOME falls back to ~/.move (cannot easily assert, just must not panic)
    let h = move_home();
    assert!(!h.to_string_lossy().is_empty());
    match old {
        Some(v) => unsafe { std::env::set_var("MOVE_HOME", v) },
        None => unsafe { std::env::remove_var("MOVE_HOME") },
    }
}

#[test]
fn dependency_caches_skips_plain_dirs() {
    let _guard = common::lock_env();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("plain")).unwrap();
    std::fs::create_dir_all(dir.path().join("gitrepo").join(".git")).unwrap();
    let old = std::env::var("MOVE_HOME").ok();
    unsafe { std::env::set_var("MOVE_HOME", dir.path()) };
    let caches = dependency_caches();
    assert_eq!(caches.len(), 1, "{caches:?}");
    match old {
        Some(v) => unsafe { std::env::set_var("MOVE_HOME", v) },
        None => unsafe { std::env::remove_var("MOVE_HOME") },
    }
}

// ---------------------------------------------------------------------------
// diagnostics false positives

#[test]
fn false_positive_case_sensitive() {
    // Sui wording is lower-case `sui::object::UID` — different case must not match
    assert!(!is_kanari_false_positive(
        "Invalid object 'X'. Must have 'id: SUI::object::UID' as first field"
    ));
}

#[test]
fn false_positive_prefix_stripped() {
    // extra leading text means message does NOT start with Invalid object
    assert!(!is_kanari_false_positive(
        "Error: Invalid object 'X'. must have 'id: sui::object::UID'"
    ));
}
