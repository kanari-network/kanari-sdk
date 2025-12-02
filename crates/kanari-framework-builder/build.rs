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

        // Prepare BuildConfig with install_dir = <pkg>/build
        let out_dir = path.join("build");
        if let Err(e) = fs::create_dir_all(&out_dir) {
            println!(
                "cargo:warning=Failed to create output dir {}: {}",
                out_dir.display(),
                e
            );
            continue;
        }

        println!(
            "cargo:warning=Compiling Move package: {} -> {}",
            path.display(),
            out_dir.display()
        );

        // Use move-package crate programmatically
        let mut config = move_package::BuildConfig::default();
        config.install_dir = Some(out_dir.clone());

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
