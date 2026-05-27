// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Core DAG consensus components
mod dag_consensus;
pub use dag_consensus::{
    AuthorityId, Checkpoint, CheckpointStats, DagConsensus, DagExecutionPlan,
    DagNetworkVertexAction, DagPendingSelection, DagProductionPlan, DagProductionPolicy,
    DagProgressPolicy, DagStore, DagVertex, PersistentDagState, Round, VertexId, VertexMetadata,
};

// Cryptographic primitives
mod crypto_signatures;
pub use crypto_signatures::{Ed25519Keypair, SignatureScheme};

// Consensus subsystems
mod protocol;
pub use protocol::{ConsensusProtocol, Protocol};

mod metrics;
pub use metrics::DagMetrics;

// Caching utilities
mod cache;
pub use cache::{CacheStats, DagCacheStats, DagCaches, LruCache};

// Byzantine fault detection and slashing
mod byzantine_detector;
pub use byzantine_detector::{
    ByzantineDetector, ByzantineEvidence, ByzantineFault, SlashingPenalty,
};

// Vertex broadcast protocol
mod vertex_broadcast;
pub use vertex_broadcast::{
    AdaptiveBatchConfig, CompressedBatch, VertexBatch, VertexBloomFilter, VertexBroadcaster,
};

// State synchronization
mod state_sync;
pub use state_sync::{StateSynchronizer, SyncProgress, SyncRequest, SyncResponse};

// Committee membership and quorum model
mod committee;
pub use committee::{
    AdaptiveQuorum, AdaptiveQuorumConfig, Committee, NetworkHealth, ValidatorInfo,
};

// Cross-shard DAG communication
mod sharding;
pub use sharding::{
    AtomicCommitPhase, AtomicCommitPlan, AtomicCommitVote, CrossShardDispatch, CrossShardMessage,
    CrossShardProof, CrossShardQueue, ShardId, ShardedDag,
};

// Persistent storage for DAG consensus
mod persistent_store;
pub use persistent_store::{PersistentDagStore, StorageStats};

// DAG pruning utilities
mod pruning;
pub use pruning::{DagPruner, PruneStats, PruningConfig, PruningPolicy};

// Parallel transaction validator
mod parallel_validator;
pub use parallel_validator::{
    ParallelValidator, ParallelValidatorConfig, ValidationResult, ValidationStats,
};
