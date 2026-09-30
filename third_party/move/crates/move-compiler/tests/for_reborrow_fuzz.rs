// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Differential-by-construction fuzz for `for` loops and explicit reborrows (`&*r`, `&mut *r`).
//!
//! Each case is a complete module generated from a deterministic PRNG seed. Valid shapes must
//! compile cleanly all the way to bytecode (exercising naming, typing, HLIR, the borrow checker,
//! and codegen); invalid shapes must fail with proper diagnostics. Anything else -- a panic, an
//! ICE, or the compiler hanging -- fails the test. Panics are caught per seed so one bad seed
//! reports instead of aborting the whole run.
//!
//! The number of seeds can be raised for a longer run, e.g.
//! `FOR_REBORROW_FUZZ_SEEDS=2000 cargo test -p move-compiler --test for_reborrow_fuzz`.

use std::{
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
};

use move_compiler::{
    Compiler, PASS_COMPILATION,
    editions::Edition,
    shared::{Flags, NumericalAddress, PackageConfig, PackagePaths},
};

//**************************************************************************************************
// PRNG
//**************************************************************************************************

struct Gen {
    state: u64,
}

impl Gen {
    fn new(seed: u64) -> Self {
        // SplitMix64 seeding for a nonzero start.
        let mut z = seed.wrapping_add(0x9e3779b97f4a7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        let state = z ^ (z >> 31);
        Self {
            state: if state == 0 {
                0x853c49e6748fea9b
            } else {
                state
            },
        }
    }

    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545f4914f6cdd1d)
    }

    fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }

    fn one_in(&mut self, n: u64) -> bool {
        self.below(n) == 0
    }
}

//**************************************************************************************************
// Program generation
//**************************************************************************************************

// A generated case is either valid (must compile) or invalid (must fail with diagnostics).
struct Case {
    source: String,
    expect_ok: bool,
}

fn prelude() -> String {
    r#"module 0x42::fuzz {
    public struct NoCopy has drop { v: u64 }
    public struct Holder has drop { items: vector<u64>, nc: vector<NoCopy> }

    fun make_u64s(): vector<u64> { vector[1, 2, 3, 4, 5] }
    fun make_nc(): vector<NoCopy> { vector[NoCopy { v: 1 }, NoCopy { v: 2 }] }
    fun use_u64(_x: u64) { }
    fun use_ref(_x: &u64) { }
"#
    .to_string()
}

// Iterable expressions over a `vector<u64>` local or place. Each is paired with the loop-variable
// type it yields (`u64`, `&u64`, `&mut u64`) so bodies can use it correctly. `r0`/`mr0` are the
// reference parameters of the generated `test` function.
fn u64_iterables() -> Vec<(&'static str, &'static str)> {
    vec![
        ("v", "u64"),
        ("&v", "&u64"),
        ("&mut v", "&mut u64"),
        ("&h.items", "&u64"),
        ("&mut h.items", "&mut u64"),
        ("&*r0", "&u64"),
        ("&mut *mr0", "&mut u64"),
        ("make_u64s()", "u64"),
    ]
}

// How to read a `u64` loop variable of each type in an expression, and how to write through it.
fn read(var: &str, ty: &str) -> String {
    match ty {
        "u64" => var.to_string(),
        "&u64" | "&mut u64" => format!("*{var}"),
        _ => unreachable!(),
    }
}

fn write(var: &str, ty: &str, val: &str) -> Option<String> {
    match ty {
        "&mut u64" => Some(format!("*{var} = {val};")),
        // By-value and shared variables cannot be assigned through.
        _ => None,
    }
}

fn gen_body(g: &mut Gen, var: &str, ty: &str, depth: u8) -> String {
    let mut stmts = vec![format!("total = total + {};", read(var, ty))];
    // Occasionally mutate through a mutable element.
    if let Some(w) = write(var, ty, "7")
        && g.one_in(3)
    {
        stmts.push(w);
    }
    // Occasionally break or continue on a value-dependent guard.
    if g.one_in(3) {
        stmts.push(format!("if ({} == 3) {{ break }};", read(var, ty)));
    }
    if g.one_in(4) {
        stmts.push(format!("if ({} == 1) {{ continue }};", read(var, ty)));
    }
    // Occasionally nest another loop (bounded depth). The inner loop always drives off a
    // fresh temporary so it can never conflict with an outer borrow.
    if depth < 2 && g.one_in(3) {
        stmts.push(format!(
            "for inner in make_u64s() {{ total = total + inner + {}; }};",
            read(var, ty)
        ));
    }
    // Occasionally return early.
    if g.one_in(5) {
        stmts.push(format!("if ({} == 4) {{ return total }};", read(var, ty)));
    }
    stmts.join("\n            ")
}

fn gen_valid(g: &mut Gen) -> String {
    let mut src = prelude();
    // Locals every case works with. `v`/`h` are owned places to borrow from; `r`/`mr` exercise
    // explicit reborrows of parameters.
    src.push_str(
        r#"    fun test(v0: vector<u64>, r0: &vector<u64>, mr0: &mut vector<u64>): u64 {
        let mut total = 0u64;
        let mut v = v0;
        let mut h = Holder { items: make_u64s(), nc: make_nc() };
"#,
    );
    // A by-value `for` consumes its vector, and a `&mut *` consumes its reference, so those
    // iterables are removed from the pool once used. Shared borrows and fresh calls never run
    // out. `h` is only ever borrowed (never consumed), so it stays.
    let mut pool: Vec<(&str, &str)> = u64_iterables();
    let nloops = 1 + g.below(3);
    for _ in 0..nloops {
        let idx = g.below(pool.len() as u64) as usize;
        let (iter, ty) = pool[idx];
        let body = gen_body(g, "x", ty, 0);
        src.push_str(&format!(
            "        for x in {iter} {{\n            {body}\n        }};\n"
        ));
        match iter {
            // By value consumes the owned local.
            "v" | "make_u64s()" => {
                pool.retain(|(i, _)| *i != "v" && *i != "&v" && *i != "&mut v");
            }
            // A mutable reborrow consumes the reference.
            "&mut *mr0" => {
                pool.retain(|(i, _)| *i != "&mut *mr0");
            }
            _ => (),
        }
        if pool.is_empty() {
            break;
        }
    }
    // Standalone reborrows on fresh locals, so they cannot conflict with anything above.
    match g.below(4) {
        0 => src.push_str(
            "        let fresh0 = make_u64s();\n        let a = &fresh0;\n        let b = &*a;\n        total = total + vector::length(b);\n",
        ),
        1 => src.push_str(
            "        let mut fresh1 = make_u64s();\n        let a = &mut fresh1;\n        let b = &mut *a;\n        vector::push_back(b, 9);\n",
        ),
        2 => src.push_str(
            "        let fresh2 = make_u64s();\n        let a = &fresh2;\n        let b = if (total == 0) &*a else a;\n        total = total + vector::length(b);\n",
        ),
        _ => src.push_str(
            "        let mut fresh3 = make_u64s();\n        let a = &mut fresh3;\n        let mut i = 0u64;\n        while (i < 2) {\n            let b = &mut *a;\n            vector::push_back(b, i);\n            i = i + 1;\n        };\n",
        ),
    }
    src.push_str("        total\n    }\n}\n");
    src
}

fn gen_invalid(g: &mut Gen) -> String {
    let mut src = prelude();
    src.push_str(
        r#"    fun test(v0: vector<u64>, r0: &vector<u64>): u64 {
        let mut total = 0u64;
        let mut v = v0;
"#,
    );
    match g.below(4) {
        // `&mut *r` from a shared reference.
        0 => src.push_str("        let r = &v;\n        let x = &mut *r;\n"),
        // `&*e` where `e` is not a reference.
        1 => src.push_str("        let x = &*v;\n"),
        // By-value `for` over a reference (no reborrow marker).
        _ => src.push_str("        for x in r0 {\n            total = total + x;\n        };\n"),
    }
    src.push_str("        total\n    }\n}\n");
    src
}

fn gen_case(seed: u64) -> Case {
    let mut g = Gen::new(seed);
    // Roughly 3 in 4 cases are valid; the rest must fail gracefully.
    if g.one_in(4) {
        Case {
            source: gen_invalid(&mut g),
            expect_ok: false,
        }
    } else {
        Case {
            source: gen_valid(&mut g),
            expect_ok: true,
        }
    }
}

//**************************************************************************************************
// Compilation driver
//**************************************************************************************************

fn compile(source: &str, seed: u64) -> Result<bool, String> {
    let mut dir = std::env::temp_dir();
    dir.push(format!("for_reborrow_fuzz_{}_{}", std::process::id(), seed));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut path = PathBuf::from(&dir);
    path.push("fuzz.move");
    std::fs::write(&path, source).map_err(|e| e.to_string())?;

    let named_address_map = BTreeMap::from([
        (
            "std".to_string(),
            NumericalAddress::parse_str("0x1").unwrap(),
        ),
        (
            "M".to_string(),
            NumericalAddress::parse_str("0x40").unwrap(),
        ),
    ]);
    let deps = vec![PackagePaths {
        name: Some(("stdlib".into(), PackageConfig::default())),
        paths: move_stdlib::move_stdlib_files(),
        named_address_map: named_address_map.clone(),
    }];
    let targets = vec![PackagePaths {
        name: None,
        paths: vec![path.to_string_lossy().replace('\\', "/")],
        named_address_map,
    }];
    let config = PackageConfig {
        edition: Edition::E2024,
        ..PackageConfig::default()
    };

    let compiler = Compiler::from_package_paths(None, targets, deps)
        .map_err(|e| format!("setup: {e:#}"))?
        .set_flags(Flags::empty().set_sources_shadow_deps(true))
        .set_default_config(config);
    // Full compilation: parser, expansion, naming, typing, HLIR, borrow checking, bytecode.
    // The outer `anyhow::Result` is only for setup/IO failures. Compile errors come back in the
    // inner `Err` with diagnostics; that is a graceful failure, not a crash.
    match compiler.run::<PASS_COMPILATION>() {
        Ok((_, Ok(_))) => Ok(true),
        Ok((_, Err((_, diags)))) if !diags.is_empty() => Ok(false),
        Ok((_, Err((_, _)))) => Err("compiler failed without diagnostics".to_string()),
        Err(e) => Err(format!("setup: {e:#}")),
    }
}

fn seed_count() -> u64 {
    std::env::var("FOR_REBORROW_FUZZ_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300)
}

#[test]
fn for_reborrow_fuzz() {
    let n = seed_count();
    let dump = std::env::var("FOR_REBORROW_FUZZ_DUMP").is_ok();
    let mut failures = vec![];
    for seed in 0..n {
        let case = gen_case(seed);
        if dump {
            eprintln!(
                "=== seed {seed} expect_ok={} ===\n{}",
                case.expect_ok, case.source
            );
        }
        let result = catch_unwind(AssertUnwindSafe(|| compile(&case.source, seed)));
        match result {
            Err(_) => failures.push(format!("seed {seed}: compiler panicked")),
            Ok(Err(e)) => failures.push(format!("seed {seed}: harness error: {e}")),
            Ok(Ok(ok)) if ok != case.expect_ok => {
                if case.expect_ok {
                    failures.push(format!("seed {seed}: valid case failed to compile"));
                } else {
                    failures.push(format!("seed {seed}: invalid case compiled clean"));
                }
            }
            Ok(Ok(_)) => (),
        }
    }
    assert!(
        failures.is_empty(),
        "for/reborrow fuzz failures ({} seeds):\n{}",
        n,
        failures.join("\n")
    );
}
