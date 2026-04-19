## 2 Architecture & Centauri Consensus

### 2.1 What is DAG Consensus?

Traditional blockchains process transactions one block at a time, creating bottlenecks for payment systems. Kanari uses a **Directed Acyclic Graph (DAG)** structure that allows multiple validators to process transactions simultaneously.

**Key Difference:**

- **Blockchain**: Linear sequence → Slow, sequential processing
- **DAG**: Parallel structure → Fast, concurrent processing

This makes DAG ideal for **high-frequency payment networks** requiring instant settlement.

### 2.2 How Centauri Consensus Works

Centauri uses the Bullshark protocol with a simple 3-round commit process:

**Round 1**: Leader creates a vertex (transaction batch)
**Round 2**: Other validators reference the leader's vertex  
**Round 3**: If enough validators agree, the vertex is committed

#### Mathematical Formulas

**Byzantine Fault Tolerance (BFT)**
For a network with `n` validators:

- Maximum faulty validators tolerated: `f = floor((n-1)/3)`
- Minimum honest validators needed: `2f + 1`

**Example Calculation:**

- With 4 validators: `f = floor((4-1)/3) = 1`
- Need `2(1) + 1 = 3` honest validators to reach consensus

**Throughput Formula**
Maximum Transactions Per Second (TPS):

```
TPS = (Validators × Transactions per Vertex) / Finality Time
```

**Example:**

- 4 validators × 500 transactions per vertex ÷ 0.3 seconds = ~6,667 TPS
- With optimizations: Up to 50,000+ TPS

This throughput supports **global payment volumes** including e-commerce, remittances, and financial services.

### 2.3 Performance Characteristics

| Metric | Value |
|--------|-------|
| Finality Time | ~300 milliseconds |
| Transaction Execution | ~10 milliseconds |
| Throughput | 50,000+ TPS |
| Validator Requirements | 2f+1 honest nodes |

These metrics make Kanari suitable for **real-time payment processing** across all industries.

### 2.4 Security Model

**Network Security Formula:**
The probability of network compromise decreases exponentially with more validators:

```
Security = 1 - (f/n)^k
```

Where:

- `f` = maximum faulty validators
- `n` = total validators  
- `k` = number of consensus rounds

**Light Client Verification**
Light clients verify transactions using Merkle proofs:

```
verify_proof(state_root, account_data, merkle_path) = true/false
```

This allows **mobile wallets, e-commerce platforms, and banking apps** to verify payments without running full nodes.

### 2.5 Simple Architecture Overview

Kanari's system is built from specialized components:

**Core Components:**

- **centauri**: Handles consensus and transaction ordering
- **kanari-core**: Coordinates the entire system
- **kanari-move-runtime**: Executes smart contracts safely
- **kanari-crypto**: Provides secure cryptography
- **kanari-node**: Runs the complete network node

Each component has a specific job, making the system reliable and maintainable for **universal payment applications**.
