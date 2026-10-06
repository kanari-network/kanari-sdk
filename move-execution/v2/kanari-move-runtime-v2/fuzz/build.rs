// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Link fix for the MSVC toolchain.
//!
//! Every fuzz target is `#![no_main]`; the `main` entry point comes from
//! libFuzzer's `FuzzerMain.cpp`, archived into `fuzzer.lib` by `libfuzzer-sys`.
//! MSVC only extracts an object from a static library when something already
//! references one of its symbols, and nothing references `main` before the CRT
//! entry is resolved, so `link.exe` fails with LNK1561 (entry point must be
//! defined). Forcing the symbol to be resolved pulls the object in.
//!
//! The flag is emitted only for `*-msvc` targets, so Linux/macOS builds and
//! `cargo fuzz` runs are unaffected (cargo-fuzz passes `/include:main` itself).
//!
//! Directives must be written to stdout: Cargo parses a build script's stdout
//! for `cargo:` directives and ignores stderr entirely. Emitting via `eprintln!`
//! drops the flag silently and the link then fails exactly as if this build
//! script did not exist.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg=/INCLUDE:main");
    }
}
