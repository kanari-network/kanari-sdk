// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl BlockchainEngine {
    pub fn new_dir(dir: &str) -> Result<Self> {
        let persistent_store = Self::try_open_store(
            || PersistentStore::open_with_path(Some(std::path::PathBuf::from(dir))),
            &format!("at '{}'", dir),
        )?;
        Self::init(persistent_store)
    }

    pub fn new() -> Result<Self> {
        let persistent_store = Self::try_open_store(PersistentStore::open_default, "default")?;
        Self::init(persistent_store)
    }

    pub fn new_in_memory() -> Result<Self> {
        Self::init(None)
    }

    fn try_open_store<F>(opener: F, context: &str) -> Result<Option<Arc<PersistentStore>>>
    where
        F: FnOnce() -> Result<PersistentStore>,
    {
        if cfg!(miri) {
            Ok(None)
        } else {
            match opener() {
                Ok(store) => Ok(Some(Arc::new(store))),
                Err(e) => {
                    if Self::strict_persistence_required() {
                        anyhow::bail!(
                            "Failed to open {} persistent store in {} mode: {}",
                            context,
                            Self::network_name(),
                            e
                        );
                    }
                    eprintln!(
                        "WARN: Failed to open {} persistent store: {}. Falling back to in-memory mode.",
                        context, e
                    );
                    Ok(None)
                }
            }
        }
    }

    fn init(persistent_store: Option<Arc<PersistentStore>>) -> Result<Self> {
        let mut blockchain = Self::load_blockchain(&persistent_store);
        let persisted_dag_state = Self::load_dag_state(&persistent_store);
        Self::repair_blockchain_from_dag_state(&mut blockchain, persisted_dag_state.as_ref())?;
        let state = Self::load_state(&persistent_store);

        let workers = num_cpus::get().max(1);
        let mut runtime_pool = Vec::new();

        let base_runtime = match if persistent_store.is_some() {
            MoveRuntime::new_with_kanari_natives()
        } else {
            MoveRuntime::new_with_kanari_natives_in_memory()
        } {
            Ok(rt) => rt,
            Err(e) => {
                log::error!("FATAL: Failed to initialize base MoveRuntime: {}", e);
                anyhow::bail!("Failed to initialize runtime pool: {}", e);
            }
        };

        log::info!(
            "Initializing runtime pool with {} workers (independent VMs sharing DB)",
            workers
        );
        runtime_pool.push(base_runtime.clone());

        for i in 1..workers {
            match base_runtime.spawn_worker() {
                Ok(rt) => runtime_pool.push(rt),
                Err(e) => {
                    log::error!("Failed to spawn worker runtime #{}: {}", i, e);
                    anyhow::bail!("Failed to initialize runtime pool: {}", e);
                }
            }
        }

        let pending_txs = Arc::new(RwLock::new(Vec::new()));
        let pending_tx_hashes = Arc::new(RwLock::new(HashSet::new()));
        let proof_cache = Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(1000).unwrap())));

        let authority_id = "0xDEFAULT_AUTHORITY".to_string();
        let authorities = vec![authority_id.clone()];

        Ok(Self {
            blockchain,
            state,
            pending_txs,
            pending_tx_hashes,
            persistent_store,
            runtime_pool,
            proof_cache,
            dag_engine: Arc::new(RwLock::new(None)),
            authority_id,
            authorities,
            persisted_dag_state,
        })
    }

    fn repair_blockchain_from_dag_state(
        blockchain: &mut Arc<RwLock<Blockchain>>,
        persisted_dag_state: Option<&PersistentDagState>,
    ) -> Result<()> {
        let Some(dag_state) = persisted_dag_state else {
            return Ok(());
        };

        let latest_dag_sequence = dag_state
            .checkpoints
            .last()
            .map(|checkpoint| checkpoint.sequence)
            .unwrap_or(0);

        let current_height = blockchain
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .height();
        if current_height > 0 || latest_dag_sequence == 0 {
            return Ok(());
        }

        let mut rebuilt = Blockchain::new();
        for checkpoint in dag_state.checkpoints.iter().skip(1) {
            rebuilt.add_checkpoint_with_validation(checkpoint.clone(), false)?;
        }
        rebuilt.rebuild_tx_hash_index();

        info!(
            "Recovered blockchain from persisted DAG state (height: {}, checkpoints: {})",
            rebuilt.height(),
            rebuilt.dag_checkpoints.len()
        );
        *blockchain = Arc::new(RwLock::new(rebuilt));
        Ok(())
    }

    fn load_blockchain(store: &Option<Arc<PersistentStore>>) -> Arc<RwLock<Blockchain>> {
        if let Some(store) = store {
            match store.load::<Blockchain>(b"blockchain") {
                Ok(Some(mut blockchain)) => {
                    info!(
                        "Successfully loaded blockchain from persistent store (height: {}, checkpoints: {})",
                        blockchain.height(),
                        blockchain.dag_checkpoints.len()
                    );
                    blockchain.rebuild_tx_hash_index();
                    Arc::new(RwLock::new(blockchain))
                }
                Ok(None) => {
                    info!("No persisted blockchain found. Creating fresh genesis.");
                    Arc::new(RwLock::new(Blockchain::new()))
                }
                Err(e) => {
                    error!(
                        "FATAL ERROR loading blockchain: {}. Falling back to fresh genesis.",
                        e
                    );
                    Arc::new(RwLock::new(Blockchain::new()))
                }
            }
        } else {
            info!("Running in-memory mode: No persistent store provided for blockchain.");
            Arc::new(RwLock::new(Blockchain::new()))
        }
    }

    fn load_state(store: &Option<Arc<PersistentStore>>) -> Arc<RwLock<StateManager>> {
        let store = store.clone().unwrap_or_else(|| {
            Arc::new(PersistentStore::open_in_memory().unwrap_or_else(|e| {
                error!("Failed to open in-memory store: {}. Using fallback.", e);
                panic!("Cannot initialize state manager without storage")
            }))
        });
        info!("Initializing StateManager with persistent store support (RocksDB)");
        Arc::new(RwLock::new(StateManager::new(store)))
    }

    fn load_dag_state(store: &Option<Arc<PersistentStore>>) -> Option<PersistentDagState> {
        if let Some(store) = store {
            match store.load::<PersistentDagState>(b"dag_state") {
                Ok(Some(state)) => {
                    info!("Successfully loaded DAG consensus state from persistent store");
                    Some(state)
                }
                Ok(None) => None,
                Err(e) => {
                    error!(
                        "Failed to load DAG state: {}. Falling back to fresh DAG.",
                        e
                    );
                    None
                }
            }
        } else {
            None
        }
    }
}
