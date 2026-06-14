// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_cli::sandbox::commands::test;

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn run_all(args_path: &Path) -> datatest_stable::Result<()> {
    let _guard = TEST_LOCK.lock().unwrap();
    let cli_exe = env!("CARGO_BIN_EXE_move");
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
    {
        test = run_all,
        root = "tests/move_unit_tests",
        pattern = r"args(\.stackless)?\.txt$",
    },
}
