# DAG Consensus Architecture Diagram

## System Overview

```rust
┌─────────────────────────────────────────────────────────────────┐
│                        Kanari Blockchain                        │
│                                                                 │
│  ┌──────────────┐                      ┌──────────────┐         │
│  │ Linear Chain │  ◄────switch────►    │   DAG Mode   │         │
│  │    Mode      │                      │              │         │
│  └──────────────┘                      └──────────────┘         │
│         │                                      │                │
│         │                                      │                │
│         ▼                                      ▼                │
│  ┌──────────────┐                      ┌──────────────┐         │
│  │   Blocks     │                      │  Checkpoints │         │
│  │  (Sequential)│                      │  (from DAG)  │         │
│  └──────────────┘                      └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

## DAG Consensus Layers

```rust
┌──────────────────────────────────────────────────────────────────┐
│                     Application Layer                            │
│  (Move VM, Smart Contracts, Transactions)                        │
└────────────────┬─────────────────────────────────────────────────┘
                 │
┌────────────────▼─────────────────────────────────────────────────┐
│                   DAG Execution Layer                            │
│                                                                  │
│  ┌───────────────────────────────────────────────────────┐       │
│  │              DagEngine                                │       │
│  │  • produce_vertex()                                   │       │
│  │  • Parallel transaction execution                     │       │
│  │  • State management                                   │       │
│  └───────────────────┬───────────────────────────────────┘       │
└──────────────────────┼───────────────────────────────────────────┘
                       │
┌──────────────────────▼───────────────────────────────────────────┐
│                 Consensus Layer (Bullshark)                      │
│                                                                  │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐                  │
│  │  Ordering  │  │  Quorum    │  │  Leader    │                  │
│  │  Protocol  │  │  Check     │  │  Election  │                  │
│  │  (3 rounds)│  │  (2f+1)    │  │  (Round-   │                  │
│  │            │  │            │  │   Robin)   │                  │
│  └────────────┘  └────────────┘  └────────────┘                  │
│                                                                  │
│  ┌────────────────────────────────────────────────────┐          │
│  │         DagConsensus                               │          │
│  │  • create_vertex()                                 │          │
│  │  • add_vertex()                                    │          │
│  │  • try_commit() → Checkpoint                       │          │
│  └────────────────────┬───────────────────────────────┘          │
└───────────────────────┼──────────────────────────────────────────┘
                        │
┌───────────────────────▼──────────────────────────────────────────┐
│              Data Availability Layer                             │
│                                                                  │
│  ┌────────────────────────────────────────────────────┐          │
│  │              DagStore                              │          │
│  │  • Store vertices (HashMap<VertexId, DagVertex>)   │          │
│  │  • Index by round                                  │          │
│  │  • Index by authority                              │          │
│  │  • Maintain pending vertices queue                 │          │
│  └────────────────────────────────────────────────────┘          │
└──────────────────────────────────────────────────────────────────┘
```

## DAG Structure Visualization

### Round-based Vertex Creation

```rust
Time flows downward ↓

Round 0 (Genesis):
┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐
│ V0  │  │ V1  │  │ V2  │  │ V3  │
│Auth1│  │Auth2│  │Auth3│  │Auth4│
└──┬──┘  └──┬──┘  └──┬──┘  └──┬──┘
   │        │        │        │
   └────────┴────────┴────────┘
            │
            ▼
Round 1:
┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐
│ V4  │  │ V5  │  │ V6  │  │ V7  │  (Each references 2f+1 parents)
│Auth1│  │Auth2│  │Auth3│  │Auth4│
└──┬──┘  └──┬──┘  └──┬──┘  └──┬──┘
   │        │        │        │
   └────────┴────────┴────────┘
            │
            ▼
Round 2:
┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐
│ V8  │  │ V9  │  │V10  │  │V11  │
│Auth1│  │Auth2│  │Auth3│  │Auth4│
└──┬──┘  └──┬──┘  └──┬──┘  └──┬──┘
   │        │        │        │
   └────────┴────────┴────────┘
            │
            ▼
Round 3:
┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐
│V12  │  │V13  │  │V14  │  │V15  │
│Auth1│  │Auth2│  │Auth3│  │Auth4│
└─────┘  └─────┘  └─────┘  └─────┘
         
         ⭐ Checkpoint #1 commits Round 1 vertices!
```

## Parallel Execution Model

```rust
┌───────────────────────────────────────────────────────────┐
│                    Transaction Pool                       │
│  [Tx1 Tx2 Tx3 Tx4 Tx5 Tx6 Tx7 Tx8 Tx9 Tx10]               │
└─────────┬─────────────────────────────────────────────────┘
          │
          │ Group by sender
          ▼
┌─────────────────────────────────────────────────────────────┐
│  Sender A: [Tx1, Tx4, Tx7]  (sequential execution)          │
│  Sender B: [Tx2, Tx5, Tx8]  (sequential execution)          │
│  Sender C: [Tx3, Tx6, Tx9]  (sequential execution)          │
│  Sender D: [Tx10]           (sequential execution)          │
└────┬──────────┬──────────┬───────────┬──────────────────────┘
     │          │          │           │
     │          │          │           │
┌────▼────┐ ┌───▼─────┐ ┌──▼──────┐ ┌──▼──────┐
│Worker 1 │ │Worker 2 │ │Worker 3 │ │Worker 4 │  (Parallel)
│Runtime 1│ │Runtime 2│ │Runtime 3│ │Runtime 4│
└────┬────┘ └───┬─────┘ └──┬──────┘ └──┬──────┘
     │          │          │           │
     └──────────┴──────────┴───────────┘
                │
                ▼
         ┌──────────────┐
         │  ChangeSet   │
         │  Aggregation │
         └──────┬───────┘
                │
                ▼
         ┌──────────────┐
         │ Apply to     │
         │ State        │
         └──────────────┘
```

## Commit Protocol (Bullshark)

```rust
Round N:
  Leader creates vertex VL
    │
    ▼
Round N+1:
  Other authorities create vertices that reference VL
    │
    │  Check: Do 2f+1 vertices reference VL?
    │
    ├─ YES ─► Continue to Round N+2
    │
    └─ NO ──► No commit yet, continue
    
Round N+2:
  Vertices created
    │
    │  Final check: Does VL have 2f+1 support?
    │
    ├─ YES ─► COMMIT!
    │         │
    │         ▼
    │   ┌──────────────────┐
    │   │ Create Checkpoint│
    │   │ - Collect all    │
    │   │   vertices to VL │
    │   │ - Order txs      │
    │   │ - Compute state  │
    │   └──────────────────┘
    │
    └─ NO ──► Try next leader
```

## Data Flow

```rust
┌─────────────┐
│   Client    │
└──────┬──────┘
       │ Submit SignedTransaction
       ▼
┌─────────────────┐
│ BlockchainEngine│
│ .submit_tx()    │
└──────┬──────────┘
       │ Add to pending_txs
       ▼
┌─────────────────┐
│  DagEngine      │
│.produce_vertex()│
└──────┬──────────┘
       │
       ├──► Execute txs in parallel
       │    └─► Create state_root
       │
       ├──► Create DagVertex
       │    └─► Compute vertex hash
       │
       ├──► Add to DagStore
       │
       └──► Try commit
            └─► Create Checkpoint (if ready)
                 └─► Add to Blockchain
```

## Comparison: Linear Chain vs DAG

### Linear Chain

```rust
Block 1 ──► Block 2 ──► Block 3 ──► Block 4 ──► ...
  (TX1-10)   (TX11-20)   (TX21-30)   (TX31-40)

• Sequential creation
• One producer at a time
• ~1 second per block
```

### DAG

```rust
         Vertex V1 (Auth1)  ──┐
         Vertex V2 (Auth2)  ──┼──► Checkpoint 1
         Vertex V3 (Auth3)  ──┤    (TX1-40)
         Vertex V4 (Auth4)  ──┘
              │
         All created simultaneously!
         
• Parallel creation
• Multiple producers
• ~100ms to checkpoint
```

## State Management

```rust
┌──────────────────────────────────────────────────┐
│              Global State                        │
│  ┌────────────────────────────────────────┐      │
│  │  Accounts: {                           │      │
│  │    0x1: {balance: 1000, seq: 5},       │      │
│  │    0x2: {balance: 2000, seq: 3},       │      │
│  │    ...                                 │      │
│  │  }                                     │      │
│  │  Token Supplies: {...}                 │      │
│  │  Contract Modules: {...}               │      │
│  └────────────────────────────────────────┘      │
└────────────┬─────────────────────────────────────┘
             │
             │ Each vertex execution creates snapshot
             │
    ┌────────┴────────┬────────┬──────────────┐
    │                 │        │              │
┌───▼───┐       ┌─────▼──┐  ┌──▼────┐    ┌────▼──┐
│State  │       │ State  │  │ State │    │ State │
│Snap 1 │       │ Snap 2 │  │ Snap 3│    │ Snap 4│
└───┬───┘       └─────┬──┘  └──┬────┘    └────┬──┘
    │                 │        │              │
    │ Execute         │        │              │
    ▼                 ▼        ▼              ▼
┌──────────┐    ┌──────────┐ ┌──────────┐ ┌──────────┐
│ChangeSets│    │ChangeSets│ │ChangeSets│ │ChangeSets│
└────┬─────┘    └────┬─────┘ └──┬───────┘ └────┬─────┘
     │               │          │              │
     └───────────────┴──────────┴──────────────┘
                     │
                     │ Apply in order
                     ▼
            ┌────────────────┐
            │  Updated State │
            │  + State Root  │
            └────────────────┘
```

## Security Model

```rust
Byzantine Fault Tolerance (BFT)

Total Authorities: n = 4
Maximum Faulty: f = (n-1)/3 = 1
Quorum Required: 2f+1 = 3

┌─────────────────────────────────────────┐
│  Honest: Auth1, Auth2, Auth3  (✓✓✓)    │
│  Byzantine: Auth4             (✗)       │ 
└─────────────────────────────────────────┘

For vertex to be valid:
• Must reference ≥ 3 parents
• Parents must be from previous round
• Signature must be valid

For commit to happen:
• Leader vertex needs ≥ 3 supporters
• Even if Auth4 is malicious, 3 honest
  authorities form quorum
```

---

**Architecture Status**: ✅ Fully Implemented  
**All layers integrated and tested**
