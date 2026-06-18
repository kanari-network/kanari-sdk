## Kanari Network

#### Technical Whitepaper

### Abstract

Kanari is a Rust-based network for programmable payments and application state transitions. The current design combines Mysticeti-style DAG ordering, a Move-based execution layer, and checkpoint persistence that advances only when real transactions exist.

This whitepaper documents the architecture as it exists today. It avoids fixed promises for throughput, latency, or finality because those values depend on hardware, validator topology, storage behavior, and workload shape.

### 1. Introduction

Kanari is designed around a small set of operational principles:

- consensus metadata and blockchain state are different layers
- checkpoints must correspond to executed transactions
- idle networks should not manufacture empty chain progress
- state divergence must be visible and debuggable

This approach is useful for payment systems, application backends, and asset workflows where developers care more about deterministic state progression than about synthetic block production.

The network is implemented primarily in Rust, uses Move for programmable execution, and keeps the state model explicit so operators can reason about persistence, replay, and recovery behavior.
