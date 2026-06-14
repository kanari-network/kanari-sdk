// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub const TEST_DIR: &str = "tests";
use move_transactional_test_runner::vm_test_harness::run_test;

datatest_stable::harness! {
    { test = run_test, root = TEST_DIR, pattern = r".*\.(mvir|move)$" },
}
