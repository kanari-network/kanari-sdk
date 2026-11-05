// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Build script: regenerate the recorded OpenRPC spec whenever `cargo build` runs,
//! using the same `Record` code path as `kanari-open-rpc-spec-builder -- record`.

use kanari_open_rpc_spec::build_and_save_kanari_rpc_spec;

fn main() {
    let path = build_and_save_kanari_rpc_spec().expect("write openrpc spec");
    println!("cargo:rerun-if-changed=../kanari-rpc-api/src");
    println!("cargo:rerun-if-changed={}", path.display());
}
