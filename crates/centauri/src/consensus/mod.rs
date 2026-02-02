// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Core DAG consensus components
mod dag_consensus;
pub use dag_consensus::{
    AuthorityId, Checkpoint, DagConsensus, DagStore, DagVertex, PersistentDagState, Round,
    VertexId, VertexMetadata,
};

// Cryptographic primitives
mod crypto_signatures;
pub use crypto_signatures::{Ed25519Keypair, SignatureScheme};

mod ecvrf;
pub use ecvrf::{VrfProof, VrfPublicKey, VrfSecretKey};

// VRF-based leader election (uses ecvrf internally)
mod vrf_leader;
pub use vrf_leader::{VrfLeaderElection, VrfOutput};

// Consensus subsystems
mod metrics;
pub use metrics::{DagMetrics, Histogram};

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
    AdaptiveBatchConfig, CompressedBatch, DeltaSync, VertexBatch, VertexBloomFilter,
    VertexBroadcaster,
};

// State synchronization
mod state_sync;
pub use state_sync::{FastSync, StateSynchronizer, SyncProgress, SyncRequest, SyncResponse};

// Light client functionality
mod light_client;
pub use light_client::{
    CheckpointBuilder, CheckpointSignature, LightCheckpoint, LightClient, LightClientQuery,
    StateProof, TransactionProof,
};

// Committee management
mod committee;
pub use committee::{
    Committee, CommitteeChange, CommitteeChangeTx, CommitteeManager, ValidatorInfo,
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
