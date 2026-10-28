// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::reroot_path;
use crate::command::common::{
    build_blocking_client, get_rpc_endpoint, get_sender_for_tx, load_wallet_for, normalize_addr,
    resolve_sender, resolve_transaction_gas,
};
use crate::command::rpc_helpers::{
    render_transaction_submission, require_rpc_result, sign_publish_package_request_as,
    submit_blocking_rpc,
};
use anyhow::{Context, Result, bail};
use clap::*;
use kanari_rpc_api::{
    BuildPublishPackageRequest, PublishPackageModule, PublishPackageRequest, methods,
};
use kanari_types::{GasEstimate, GasOperation};
use move_package::{BuildConfig, compilation::compiled_package::CompiledUnitWithSource};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

/// Maximum number of retries when the server reports that the gas coin is not
/// yet spendable (e.g. still locked by a previous transaction).
const MAX_PUBLISH_GAS_RETRIES: usize = 60;

/// Delay between retries. 60 * 250ms = 15s total wait window, which covers
/// the typical block confirmation time on Kanari.
const PUBLISH_GAS_RETRY_DELAY_MS: u64 = 250;

/// How often to log progress during retries (avoids spamming stderr).
const RETRY_LOG_INTERVAL: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishOperation {
    Publish,
    Upgrade,
}

impl PublishOperation {
    fn verb(self) -> &'static str {
        match self {
            Self::Publish => "Publishing",
            Self::Upgrade => "Upgrading",
        }
    }

    fn past_tense(self) -> &'static str {
        match self {
            Self::Publish => "Published",
            Self::Upgrade => "Upgraded",
        }
    }

    fn past_tense_lower(self) -> &'static str {
        match self {
            Self::Publish => "published",
            Self::Upgrade => "upgraded",
        }
    }

    fn build_method(self) -> &'static str {
        match self {
            Self::Publish => methods::BUILD_PUBLISH_PACKAGE,
            Self::Upgrade => methods::BUILD_UPGRADE_PACKAGE,
        }
    }

    fn submit_method(self) -> &'static str {
        match self {
            Self::Publish => methods::PUBLISH_PACKAGE,
            Self::Upgrade => methods::UPGRADE_PACKAGE,
        }
    }
}

/// Publish or upgrade Move modules on chain.
///
/// Re-running this command for an already-published module id performs a
/// compatibility-checked upgrade. Existing module storage is preserved when the
/// upgrade is rejected.
#[derive(Parser, Clone)]
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
    /// Detect whether the error is a transient "gas coin not yet spendable"
    /// condition that warrants a retry.
    ///
    /// Uses substring matching on the error message. If the server provides
    /// structured error codes in the future, prefer checking those instead.
    fn is_gas_temporarily_unavailable(error: &anyhow::Error) -> bool {
        let message = format!("{:#}", error);
        message.contains("No spendable native gas coin object found")
    }

    pub fn execute(self, path: Option<PathBuf>, config: BuildConfig) -> Result<()> {
        self.execute_as(path, config, PublishOperation::Publish)
    }

    pub fn execute_as(
        self,
        path: Option<PathBuf>,
        config: BuildConfig,
        operation: PublishOperation,
    ) -> Result<()> {
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
        let wallet = load_wallet_for(&sender_normalized, None)?;
        eprintln!(
            "   Wallet loaded: {} (curve: {})",
            sender_normalized, wallet.curve_type
        );
        let sender_for_tx = get_sender_for_tx(&wallet, &sender_normalized)?;

        // Filter modules whose self-address matches the sender and serialize them.
        let package_modules = Self::collect_modules_to_publish(&modules, &sender_normalized)?;
        let skipped_count = modules.len() - package_modules.len();

        if package_modules.is_empty() {
            eprintln!(
                "Warning: No modules were {}. All modules were skipped or rejected.",
                operation.past_tense_lower()
            );
            eprintln!("Package build and validation complete!");
            eprintln!("   {}: 0 modules", operation.past_tense());
            eprintln!("   Skipped: {} dependency modules", skipped_count);
            return Ok(());
        }

        Self::print_module_summary(&package_modules, gas_price, operation)?;

        eprintln!("{} modules on chain...", operation.verb());
        eprintln!("   RPC: {}", rpc);
        eprintln!("   Sender: {}", sender_for_tx);
        eprintln!("   Package modules: {}", package_modules.len());
        eprintln!("   Limit: {} units", gas_limit);

        let total_size: usize = package_modules.iter().map(|m| m.module_bytes.len()).sum();
        let total_estimate = GasEstimate::from_operation(
            GasOperation::PublishModule {
                module_size: total_size,
            },
            gas_price,
        );
        eprintln!(
            "   Total Cost: {} Mist ({:.9} KANARI)",
            total_estimate.total_cost_mist, total_estimate.total_cost_kanari
        );

        // Build the transaction, retrying if the gas coin is not yet spendable.
        let prepared = Self::build_with_retry(
            &client,
            &rpc,
            operation,
            sender_for_tx.clone(),
            package_modules.clone(),
            gas_limit,
            gas_price,
        )?;

        if let Some(gas_payment) = &prepared.gas_payment
            && let Some(gas_object) = gas_payment.payment_objects.first()
        {
            eprintln!("   Gas payment object: {}", gas_object.object_id);
        }

        let is_upgrade = operation == PublishOperation::Upgrade;
        let request = sign_publish_package_request_as(prepared, &wallet, is_upgrade)?;

        eprintln!(
            "     Sending {} package RPC to {}...",
            operation.past_tense_lower(),
            rpc
        );
        let rpc_response = submit_blocking_rpc(
            &client,
            &rpc,
            operation.submit_method(),
            serde_json::to_value(request)?,
        )?;
        let published = render_transaction_submission(&client, &rpc, rpc_response, "     ", false)?;

        eprintln!("Package build and validation complete!");
        eprintln!(
            "   {}: {} package transaction ({} modules)",
            operation.past_tense(),
            usize::from(published),
            package_modules.len()
        );
        eprintln!("   Skipped: {} dependency modules", skipped_count);
        Ok(())
    }

    /// Collect modules whose self-address matches the sender, serializing each
    /// to bytecode. Dependency modules (address != sender) are silently skipped.
    fn collect_modules_to_publish(
        modules: &[&CompiledUnitWithSource],
        sender_normalized: &str,
    ) -> Result<Vec<PublishPackageModule>> {
        let mut out = Vec::new();
        for module_unit in modules {
            let module = &module_unit.unit.module;
            let module_address = module.self_id().address().to_string();
            let module_addr_normalized =
                normalize_addr(&module_address).unwrap_or(module_address.to_owned());
            if !module_addr_normalized.eq_ignore_ascii_case(sender_normalized) {
                continue;
            }
            let mut module_bytecode = vec![];
            module.serialize(&mut module_bytecode)?;
            out.push(PublishPackageModule {
                module_name: module.self_id().name().to_string(),
                module_bytes: module_bytecode,
            });
        }
        Ok(out)
    }

    /// Print a per-module summary (size, address, function count, gas estimate).
    fn print_module_summary(
        package_modules: &[PublishPackageModule],
        gas_price: u64,
        operation: PublishOperation,
    ) -> Result<()> {
        let total_function_count = 0usize;
        for m in package_modules {
            let estimate = GasEstimate::from_operation(
                GasOperation::PublishModule {
                    module_size: m.module_bytes.len(),
                },
                gas_price,
            );
            eprintln!("   Module: {}", m.module_name);
            eprintln!("     Size: {} bytes", m.module_bytes.len());
            eprintln!(
                "     Estimated contribution: {} units ({} Mist / {:.9} KANARI)",
                estimate.gas_units, estimate.total_cost_mist, estimate.total_cost_kanari
            );
        }
        let _ = total_function_count;
        let _ = operation;
        Ok(())
    }

    /// Build the publish/upgrade request, retrying up to `MAX_PUBLISH_GAS_RETRIES`
    /// times if the server reports the gas coin is not yet spendable.
    fn build_with_retry(
        client: &reqwest::blocking::Client,
        rpc: &str,
        operation: PublishOperation,
        sender_for_tx: String,
        package_modules: Vec<PublishPackageModule>,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<PublishPackageRequest> {
        let mut retry_count = 0usize;
        loop {
            let attempt = || -> Result<PublishPackageRequest> {
                let prepared = submit_blocking_rpc(
                    client,
                    rpc,
                    operation.build_method(),
                    serde_json::to_value(BuildPublishPackageRequest {
                        sender: sender_for_tx.clone(),
                        modules: package_modules.clone(),
                        gas_limit,
                        gas_price,
                        nonce: None,
                        execute_immediate: Some(true),
                    })
                    .with_context(|| {
                        format!(
                            "Failed to serialize build {} package request",
                            operation.past_tense_lower()
                        )
                    })?,
                )?;
                serde_json::from_value(require_rpc_result(
                    prepared,
                    &format!(
                        "RPC did not return prepared {} package request",
                        operation.past_tense_lower()
                    ),
                )?)
                .with_context(|| {
                    format!(
                        "Failed to decode prepared {} package request",
                        operation.past_tense_lower()
                    )
                })
            };

            match attempt() {
                Ok(prepared) => break Ok(prepared),
                Err(error)
                    if Self::is_gas_temporarily_unavailable(&error)
                        && retry_count < MAX_PUBLISH_GAS_RETRIES =>
                {
                    retry_count += 1;
                    if retry_count == 1 || retry_count % RETRY_LOG_INTERVAL == 0 {
                        eprintln!(
                            "     Waiting for spendable gas coin to become available (retry {}/{})...",
                            retry_count, MAX_PUBLISH_GAS_RETRIES
                        );
                    }
                    sleep(Duration::from_millis(PUBLISH_GAS_RETRY_DELAY_MS));
                }
                Err(error) => break Err(error),
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_operation_verb_and_tense() {
        assert_eq!(PublishOperation::Publish.verb(), "Publishing");
        assert_eq!(PublishOperation::Upgrade.verb(), "Upgrading");
        assert_eq!(PublishOperation::Publish.past_tense(), "Published");
        assert_eq!(PublishOperation::Upgrade.past_tense(), "Upgraded");
        assert_eq!(PublishOperation::Publish.past_tense_lower(), "published");
        assert_eq!(PublishOperation::Upgrade.past_tense_lower(), "upgraded");
    }

    #[test]
    fn publish_operation_methods() {
        assert_eq!(
            PublishOperation::Publish.build_method(),
            methods::BUILD_PUBLISH_PACKAGE
        );
        assert_eq!(
            PublishOperation::Upgrade.build_method(),
            methods::BUILD_UPGRADE_PACKAGE
        );
        assert_eq!(
            PublishOperation::Publish.submit_method(),
            methods::PUBLISH_PACKAGE
        );
        assert_eq!(
            PublishOperation::Upgrade.submit_method(),
            methods::UPGRADE_PACKAGE
        );
    }

    #[test]
    fn gas_unavailable_detection() {
        let err =
            anyhow::anyhow!("No spendable native gas coin object found with required balance");
        assert!(Publish::is_gas_temporarily_unavailable(&err));

        let err = anyhow::anyhow!("Some other error");
        assert!(!Publish::is_gas_temporarily_unavailable(&err));
    }
}
