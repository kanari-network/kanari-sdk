// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Differential-testing helper: format one file to stdout.
#![allow(clippy::print_stdout)]
use std::io::Read;

fn main() {
    let path = match std::env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: dump <file.move>");
            std::process::exit(2);
        }
    };
    let mut source = String::new();
    std::fs::File::open(&path)
        .expect("open")
        .read_to_string(&mut source)
        .expect("read");
    let out = kanari_move_fmt::format_source(&source, &kanari_move_fmt::FormatOptions::default())
        .expect("format");
    print!("{out}");
}
