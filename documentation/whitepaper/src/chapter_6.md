## 6. The Universal Payment Ecosystem

### 6.1 Developer Surfaces

Kanari exposes a set of tools for application teams:

- node runtime and CLI workflows
- JSON-RPC endpoints
- Move packages and publish flows
- explorer and monitoring surfaces

### 6.2 Integration Approach

A typical integration uses RPC for reads and transaction submission, while Move modules define application behavior.

Common patterns include:

- balance and asset queries
- transaction submission
- module publishing
- object and ownership inspection

### 6.3 Explorer Semantics

Explorer output should reflect committed history, not synthetic progress. If the explorer shows unexpected transaction count changes, the correct first step is to inspect checkpoint persistence, transaction indexing, and any retained historical data before assuming new user traffic exists.

### 6.4 Operational Guidance

For accurate multi-node behavior:

- run the same node build on every authority
- avoid mixing old and new persistence semantics
- rebuild and restart when checkpoint logic changes
- verify both height and state root during debugging

### 6.5 Application Direction

Kanari is suitable for teams building application-specific payment and asset systems where clear state semantics matter as much as raw execution speed.
