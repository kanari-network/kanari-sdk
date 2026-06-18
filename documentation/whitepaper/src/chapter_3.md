## 3. MoveVM Execution Layer

### 3.1 Why Move for Kanari

Kanari uses Move to express programmable state transitions with deterministic execution rules. This is useful for payment and asset workloads where ownership, balances, and module-level policy need to remain explicit.

### 3.2 Execution Model

The execution path is intentionally simple:

1. signed transactions enter the mempool
2. ordered batches are selected for execution
3. Move applies state changes deterministically
4. committed effects are persisted into checkpoint history

Execution does not define consensus by itself. It consumes ordered work and produces state changes that can be verified and replayed.

### 3.3 Transaction-Driven Progress

Kanari does not advance checkpoint height just because the network is alive. The Move layer participates only when there is real work to execute.

Benefits of this model:

- no empty-checkpoint spam
- clearer explorer semantics
- state-root changes stay tied to actual transactions

### 3.4 Performance Notes

Observed performance depends on:

- transaction mix
- signer distribution
- worker count
- storage backend cost
- state-root update strategy

For that reason, benchmark numbers should always be published together with the exact command, code revision, and hardware profile used.

### 3.5 Developer Workflow

Typical development flow:

```bash
# Create or update a Move package

# Build modules

# Run tests

# Publish through the Kanari toolchain
```

Developers should treat the runtime as deterministic infrastructure, not as a source of synthetic chain activity.
