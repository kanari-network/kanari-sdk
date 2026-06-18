## 4. The Zero-Fee Payment Infrastructure

### 4.1 Fee Policy

Kanari is designed for low-friction application flows. In the current developer setup, many examples use zero-gas or operator-controlled fee policy so teams can test payment paths without exposing end users to chain UX complexity.

### 4.2 Why This Matters

For payment-oriented systems, fee predictability is often more important than speculative token mechanics. Kanari therefore emphasizes:

- simple transfer flows
- predictable application behavior
- developer-controlled integration surfaces

### 4.3 Account and Asset Flows

Kanari supports programmable account and asset workflows through Move modules and RPC services. This allows teams to model balances, token logic, escrow, and application-specific rules without changing the consensus layer.

### 4.4 Economic Considerations

The cost model still exists even when user-facing gas is hidden:

- operators pay for hardware, storage, and availability
- validators pay for synchronization and persistence work
- application design still determines system load

A low-friction UX is sustainable only when runtime and storage behavior stay efficient.

### 4.5 Payment Scenarios

Example use cases include:

- consumer payment flows
- in-game assets and marketplace settlement
- creator payouts
- service credits and prepaid balances
- internal application accounting

The shared requirement across these cases is deterministic, observable state progression.
