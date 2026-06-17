// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

mod config;
mod execution;
mod report;
mod runner;
mod workload;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() -> Result<()> {
    let config = config::parse_args(std::env::args())?;
    let reports = runner::run_many(&config)?;
    eprintln!("{}", runner::render_reports(&config, &reports));
    runner::ensure_targets(&reports)
}
