// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Result, bail};
use kanari_open_rpc::OPENRPC_VERSION;
use kanari_open_rpc_spec::{build_and_save_kanari_rpc_spec, read_recorded_spec};
use kanari_rpc_api::methods;
use std::io::{self, Write};
use std::path::PathBuf;

pub use kanari_open_rpc_spec::build_kanari_rpc_spec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Print,
    Test,
    Record,
}

pub fn run_action(action: Action) -> Result<()> {
    match action {
        Action::Print => {
            let content = serde_json::to_string_pretty(&build_kanari_rpc_spec())?;
            let mut stdout = io::stdout().lock();
            stdout.write_all(content.as_bytes())?;
            stdout.write_all(b"\n")?;
            Ok(())
        }
        Action::Record => {
            let path = build_and_save_kanari_rpc_spec()?;
            let mut stdout = io::stdout().lock();
            stdout.write_all(path.display().to_string().as_bytes())?;
            stdout.write_all(b"\n")?;
            Ok(())
        }
        Action::Test => {
            let doc = build_kanari_rpc_spec();
            if doc.methods.is_empty() {
                bail!("OpenRPC doc has no methods");
            }
            if doc.openrpc != OPENRPC_VERSION {
                bail!("unexpected openrpc version {}", doc.openrpc);
            }
            let names: std::collections::BTreeSet<_> = doc.methods.iter().map(|m| m.name).collect();
            for required in [
                methods::GET_OWNER,
                methods::GET_STATS,
                methods::HEALTH,
                methods::VIEW_FUNCTION,
                methods::GET_OBJECT,
            ] {
                if !names.contains(required) {
                    bail!("required method missing from spec: {required}");
                }
            }

            let path = spec_file();
            if path.exists() {
                let actual = read_recorded_spec()?;
                let expected = serde_json::to_value(&doc)?;
                if actual != expected {
                    bail!(
                        "recorded spec is out of date: {}. Run with `record` to refresh.",
                        path.display()
                    );
                }
            }
            Ok(())
        }
    }
}

pub fn spec_file() -> PathBuf {
    kanari_open_rpc_spec::spec_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_spec_contains_core_methods() {
        let spec = build_kanari_rpc_spec();
        assert_eq!(spec.openrpc, OPENRPC_VERSION);
        assert!(spec.methods.iter().any(|method| method.name == methods::GET_OWNER));
        assert!(spec.methods.iter().any(|method| method.name == methods::HEALTH));
        assert!(spec.methods.iter().any(|method| method.name == methods::GET_OBJECT));
    }
}
