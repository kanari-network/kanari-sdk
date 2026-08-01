# Kanari Crypto Dependency Audit

Last checked: 2026-08-02

Command:

```text
cargo audit -q
```

## Result

`cargo audit` completed successfully and reported advisories in the workspace lockfile.

Direct PQC-related findings affecting `kanari-crypto`:

- `pqcrypto-dilithium 0.5.0` — `RUSTSEC-2024-0380`, unmaintained, originally replaced by `pqcrypto-mldsa`.
- `pqcrypto-sphincsplus 0.7.2` — `RUSTSEC-2026-0160`, unmaintained because the upstream PQClean project is being archived.
- `pqcrypto-traits 0.3.5` — `RUSTSEC-2026-0162`, unmaintained because the upstream PQClean project is being archived.
- `pqcrypto-internals 0.2.11` — `RUSTSEC-2026-0163`, unmaintained because the upstream PQClean project is being archived.

Other workspace findings observed during the same audit:

- `hickory-proto 0.25.2` — `RUSTSEC-2026-0118`, no fixed upgrade available.
- `hickory-proto 0.25.2` — `RUSTSEC-2026-0119`, fixed in `>=0.26.1`.
- `rsa 0.9.10` — `RUSTSEC-2023-0071`, no fixed upgrade available.
- `event-listener 5.4.1` — `RUSTSEC-2026-0221`, unsound.
- `bincode 1.3.3` / `bincode 2.0.1`, `difference 2.0.0`, `paste 1.0.15` — unmaintained warnings.
- `spin 0.9.8` — yanked warning.

## Current mitigation in this crate

- PQC private key import now rejects oversized formatted private keys before parsing to reduce memory/CPU denial-of-service risk.
- PQC/hybrid import tests include deterministic malformed seed corpus and higher proptest case counts.
- Signature verification tests include deterministic malformed tagged-address/signature corpus and oversized signature rejection coverage.
- Wallet/keystore compatibility tests cover legacy missing metadata, legacy array-form encrypted data, and older wallet TOML without newer optional fields.

## Required production follow-up

- Plan a PQC migration away from `pqcrypto-dilithium` to an actively maintained ML-DSA implementation.
- Do not migrate to `pqcrypto-mldsa` as the final production answer; RustSec also reports `pqcrypto-mldsa` as unmaintained due to PQClean archival.
- Re-evaluate SPHINCS+/SLH-DSA provider options before enabling it as a default production curve.
- Re-run `cargo audit` in CI on every dependency update and block newly introduced vulnerabilities.
