// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::reroot_path;
use clap::*;
use move_package::BuildConfig;
use rand::{thread_rng, Rng};
use serde_json::{json, Value as JsonValue};
use std::{iter, path::PathBuf};

/// Build the package at `path`. If no path is provided defaults to current directory.
#[derive(Parser)]
#[clap(name = "build")]
pub struct Build;

// This is a temporary solution to generate object IDs for functions and structs
fn generate_object_id() -> String {
    let mut rng = thread_rng();
    let hex_chars: Vec<char> = "0123456789abcdef".chars().collect();
    let random_hex: String = iter::repeat(())
        .map(|()| hex_chars[rng.gen_range(0..16)])
        .take(64)
        .collect();
    format!("0x{}", random_hex)
}

impl Build {
    pub fn execute(self, path: Option<PathBuf>, config: BuildConfig) -> anyhow::Result<()> {
        let rerooted_path = reroot_path(path)?;
        if config.fetch_deps_only {
            let mut config = config;
            if config.test_mode {
                config.dev_mode = true;
            }
            config.download_deps_for_package(&rerooted_path, &mut std::io::stdout())?;
            println!(
                "{}",
                json!({
                    "status": "success",
                    "type": "deps_only",
                    "path": rerooted_path.to_string_lossy()
                })
            );
            return Ok(());
        }

        let compiled_package = config
            .clone()
            .compile_package(&rerooted_path, &mut Vec::new())?;

        // Enhanced metadata JSON output with detailed function info
        let result = json!({
            "status": "success",
            "type": "full_build",
            "metadata": {
                "package": {
                    "name": compiled_package.compiled_package_info.package_name.to_string(),
                    "path": rerooted_path.to_string_lossy(),
                    "info": {
                        "source_digest": compiled_package.compiled_package_info.source_digest,
                        "build_flags": compiled_package.compiled_package_info.build_flags,
                        "addresses": compiled_package.compiled_package_info.address_alias_instantiation
                            .iter()
                            .map(|(name, addr)| (name.to_string(), json!(format!("0x{}", addr.to_hex()))))
                            .collect::<serde_json::Map<String, JsonValue>>()
                    }
                },
                "modules": compiled_package.root_compiled_units
                    .iter()
                    .map(|unit| {
                        let module = &unit.unit;
                        json!({
                            "id": generate_object_id(),
                            "name": module.name().to_string(),
                            "source_path": unit.source_path.to_string_lossy(),
                            "content": {
                                "functions": module.module.function_defs()
                                    .iter()
                                    .map(|fdef| {
                                        let handle = module.module.function_handle_at(fdef.function);
                                        let name = module.module.identifier_at(handle.name);
                                        json!({
                                            "id": generate_object_id(),
                                            "name": name.to_string(),
                                            "metadata": {
                                                "visibility": format!("{:?}", fdef.visibility),
                                                "is_entry": fdef.is_entry,
                                                "handle_id": format!("0x{}", fdef.function.0)
                                            },
                                            "signature": {
                                                "parameters": module.module.signature_at(handle.parameters)
                                                    .0
                                                    .iter()
                                                    .map(|ty| format!("{:?}", ty))
                                                    .collect::<Vec<_>>(),
                                                "return_types": module.module.signature_at(handle.return_)
                                                    .0
                                                    .iter()
                                                    .map(|ty| format!("{:?}", ty))
                                                    .collect::<Vec<_>>()
                                            }
                                        })
                                    })
                                    .collect::<Vec<_>>(),
                                "structs": module.module.struct_defs()
                                    .iter()
                                    .map(|sdef| {
                                        let handle = module.module.struct_handle_at(sdef.struct_handle);
                                        let name = module.module.identifier_at(handle.name);
                                        json!({
                                            "id": generate_object_id(),
                                            "name": name.to_string(),
                                            "metadata": {
                                                "handle_id": format!("0x{}", sdef.struct_handle.0),
                                                "abilities": format!("{:?}", handle.abilities)
                                            },
                                            "fields": match &sdef.field_information {
                                                move_binary_format::file_format::StructFieldInformation::Native => Vec::new(),
                                                move_binary_format::file_format::StructFieldInformation::Declared(fields) => {
                                                    fields.iter()
                                                        .map(|field| json!({
                                                            "id": generate_object_id(),
                                                            "name": module.module.identifier_at(field.name).to_string(),
                                                            "type": format!("{:?}", field.signature.0)
                                                        }))
                                                        .collect::<Vec<_>>()
                                                }
                                            }
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            }
                        })
                    })
                    .collect::<Vec<_>>()
            }
        });

        println!("{}", serde_json::to_string_pretty(&result)?);
        Ok(())
    }
}
