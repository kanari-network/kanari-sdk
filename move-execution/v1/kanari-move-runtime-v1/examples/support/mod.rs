//! Shared setup for the e2e publish examples.
//!
//! Both `e2e_token_publish` and `e2e_nft_publish` do the same five things
//! before they can do anything example-specific: find a compiled `.mv`, build a
//! runtime, deserialize it, publish it as its own address, and encode a
//! `TxContext` for a no-argument entry call. Only the module file name and the
//! console wording differ, so those are the only parameters here.

use anyhow::{Context, Result};
use kanari_move_runtime_v1::changeset::ChangeSet;
use kanari_move_runtime_v1::move_runtime::MoveRuntime;
use kanari_move_runtime_v1::storage::persistent_store::PersistentStore;
use move_binary_format::file_format::CompiledModule;
use move_core_types::account_address::AccountAddress;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where the compiled Move modules of the `james` example package land,
/// relative to either the crate directory or the repository root.
const MODULE_SUBDIR: &str = "example_move/james/build/james/bytecode_modules";

/// Locate `file_name` under the first search root that has it.
///
/// `cli_path` wins when it points at an existing file, so a built module
/// outside this repo can still be exercised. Otherwise the lookup is anchored
/// to `CARGO_MANIFEST_DIR` as well as the working directory: `cargo run
/// --example` executes with the crate directory as cwd, but the built package
/// lives at the repository root, so a purely relative search misses it
/// depending on where the example was launched from.
pub fn locate_module(file_name: &str, cli_path: Option<&str>) -> Result<PathBuf> {
    if let Some(cli_path) = cli_path {
        let path = PathBuf::from(cli_path);
        if path.exists() {
            return Ok(path);
        }
    }

    let relative = Path::new(MODULE_SUBDIR).join(file_name);
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        relative.clone(),
        manifest_dir.join(&relative),
        manifest_dir.join("../../../").join(&relative),
    ];

    candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
        .with_context(|| {
            format!(
                "compiled {file_name} not found; searched [{}]. \
                 Build the Move package first, or pass a path as the first argument.",
                candidates
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// Read and deserialize a compiled module, keeping the raw bytes for publishing.
pub fn read_module(path: &Path) -> Result<(CompiledModule, Vec<u8>)> {
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let compiled = CompiledModule::deserialize_with_defaults(&bytes)
        .with_context(|| format!("failed to deserialize {}", path.display()))?;
    Ok((compiled, bytes))
}

/// Directory this example's runtime database lives in.
///
/// `MoveRuntime::new_with_kanari_natives()` resolves its store from
/// `KANARI_MOVE_VM_DB` and, when that is unset, falls back to the node's real
/// database under `~/.kanari`. An example that used it directly would write
/// published modules into live node state just by being run, so examples go
/// through an explicit scratch directory instead.
pub fn example_db_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "kanari-move-runtime-example-{}",
        std::process::id()
    ))
}

/// Delete the directory [`example_db_dir`] points at.
///
/// Windows keeps RocksDB file locks briefly after the last handle closes, so
/// this is best-effort and never fails the example.
pub fn cleanup_db_dir() {
    let dir = example_db_dir();
    for attempt in 1..=5 {
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => return,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(_) if attempt < 5 => {
                std::thread::sleep(std::time::Duration::from_millis(200 * attempt));
            }
            Err(e) => {
                eprintln!("could not remove {}: {e}", dir.display());
                return;
            }
        }
    }
}

/// Build a runtime with the Kanari natives and framework modules preloaded,
/// backed by a scratch store in the system temp directory.
pub fn runtime() -> Result<MoveRuntime> {
    let db_dir = example_db_dir();
    if db_dir.exists() {
        let _ = std::fs::remove_dir_all(&db_dir);
    }
    let store = PersistentStore::open_with_path(Some(db_dir))
        .context("failed to open the example's scratch store")?;
    MoveRuntime::new_with_kanari_natives_and_store(Arc::new(store))
        .context("failed to init MoveRuntime")
}

/// Publish `bytes` under its own address.
///
/// The VM requires the sender to equal the module address, so examples never
/// pass their own address here.
pub fn publish(
    runtime: &MoveRuntime,
    bytes: Vec<u8>,
    module_address: AccountAddress,
) -> Result<ChangeSet> {
    runtime
        .publish_module(bytes, module_address, None, None)
        .context("publish_module failed")
}

/// First positional CLI argument, if the example was given one.
pub fn cli_arg(args: &[String], index: usize) -> Option<&str> {
    args.get(index).map(String::as_str)
}
