use anyhow::Result;
use clap::Parser;
use move_package::{
    compilation::compiled_package::CompiledPackage,
    BuildConfig
};
use move_vm_test_utils::gas_schedule::CostTable;
use std::path::PathBuf;
use crate::sandbox::{
    utils::on_disk_state_view::OnDiskStateView,
    commands::publish::publish,
};
use crate::DEFAULT_STORAGE_DIR;

#[derive(Parser)]
pub struct Publish {
    /// Path to Move module
    #[clap(long)]
    pub module_path: PathBuf,

    /// Gas budget for deployment
    #[clap(long, default_value = "1000000")]
    pub gas_budget: u64,

    /// Module address
    #[clap(long)]
    pub address: Option<String>,

    /// Skip verification
    #[clap(long)]
    pub skip_verify: bool,
}

impl Publish {
    pub fn execute(
        self,
        path: Option<PathBuf>,
        config: BuildConfig,
        cost_table: &CostTable,
    ) -> Result<()> {
        let package_path = path.unwrap_or_else(|| self.module_path.clone());
        let storage_path = package_path.join(DEFAULT_STORAGE_DIR);
        
        let state = OnDiskStateView::create(&package_path, &storage_path)?;
        let package = compile_package(&package_path, config)?;

        publish(
            vec![], // natives
            cost_table,
            &state,
            &package,
            self.skip_verify,
            true,    // with_deps
            true,    // bundle
            None,    // override_ordering
            false,   // verbose
        )
    }
}

fn compile_package(
    path: &PathBuf,
    config: BuildConfig,
) -> Result<CompiledPackage> {
    let build_config = BuildConfig {
        install_dir: Some(path.clone()),
        additional_named_addresses: config.additional_named_addresses,
        ..BuildConfig::default()
    };

    build_config.compile_package(path, &mut Vec::new())
}