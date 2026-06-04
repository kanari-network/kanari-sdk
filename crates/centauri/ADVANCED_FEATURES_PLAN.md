# Centauri Consensus Roadmap

## Purpose

This roadmap tracks the next meaningful delivery milestones for `centauri`.
It is intentionally short: only active or near-term release work should appear here.

## Current Baseline

The repository already includes:

- core DAG consensus
- checkpointing
- pruning and persistence
- state sync
- Mysticeti deterministic multi-leader selection
- parallel validation
- metrics export
- adaptive quorum policy hooks

The next work should focus on integration quality, operational readiness, and narrowing the gap between primitives and production flows.

## Release Milestones

### Milestone A: Integration Hardening

**Goal**: make the current consensus surface behave as one coherent system.

**Scope**:

- tighten integration between DAG consensus, engine, and RPC paths
- reduce test flakiness and environment-coupled behavior
- align public APIs with actual supported runtime flows
- remove or isolate any remaining test-only concepts that leak into production-facing modules

**Exit Criteria**:

- core `centauri`, `kanari-core`, and `kanari-rpc-server` test suites pass reliably
- metrics and quorum policy are reachable through stable runtime paths
- docs describe current behavior without roadmap drift

### Milestone B: Observability and Operations

**Goal**: make the system easier to run, debug, and measure under load.

**Scope**:

- expand metrics beyond basic counters and gauges
- add latency and throughput histograms where they matter
- document operational expectations for persistence, checkpoint roots, and DAG progress
- add benchmark/stress coverage for consensus hot paths

**Exit Criteria**:

- operators can inspect consensus health from runtime metrics alone
- hot-path regressions are visible through tests or benchmarks
- failure modes around backpressure, sync lag, and storage are measurable

### Milestone C: Adaptive Policy Maturation

**Goal**: turn adaptive quorum from a policy hook into an operationally credible feature.

**Scope**:

- feed real network-health signals into quorum policy
- tune thresholds using simulation or stress testing
- define whether policy state is ephemeral or persisted
- validate that adaptive behavior improves resilience without harming liveness

**Exit Criteria**:

- quorum adaptation is driven by real telemetry instead of manual snapshots
- thresholds are justified by tests or experiments
- policy behavior is predictable enough to support production rollout decisions

## Deferred Until Explicit Proposal

These are intentionally out of the near-term roadmap:

- sharding and cross-shard atomic commit
- optimistic execution
- ML-based Byzantine prediction
- light-client redesign
- ZK proof systems

They should come back only with a concrete design note, owner, and implementation path.

## Planning Rule

If a feature is not implemented or actively scheduled for a milestone, this file should not present it as current capability.
