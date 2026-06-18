## 7. Roadmap & Conclusion

### 7.1 Current Engineering Priorities

The most important optimization areas are:

- incremental state-root maintenance
- reducing transaction cloning between execution and checkpoint paths
- keeping checkpoint persistence compact
- preserving deterministic replay and divergence visibility

### 7.2 Operational Safety

Kanari should prefer explicit invariants over hidden automation. That means:

- do not generate checkpoints without transactions
- do not hide state-root mismatches
- do not treat sync traffic as committed execution

These rules make debugging harder to ignore, but they keep the system honest.

### 7.3 Performance Scaling

Performance work should be measured, not assumed. Useful benchmark reports include:

- exact command line
- validator count
- sender distribution
- CPU and RAM profile
- workload mode

Without that context, a TPS number is only anecdotal.

### 7.4 Long-Term Direction

The long-term goal is a payment-oriented programmable network with:

- efficient execution
- compact persistence
- clear validator recovery behavior
- observable state convergence

### 7.5 Conclusion

Kanari's current design is centered on one simple idea: blockchain state should move only when real user work is committed. Mysticeti-style ordering, Move execution, and explicit state-root comparison all serve that goal.

That makes the system easier to reason about, easier to benchmark honestly, and easier to operate in a multi-node environment.
