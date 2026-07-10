// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::reroot_path;
use crate::command::common::{
    build_blocking_client, get_rpc_endpoint, get_sender_for_tx, load_wallet_for, normalize_addr,
    resolve_sender, resolve_transaction_gas,
};
use crate::command::rpc_helpers::{
    render_transaction_submission, sign_publish_module_request, submit_blocking_rpc,
};
use anyhow::{Context, Result, bail};
use clap::*;
use kanari_rpc_api::{BuildPublishModuleRequest, PublishModuleRequest, methods};
use kanari_types::{GasEstimate, GasOperation};
use move_package::BuildConfig;
use std::path::PathBuf;

/// Publish the Move module to the blockchain
#[derive(Parser, Clone)]
#[clap(name = "publish")]
pub struct Publish {
    #[clap(flatten)]
    pub build_config: BuildConfig,
    #[clap(long = "package-path")]
    pub package_path: Option<PathBuf>,
    #[clap(long = "gas-limit")]
    pub gas_limit: Option<u64>,
    #[clap(long = "gas-price")]
    pub gas_price: Option<u64>,
    #[clap(long = "rpc")]
    pub rpc_endpoint: Option<String>,
}

impl Publish {
    pub fn execute(self, path: Option<PathBuf>, config: BuildConfig) -> Result<()> {
        let rerooted_path = reroot_path(path.or(self.package_path.clone()))?;
        let (gas_limit, gas_price) = resolve_transaction_gas(self.gas_limit, self.gas_price);
        let rpc = get_rpc_endpoint(self.rpc_endpoint.clone());
        let sender_normalized = resolve_sender(None)?;

        eprintln!("Building Move package...");
        let compiled_package = config.compile_package(&rerooted_path, &mut std::io::stderr())?;
        let modules: Vec<_> = compiled_package.all_modules().collect();
        if modules.is_empty() {
            bail!("No modules found in package");
        }

        eprintln!("Package compiled successfully!");
        eprintln!("   Modules: {}", modules.len());

        let client = build_blocking_client(30)?;
        let modules_to_publish: Vec<_> = modules
            .iter()
            .filter(|module_unit| {
                let module_address = module_unit.unit.module.self_id().address().to_string();
                let module_addr_normalized =
                    normalize_addr(&module_address).unwrap_or(module_address);
                module_addr_normalized.eq_ignore_ascii_case(&sender_normalized)
            })
            .collect();

        let total_estimated_gas =
            modules_to_publish
                .iter()
                .try_fold(0u64, |acc, module_unit| -> Result<u64> {
                    let mut bytes = vec![];
                    module_unit.unit.module.serialize(&mut bytes)?;
                    let estimate = GasEstimate::from_operation(
                        GasOperation::PublishModule {
                            module_size: bytes.len(),
                        },
                        gas_price,
                    );
                    Ok(acc.saturating_add(estimate.gas_units))
                })?;
        if modules_to_publish.len() > 1 {
            eprintln!(
                "Total estimated gas for all modules: {}",
                total_estimated_gas
            );
        }

        let wallet = load_wallet_for(&sender_normalized, None)?;
        eprintln!(
            "   Wallet loaded: {} (curve: {})",
            sender_normalized, wallet.curve_type
        );
        let sender_for_tx = get_sender_for_tx(&wallet, &sender_normalized)?;

        eprintln!("Publishing modules to blockchain...");
        eprintln!("   RPC: {}", rpc);
        eprintln!("   Sender: {}", sender_for_tx);

        let mut published_count = 0;
        let mut skipped_count = 0;

        for module_unit in &modules {
            let module = &module_unit.unit.module;
            let module_name = module.self_id().name().to_string();
            let module_address = module.self_id().address().to_string();
            let module_addr_normalized = normalize_addr(&module_address).unwrap_or(module_address);

            if !module_addr_normalized.eq_ignore_ascii_case(&sender_normalized) {
                skipped_count += 1;
                continue;
            }

            let module_bytecode = {
                let mut bytes = vec![];
                module.serialize(&mut bytes)?;
                bytes
            };
            let estimate = GasEstimate::from_operation(
                GasOperation::PublishModule {
                    module_size: module_bytecode.len(),
                },
                gas_price,
            );

            eprintln!("   Module: {}", module_name);
            eprintln!("     Size: {} bytes", module_bytecode.len());
            eprintln!("     Address: {}", module.self_id().address());
            eprintln!("     Functions: {}", module.function_defs.len());
            eprintln!("   Estimated: {} units", estimate.gas_units);
            eprintln!("   Limit: {} units", gas_limit);
            eprintln!(
                "   Total Cost: {} Mist ({:.9} KANARI)",
                estimate.total_cost_mist, estimate.total_cost_kanari
            );

            let prepared = submit_blocking_rpc(
                &client,
                &rpc,
                methods::BUILD_PUBLISH_MODULE,
                serde_json::to_value(BuildPublishModuleRequest {
                    sender: sender_for_tx.clone(),
                    module_bytes: module_bytecode,
                    module_name,
                    gas_limit,
                    gas_price,
                    execute_immediate: None,
                })
                .context("Failed to serialize build publish request")?,
            )?;
            let prepared: PublishModuleRequest = serde_json::from_value(
                prepared
                    .result
                    .context("RPC did not return prepared publish request")?,
            )
            .context("Failed to decode prepared publish request")?;

            if let Some(gas_payment) = &prepared.gas_payment
                && let Some(gas_object) = gas_payment.payment_objects.first()
            {
                eprintln!("   Gas payment object: {}", gas_object.object_id);
            }

            let request = sign_publish_module_request(prepared, &wallet)?;
            eprintln!("     Sending publish RPC to {}...", rpc);
            let rpc_response = submit_blocking_rpc(
                &client,
                &rpc,
                methods::PUBLISH_MODULE,
                serde_json::to_value(request)?,
            )?;
            if render_transaction_submission(&client, &rpc, rpc_response, "     ", false)? {
                published_count += 1;
            }
        }

        if published_count == 0 {
            eprintln!("Warning: No modules were published. All modules were skipped or rejected.");
        }

        eprintln!("Package build and validation complete!");
        eprintln!("   Published: {} modules", published_count);
        eprintln!("   Skipped: {} dependency modules", skipped_count);
        Ok(())
    }
}
