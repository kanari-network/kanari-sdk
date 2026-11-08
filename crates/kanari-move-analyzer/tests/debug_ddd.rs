//! Regression: a stray token at module level must be reported.
//! (This is the "ddd doesn't warn" case — `unexpected token` is a real
//! diagnostic, never filtered.)

use move_compiler::command_line::compiler::move_check_for_errors;
use move_compiler::diagnostics::{Diagnostics, report_diagnostics_to_buffer};
use move_compiler::editions::{Edition, Flavor};
use move_compiler::shared::{Flags, NumericalAddress, PackageConfig, PackagePaths};
use move_compiler::{Compiler, PASS_PARSER};
use std::collections::BTreeMap;

fn testing_addresses() -> BTreeMap<String, NumericalAddress> {
    vec![("std", "0x1"), ("sui", "0x2"), ("M", "0x40")]
        .into_iter()
        .map(|(name, addr)| (name.to_string(), NumericalAddress::parse_str(addr).unwrap()))
        .collect()
}

fn compile_source(source: &str) -> (String, Diagnostics) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.move");
    std::fs::write(&path, source).unwrap();
    let path_str = path.to_string_lossy().replace('\\', "/");

    let targets = vec![PackagePaths {
        name: None,
        paths: vec![path_str],
        named_address_map: testing_addresses(),
    }];
    let config = PackageConfig {
        edition: Edition::E2024,
        flavor: Flavor::default(),
        ..PackageConfig::default()
    };
    let compiler = Compiler::from_package_paths(None, targets, vec![])
        .unwrap()
        .set_flags(Flags::empty().set_sources_shadow_deps(true))
        .set_default_config(config);
    let (files, res) = compiler.run::<PASS_PARSER>().unwrap();
    let diags = move_check_for_errors(res);
    let rendered = if diags.is_empty() {
        String::new()
    } else {
        String::from_utf8(report_diagnostics_to_buffer(&files, diags.clone(), false)).unwrap()
    };
    let _ = dir.close();
    (rendered, diags)
}

#[test]
fn stray_token_is_reported() {
    // `ddd` at module level is unexpected (E01002) — the editor must show it.
    let src = "module M::f {\n    ddd\n}\n";
    let (rendered, diags) = compile_source(src);
    assert!(!diags.is_empty(), "stray token must produce diagnostics");
    assert!(
        rendered.contains("ddd") && rendered.contains("unexpected"),
        "diagnostic must mention the stray token: {rendered}"
    );
}
