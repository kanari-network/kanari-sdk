// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::reroot_path;
use crate::command::common::{
    build_blocking_client, get_rpc_endpoint, get_sender_for_tx, load_wallet_for, normalize_addr,
    resolve_sender, resolve_transaction_gas,
};
use crate::command::rpc_helpers::{
    render_transaction_submission, require_rpc_result, sign_publish_package_request,
    submit_blocking_rpc,
};
use anyhow::{Context, Result, bail};
use clap::*;
use kanari_rpc_api::{
    BuildPublishPackageRequest, PublishPackageModule, PublishPackageRequest, methods,
};
use kanari_types::{GasEstimate, GasOperation};
use move_package::BuildConfig;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

const MAX_PUBLISH_GAS_RETRIES: usize = 60;
const PUBLISH_GAS_RETRY_DELAY_MS: u64 = 250;

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
    fn is_publish_gas_temporarily_unavailable(error: &anyhow::Error) -> bool {
        let message = format!("{:#}", error);
        message.contains("No spendable native gas coin object found with required balance")
            || message.contains("No spendable native gas coin object found")
    }

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

        let mut skipped_count = 0;
        let mut package_modules = Vec::new();
        let mut package_function_count = 0usize;

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
            eprintln!(
                "     Estimated contribution: {} units ({} Mist / {:.9} KANARI)",
                estimate.gas_units, estimate.total_cost_mist, estimate.total_cost_kanari
            );
            package_function_count += module.function_defs.len();
            package_modules.push(PublishPackageModule {
                module_name,
                module_bytes: module_bytecode,
            });
        }

        if package_modules.is_empty() {
            eprintln!("Warning: No modules were published. All modules were skipped or rejected.");
            eprintln!("Package build and validation complete!");
            eprintln!("   Published: 0 modules");
            eprintln!("   Skipped: {} dependency modules", skipped_count);
            return Ok(());
        }

        eprintln!("   Package modules: {}", package_modules.len());
        eprintln!("   Package functions: {}", package_function_count);
        eprintln!("   Limit: {} units", gas_limit);
        let total_estimate = GasEstimate::from_operation(
            GasOperation::PublishModule {
                module_size: package_modules
                    .iter()
                    .map(|module| module.module_bytes.len())
                    .sum(),
            },
            gas_price,
        );
        eprintln!(
            "   Total Cost: {} Mist ({:.9} KANARI)",
            total_estimate.total_cost_mist, total_estimate.total_cost_kanari
        );

        let prepared: PublishPackageRequest = {
            let mut retry_count = 0usize;
            loop {
                let attempt = || -> Result<PublishPackageRequest> {
                    let prepared = submit_blocking_rpc(
                        &client,
                        &rpc,
                        methods::BUILD_PUBLISH_PACKAGE,
                        serde_json::to_value(BuildPublishPackageRequest {
                            sender: sender_for_tx.clone(),
                            modules: package_modules.clone(),
                            gas_limit,
                            gas_price,
                            nonce: None,
                            execute_immediate: Some(true),
                        })
                        .context("Failed to serialize build publish package request")?,
                    )?;
                    serde_json::from_value(require_rpc_result(
                        prepared,
                        "RPC did not return prepared publish package request",
                    )?)
                    .context("Failed to decode prepared publish package request")
                };

                match attempt() {
                    Ok(prepared) => break prepared,
                    Err(error)
                        if Self::is_publish_gas_temporarily_unavailable(&error)
                            && retry_count < MAX_PUBLISH_GAS_RETRIES =>
                    {
                        retry_count += 1;
                        if retry_count == 1 || retry_count.is_multiple_of(10) {
                            eprintln!(
                                "     Waiting for spendable gas coin to become available (retry {}/{})...",
                                retry_count, MAX_PUBLISH_GAS_RETRIES
                            );
                        }
                        sleep(Duration::from_millis(PUBLISH_GAS_RETRY_DELAY_MS));
                    }
                    Err(error) => return Err(error),
                }
            }
        };

        if let Some(gas_payment) = &prepared.gas_payment
            && let Some(gas_object) = gas_payment.payment_objects.first()
        {
            eprintln!("   Gas payment object: {}", gas_object.object_id);
        }

        let request = sign_publish_package_request(prepared, &wallet)?;
        eprintln!("     Sending publish package RPC to {}...", rpc);
        let rpc_response = submit_blocking_rpc(
            &client,
            &rpc,
            methods::PUBLISH_PACKAGE,
            serde_json::to_value(request)?,
        )?;
        let published = render_transaction_submission(&client, &rpc, rpc_response, "     ", false)?;

        eprintln!("Package build and validation complete!");
        eprintln!(
            "   Published: {} package transaction ({} modules)",
            usize::from(published),
            package_modules.len()
        );
        eprintln!("   Skipped: {} dependency modules", skipped_count);
        Ok(())
    }
}
