// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use kanari_open_rpc::Project;
use kanari_rpc_api::methods;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub const SPEC_RELATIVE_PATH: &str = "schemas/openrpc.json";
pub const SPEC_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn spec_file() -> PathBuf {
    crate_dir().join(SPEC_RELATIVE_PATH)
}

pub fn read_recorded_spec() -> Result<Value> {
    let path = spec_file();
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("invalid json in {}", path.display()))
}

pub fn write_recorded_spec(project: &Project) -> Result<PathBuf> {
    let spec_path = spec_file();
    write_spec_to_path(project, &spec_path)?;
    Ok(spec_path)
}

pub fn write_spec_to_path(project: &Project, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create spec directory {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(project)?;
    fs::write(path, format!("{content}\n"))
        .with_context(|| format!("failed to write {}", path.display()))
}

pub fn kanari_rpc_doc(version: &'static str) -> Project {
    Project::new(
        version,
        "Kanari JSON-RPC",
        "Kanari JSON-RPC API for interaction with kanari server.",
        "Kanari Network",
        "https://kanarinetwork.site",
        "opensource@kanarinetwork.site",
        "Apache-2.0",
        "https://raw.githubusercontent.com/kanari-network/kanari-sdk/main/LICENSE",
    )
}

pub fn build_kanari_rpc_spec() -> Project {
    let mut open_rpc = kanari_rpc_doc(SPEC_VERSION);
    open_rpc.add_methods(methods::open_rpc_methods());
    open_rpc.normalize_schemas();
    open_rpc
}

pub fn build_and_save_kanari_rpc_spec() -> Result<PathBuf> {
    let open_rpc = build_kanari_rpc_spec();
    write_recorded_spec(&open_rpc)
}
