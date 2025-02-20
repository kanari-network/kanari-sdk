// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0


use std::{env, path::{Path, PathBuf}};

use kari_move::sandbox::commands::test;

fn run_all(args_path: &Path) -> datatest_stable::Result<()> {
    let cli_exe = env::var("CARGO_BIN_EXE_move")
        .expect("Failed to get move binary path from environment");
    let use_temp_dir = !args_path.parent().unwrap().join("NO_TEMPDIR").exists();
    test::run_one(
        args_path,
        &PathBuf::from(cli_exe),
        /* use_temp_dir */ use_temp_dir,
        /* track_cov */ false,
    )?;
    Ok(())
}

// runs all the tests
datatest_stable::harness! {
    { test = run_all, root = "tests/sandbox_tests", pattern = r"args\.txt$" },
}