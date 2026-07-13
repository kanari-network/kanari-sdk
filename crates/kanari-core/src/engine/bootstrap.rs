// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl BlockchainEngine {
    fn recover_checkpoint_sequences_from_store(store: &PersistentStore) -> Vec<u64> {
        let mut sequences = store
            .logical_entries()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(key, _)| {
                let key = String::from_utf8(key).ok()?;
                key.strip_prefix("checkpoint_meta/")
                    .or_else(|| key.strip_prefix("checkpoint_txs/"))?
                    .parse::<u64>()
                    .ok()
            })
            .collect::<Vec<_>>();
        sequences.sort_unstable();
        sequences.dedup();
        sequences
    }

    fn recover_blockchain_from_checkpoint_store(store: &PersistentStore) -> Option<Blockchain> {
        let checkpoint_sequences = Self::recover_checkpoint_sequences_from_store(store);

        if checkpoint_sequences.is_empty() {
            return None;
        }

        let mut blockchain = Blockchain::new();
        for sequence in checkpoint_sequences {
            if sequence == 0 {
                continue;
            }
            let mut checkpoint = match Self::load_checkpoint_metadata(store, sequence) {
                Some(checkpoint) => checkpoint,
                None => {
                    let transactions = Self::load_checkpoint_transactions(store, sequence)?;
                    let prev_hash = blockchain.latest_checkpoint().hash().ok()?;
                    Checkpoint::new(
                        sequence,
                        Vec::new(),
                        transactions,
                        Vec::new(),
                        blockchain.latest_checkpoint().timestamp.saturating_add(1),
                        prev_hash,
                    )
                }
            };
            if checkpoint.transactions.is_empty()
                && let Some(transactions) = Self::load_checkpoint_transactions(store, sequence)
            {
                checkpoint.transactions = transactions;
            }

            if let Err(error) = blockchain.add_checkpoint_with_validation(checkpoint, true) {
                tracing::error!(
                    checkpoint = sequence,
                    "Failed to recover checkpoint from metadata store: {}",
                    error
                );
                return None;
            }
        }

        blockchain.rebuild_tx_hash_index();
        Some(blockchain)
    }

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
                    tracing::warn!(
                        "Failed to open {} persistent store: {}. Falling back to in-memory mode.",
                        context,
                        e
                    );
                    Ok(None)
                }
            }
        }
    }

    fn init(persistent_store: Option<Arc<PersistentStore>>) -> Result<Self> {
        tracing::info!("Loading blockchain checkpoints");
        let blockchain = Self::load_blockchain(&persistent_store);
        tracing::info!("Opening state database");
        let state = Self::load_state(&persistent_store)?;
        let shared_runtime_store = {
            let state_guard = state.read().unwrap_or_else(|e| e.into_inner());
            state_guard.store()
        };

        let workers = Self::runtime_worker_count();
        let mut runtime_pool = Vec::new();

        let base_runtime =
            match MoveRuntime::new_with_kanari_natives_and_store(shared_runtime_store) {
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
        let verbose_startup = std::env::var("KANARI_VERBOSE_STARTUP")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false);
        tracing::info!(workers, "Move runtime pool initializing");
        runtime_pool.push(base_runtime.clone());

        for i in 1..workers {
            if verbose_startup {
                tracing::info!(worker = i + 1, workers, "Starting Move runtime worker");
            }
            match base_runtime.spawn_worker() {
                Ok(rt) => {
                    runtime_pool.push(rt);
                    if verbose_startup {
                        tracing::info!(worker = i + 1, workers, "Move runtime worker ready");
                    }
                }
                Err(e) => {
                    log::error!("Failed to spawn worker runtime #{}: {}", i, e);
                    anyhow::bail!("Failed to initialize runtime pool: {}", e);
                }
            }
        }
        tracing::info!(workers = runtime_pool.len(), "Move runtime pool ready");
        state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .repair_persisted_smt()
            .context("Failed to reconcile state SMT after runtime initialization")?;
        tracing::info!("Preparing mempool, proof cache, and authority defaults");

        let mempool = Arc::new(RwLock::new(MempoolState::default()));
        let proof_cache = Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(1000).unwrap())));

        let authority_id = "0xDEFAULT_AUTHORITY".to_string();
        let authorities = vec![authority_id.clone()];

        let engine = Self {
            blockchain,
            state,
            mempool,
            invalid_pending_drop_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            persistent_store,
            runtime_pool,
            proof_cache,
            dag_engine: Arc::new(RwLock::new(None)),
            authority_id,
            authorities,
            consensus_signing_key: None,
            consensus_public_keys: BTreeMap::new(),
        };

        let stats = engine.get_stats();
        tracing::info!(
            height = stats.height,
            txs = stats.total_transactions,
            owners = stats.total_owners,
            pending = stats.pending_transactions,
            "Kanari engine ready"
        );
        Ok(engine)
    }

    fn runtime_worker_count() -> usize {
        const DEFAULT_MAX_RUNTIME_WORKERS: usize = 4;

        if let Ok(raw) = std::env::var("KANARI_RUNTIME_WORKERS") {
            match raw.trim().parse::<usize>() {
                Ok(value) if value > 0 => return value,
                _ => tracing::warn!(
                    "Ignoring invalid KANARI_RUNTIME_WORKERS='{}'. Using default.",
                    raw
                ),
            }
        }

        num_cpus::get().clamp(1, DEFAULT_MAX_RUNTIME_WORKERS)
    }

    fn load_blockchain(store: &Option<Arc<PersistentStore>>) -> Arc<RwLock<Blockchain>> {
        if let Some(store) = store {
            match store.load::<Blockchain>(b"blockchain") {
                Ok(Some(mut blockchain)) => {
                    Self::hydrate_blockchain_transactions(store, &mut blockchain);
                    info!(
                        "Successfully loaded blockchain from persistent store (height: {}, checkpoints: {})",
                        blockchain.height(),
                        blockchain.dag_checkpoints.len()
                    );
                    blockchain.rebuild_tx_hash_index();
                    Arc::new(RwLock::new(blockchain))
                }
                Ok(None) => {
                    if let Some(blockchain) = Self::recover_blockchain_from_checkpoint_store(store)
                    {
                        if let Err(error) =
                            Self::persist_blockchain_snapshot_to_store(store, &blockchain)
                        {
                            tracing::warn!(
                                "Recovered blockchain but failed to repersist normalized snapshot: {}",
                                error
                            );
                        }
                        info!(
                            "Recovered blockchain from checkpoint metadata (height: {}, checkpoints: {})",
                            blockchain.height(),
                            blockchain.dag_checkpoints.len()
                        );
                        Arc::new(RwLock::new(blockchain))
                    } else {
                        info!("No persisted blockchain found. Creating fresh genesis.");
                        Arc::new(RwLock::new(Blockchain::new()))
                    }
                }
                Err(e) => {
                    error!("FATAL ERROR loading blockchain: {}", e);
                    if let Some(blockchain) = Self::recover_blockchain_from_checkpoint_store(store)
                    {
                        if let Err(error) =
                            Self::persist_blockchain_snapshot_to_store(store, &blockchain)
                        {
                            tracing::warn!(
                                "Recovered blockchain after load failure but failed to repersist normalized snapshot: {}",
                                error
                            );
                        }
                        info!(
                            "Recovered blockchain from checkpoint metadata after primary load failure (height: {}, checkpoints: {})",
                            blockchain.height(),
                            blockchain.dag_checkpoints.len()
                        );
                        Arc::new(RwLock::new(blockchain))
                    } else {
                        error!("Falling back to fresh genesis after blockchain recovery failure.");
                        Arc::new(RwLock::new(Blockchain::new()))
                    }
                }
            }
        } else {
            info!("Running in-memory mode: No persistent store provided for blockchain.");
            Arc::new(RwLock::new(Blockchain::new()))
        }
    }

    fn load_state(store: &Option<Arc<PersistentStore>>) -> Result<Arc<RwLock<StateManager>>> {
        let store = match store.clone() {
            Some(store) => store,
            None => Arc::new(PersistentStore::open_in_memory()?),
        };
        info!("Initializing StateManager with persistent store support (RocksDB)");
        Ok(Arc::new(RwLock::new(StateManager::try_new(store)?)))
    }
}
