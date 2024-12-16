use k2::block::Block;
use consensus_pos::Blake3Algorithm;
use k2::transaction::{Transaction, TransactionType};
use kari_move::sandbox::commands::publish;
use kari_move::sandbox::utils::on_disk_state_view::OnDiskStateView;
use kari_move::DEFAULT_STORAGE_DIR;
use move_package::compilation::compiled_package::CompiledPackage;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::task;
use anyhow::Result;
use move_package::BuildConfig;
use move_vm_test_utils::gas_schedule::zero_cost_schedule;
use std::path::PathBuf;


pub struct Blockchain {
    pub blocks: Arc<Mutex<VecDeque<Block<Blake3Algorithm>>>>,
    pub balances: Arc<Mutex<HashMap<String, u64>>>,
    pub move_modules: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Blockchain {
    pub fn new() -> Self {
        Blockchain {
            blocks: Arc::new(Mutex::new(VecDeque::new())),
            balances: Arc::new(Mutex::new(HashMap::new())),
            move_modules: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn process_transactions(&self, transactions: Vec<Transaction>) {
        let mut handles = vec![];

        for tx in transactions {
            let blocks = Arc::clone(&self.blocks);
            let balances = Arc::clone(&self.balances);
            let move_modules = Arc::clone(&self.move_modules);

            let handle = task::spawn(async move {
                match tx.tx_type {
                    TransactionType::MoveModuleDeploy(module_bytes) => {
                        // Handle Move module deployment
                        let package_path = PathBuf::from("path/to/package");
                        let config = BuildConfig::default();
                        let cost_table = zero_cost_schedule(); // Initialize CostTable correctly
                        let storage_path = package_path.join(DEFAULT_STORAGE_DIR);
                        let state = OnDiskStateView::create(&package_path, &storage_path).unwrap();
                        let package = compile_package(&package_path, config).unwrap();

                        publish(
                            vec![],
                            &cost_table,
                            &state,
                            &package,
                            false,
                            true,
                            true,
                            None,
                            false,
                        ).unwrap();

                        let mut move_modules = move_modules.lock().unwrap();
                        move_modules.insert(tx.sender.clone(), module_bytes);
                    }
                    _ => {
                        // Handle other transaction types
                        {
                            let mut blocks = blocks.lock().unwrap();
                            // Add transaction to the latest block or create a new block
                            // Update block hash, etc.
                        }

                        {
                            let mut balances = balances.lock().unwrap();
                            // Update balances based on the transaction
                        }
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            if let Err(e) = handle.await {
                eprintln!("Error processing transaction: {:?}", e);
            }
        }
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

pub struct Shard {
    pub blockchain: Blockchain,
}

impl Shard {
    pub fn new() -> Self {
        Shard {
            blockchain: Blockchain::new(),
        }
    }

    pub async fn process_shard_transactions(&self, transactions: Vec<Transaction>) {
        self.blockchain.process_transactions(transactions).await;
    }
}