<div align="center">

## Kanari Network: A Decentralized Metadata Management Protocol
#### Technical Whitepaper

https://kanari.network

Version 1.0.0 (February 2025)
</div>

### Abstract

Kanari Network is a decentralized protocol for managing metadata about digital assets with cryptographic guarantees for integrity, provenance, access control, and versioning. Existing decentralized storage solutions address content availability but do not provide a unified, auditable system for metadata lifecycle management. Kanari combines the Move VM's safety model with on-chain records to create tamper-evident metadata entries, verifiable ownership, and fine-grained permissioning without relying on centralized intermediaries.

The protocol captures metadata as immutable, signed records linked into an append-only history. Access controls and delegation are expressed as verifiable capabilities, while efficient indexing and compact proofs enable external verifiers to confirm state without trusting any single operator. Kanari targets applications that require enterprise-grade metadata guarantees: provenance tracking, digital rights management, institutional registries, and composable Web3 services.

### 1. Introduction

Digital assets increasingly depend on metadata: descriptive attributes, provenance, licensing, and version history. When metadata is centralized, it becomes a single point of failure and a vector for tampering. Decentralized storage systems (e.g., IPFS) solve content addressing and distribution, but they leave the metadata layer under-specified: ownership claims, structured permissions, and authoritative updates remain brittle or off-chain.

Key limitations of current approaches:

- Lack of authoritative metadata records that are both tamper-evident and efficiently verifiable.
- Weak or absent mechanisms for delegating and enforcing access and update rights.
- Fragmented provenance models that hinder composability and auditability.
- Poorly defined version semantics across storage and registry layers.

Kanari addresses these gaps by providing:

- Cryptographically anchored metadata records managed by Move-based smart contracts.
- Clear ownership semantics with transfer, delegation, and revocation primitives.
- Versioning and immutable change history for each metadata object.
- Compact proofs and APIs for off-chain verifiers and indexing services.

In the sections that follow, we describe Kanari's data model, Move contracts and native extensions used to enforce policy, the protocol's primitives for ownership and delegation, and how clients and indexers interact to achieve scalable, auditable metadata management.
