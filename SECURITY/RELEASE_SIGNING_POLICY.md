# Release Signing, SBOM, and Provenance Policy

## 1. Scope

Every public Kanari release tag must be built only from a tagged commit whose
CI gates are green, then signed before it is treated as installable. The
workflow `.github/workflows/signed-release.yml` implements the signing step.
It uses Sigstore keyless signing with the GitHub Actions OIDC identity of this
repository, so no long-lived private signing key is stored in CI.

This document defines the required process. It is not a claim that a specific
release has already been signed or audited.

## 2. Required gates before tagging

- `cargo fmt --all -- --check`
- `cargo deny check`
- `cargo test --workspace`
- the PR/nightly security gates, including four-node chaos and fuzz campaigns
- the explicitly ignored long-run soaks:
  - `cargo test -p kanari-core long_run_byzantine_native_blocks_cannot_advance_checkpoint -- --ignored --test-threads=1`
  - `cargo test -p kanari-node long_run_malformed_compressed_payloads_are_bounded -- --ignored --test-threads=1`

Framework artifact hashes must be published and pinned as described in
`SECURITY/FRAMEWORK_UPGRADE_POLICY.md`.

## 3. What the signing workflow produces

For each published tag:

- SHA-256 manifest of each release archive (`sha256sums.txt`);
- an SPDX JSON SBOM of the tagged source tree (`kanari-sbom.spdx.json`);
- a Sigstore bundle (`<asset>.sigstore.json`) for every archive, the checksum
  manifest, and the SBOM;
- a GitHub build-provenance attestation tying the subjects to the tagged commit
  and workflow run;
- local verification of every bundle before upload.

Required token permissions are `contents: write`, `id-token: write`, and
`attestations: write`. No repository secret is required for signing.

## 4. Verification by a user

```powershell
# Install cosign, then from a directory containing the release assets:
cosign verify-blob `
  --bundle kanari-windows-x86_64.zip.sigstore.json `
  --certificate-identity-regexp "https://github.com/kanari-network/kanari-sdk/.*" `
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com" `
  kanari-windows-x86_64.zip

Get-FileHash -Algorithm SHA256 kanari-windows-x86_64.zip
gh attestation verify kanari-windows-x86_64.zip --repo kanari-network/kanari-sdk
```

A release is invalid if any signature, checksum, or provenance attestation is
missing or does not match the tagged commit.

## 5. External audit handoff

An independent audit is a separate engagement and cannot be self-certified.
The handoff package is:

- reviewed commit hash and signed release tag;
- `SECURITY/SECURITY_AUDIT_SCOPE.md` with scope and evidence requirements;
- `SECURITY/FRAMEWORK_UPGRADE_POLICY.md` and this signing policy;
- green gate logs, soak results, chaos/fuzz artifacts, SBOM, and signatures;
- auditor-supplied report with severity, proof of concept, remediation commit,
  and retest statement.

Until that report exists, describe the project as security-hardened with local
gates, not as externally audited.
