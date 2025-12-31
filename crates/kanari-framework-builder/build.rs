// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Build script for compiling Move packages in kanari-frameworks/packages
// using the move-package crate.
use std::env;
use std::fs;
use std::io::{self};
use std::path::PathBuf;

fn main() {
    // Paths
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let packages_dir = manifest_dir
        .join("..")
        .join("kanari-frameworks")
        .join("packages");

    println!("cargo:rerun-if-changed={}", packages_dir.display());

    if !packages_dir.exists() {
        println!(
            "cargo:warning=Packages directory not found: {}",
            packages_dir.display()
        );
        return;
    }

    // Iterate packages and compile using move-package API
    for entry in fs::read_dir(&packages_dir).unwrap_or_else(|e| {
        panic!(
            "Failed to read packages dir {}: {}",
            packages_dir.display(),
            e
        )
    }) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                println!("cargo:warning=Failed to read entry in packages dir: {}", e);
                continue;
            }
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        println!("cargo:warning=Compiling Move package: {}", path.display());

        // Use move-package crate programmatically
        // install_dir points to the package dir, compiler will create build/ inside it
        let config = move_package::BuildConfig {
            install_dir: Some(path.clone()),
            ..Default::default()
        };

        // compile_package consumes the config
        match config.compile_package(&path, &mut io::stdout()) {
            Ok(_compiled) => {
                println!("cargo:warning=Compiled package {}", path.display());
            }
            Err(e) => {
                // Fail the build script to stop cargo build on compile error
                panic!("Failed to compile Move package {}: {}", path.display(), e);
            }
        }
    }
}
