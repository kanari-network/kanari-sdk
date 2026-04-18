# Centauri Consensus - Fuzzing Guide

**Version:** 0.1.5  
**Last Updated:** 2026-04-18  
**Status:** ✅ Production-Ready Fuzzing Infrastructure

---

## 🧪 Overview

This directory contains **coverage-guided fuzzing tests** for the Centauri consensus engine using [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) (libFuzzer).

Fuzzing helps discover:

- 🐛 **Panics and crashes** from unexpected inputs
- 💾 **Memory leaks** from unbounded data structures
- 🔒 **Security vulnerabilities** in validation logic
- ⚡ **Performance bottlenecks** under stress

---

## 📦 Prerequisites

### Install cargo-fuzz

```bash
cargo install cargo-fuzz
```

### Verify Installation

```bash
cargo fuzz --help
```

---

## 🎯 Available Fuzz Targets

| Target | Description | Key Invariants Tested | Estimated Runtime |
|--------|-------------|----------------------|-------------------|
| **fuzz_vertex_validation** | DagVertex creation with random inputs | Hash consistency, field preservation | 30 min |
| **fuzz_consensus_rounds** | Multi-round consensus simulation | Quorum calculation, stability | 1 hour |
| **fuzz_sync_protocol** | Sync request/response handling | DoS protection, buffer bounds | 45 min |
| **fuzz_byzantine_detection** | Byzantine fault injection | Reputation system, banning logic | 30 min |
| **fuzz_committee_transitions** | Epoch-based validator rotation | Minimum quorum, unique IDs | 30 min |

---

## 🚀 Quick Start

### Run Single Fuzz Target (30 minutes)

```bash
cd crates/centauri/fuzz

# Run vertex validation fuzzing
cargo fuzz run fuzz_vertex_validation -- -max_total_time=1800
```

### Run All Fuzz Targets (Sequential)

```bash
cd crates/centauri/fuzz

for target in fuzz_vertex_validation fuzz_consensus_rounds fuzz_sync_protocol \
              fuzz_byzantine_detection fuzz_committee_transitions; do
    echo "🔍 Fuzzing $target for 30 minutes..."
    cargo fuzz run $target -- -max_total_time=1800
done
```

### Run with Custom Corpus Directory

```bash
# Use persistent corpus across runs
cargo fuzz run fuzz_vertex_validation -- -artifact_prefix=./crashes/
```

---

## 📊 Interpreting Results

### Success Output

```bash
INFO: Running with entropic power schedule (0xFF, 100).
INFO: Seed: 1234567890
INFO: Loaded 1 modules   (5000 inline 8-bit counters): 5000 [0x..., 0x...), 
INFO: Loaded 1 PC tables (5000 PCs): 5000 [0x...,0x...), 
#2      INITED cov: 100 ft: 100 corp: 1/1b exec/s: 0 rss: 50Mb
#1024   NEW    cov: 150 ft: 150 corp: 2/5b exec/s: 0 rss: 52Mb
#8192   pulse  cov: 200 ft: 200 corp: 5/20b exec/s: 4096 rss: 55Mb
...
Done 100000 runs in 1800 second(s)
```

✅ **No crashes found** = Test passed!

---

### Crash Detected

```bash
ERROR: libFuzzer: deadly signal
NOTE: libFuzzer has rudimentary signal handlers.
      Combine libFuzzer with AddressSanitizer or similar for better crash reports.
SUMMARY: libFuzzer: deadly signal
MS: 1 ChangeByte-; base unit: abc123...
0x41,0x42,0x43,
ABC
artifact_prefix='./'; Test unit written to ./crash-abc123
Base64: QUJD
```

❌ **Crash found!** Take these steps:

1. **Examine the crash input:**

   ```bash
   xxd crash-abc123
   ```

2. **Reproduce the crash:**

   ```bash
   cargo fuzz run fuzz_vertex_validation crash-abc123
   ```

3. **Debug with sanitizers:**

   ```bash
   RUSTFLAGS="-Z sanitizer=address" cargo fuzz run fuzz_vertex_validation crash-abc123
   ```

4. **Fix the bug** and re-run to verify.

---

## 🔬 Advanced Usage

### Parallel Fuzzing (Multiple Cores)

```bash
# Run 4 parallel jobs on fuzz_consensus_rounds
cargo fuzz run fuzz_consensus_rounds --jobs=4 -- -max_total_time=3600
```

### Minimize Crash Input

```bash
# Reduce crash input to minimal reproducer
cargo fuzz tmin fuzz_vertex_validation crash-abc123 minimized-crash
```

### Merge Corpora from Multiple Runs

```bash
# Combine interesting inputs from different runs
cargo fuzz merge fuzz_vertex_validation/corpus1 fuzz_vertex_validation/corpus2 merged_corpus
```

### Custom Sanitizers

```bash
# AddressSanitizer (memory errors)
RUSTFLAGS="-Z sanitizer=address" cargo fuzz run fuzz_vertex_validation

# ThreadSanitizer (data races)
RUSTFLAGS="-Z sanitizer=thread" cargo fuzz run fuzz_consensus_rounds

# LeakSanitizer (memory leaks)
RUSTFLAGS="-Z sanitizer=leak" cargo fuzz run fuzz_sync_protocol
```

---

## 📈 Coverage Analysis

### Generate Coverage Report

```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Run with coverage
cargo tarpaulin --out Html --output-dir coverage_report
```

### View Coverage in Browser

```bash
open coverage_report/tarpaulin-report.html
```

**Target Coverage:** 85%+ on critical paths (consensus, networking, storage)

---

## 🐛 Common Issues & Solutions

### Issue 1: Out of Memory During Fuzzing

**Symptom:** `ERROR: libFuzzer: out of memory`

**Solution:**

```bash
# Limit memory usage
cargo fuzz run fuzz_target -- -rss_limit_mb=4096
```

---

### Issue 2: Slow Fuzzing Speed (< 1000 exec/sec)

**Causes:**

- Heavy I/O operations in fuzzed code
- Complex cryptographic operations
- Unoptimized debug builds

**Solutions:**

```bash
# Use release mode for faster execution
cargo fuzz run fuzz_target --release

# Reduce complexity in fuzz target (e.g., skip disk writes)
```

---

### Issue 3: False Positives from Non-Determinism

**Symptom:** Same input produces different results

**Cause:** Timestamps, random number generation, thread scheduling

**Solution:**

```rust
// In fuzz target, use deterministic values
let timestamp = 1_000_000_000u64; // Fixed instead of SystemTime::now()
```

---

## 🔄 CI/CD Integration

### GitHub Actions Workflow

Create `.github/workflows/fuzz.yml`:

```yaml
name: Fuzzing Tests

on:
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM UTC
  workflow_dispatch:     # Manual trigger

jobs:
  fuzz:
    runs-on: ubuntu-latest
    timeout-minutes: 120
    
    strategy:
      matrix:
        fuzz_target:
          - fuzz_vertex_validation
          - fuzz_consensus_rounds
          - fuzz_sync_protocol
          - fuzz_byzantine_detection
          - fuzz_committee_transitions
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz
      
      - name: Run fuzzing
        run: |
          cd crates/centauri/fuzz
          cargo fuzz run ${{ matrix.fuzz_target }} -- -max_total_time=3600
      
      - name: Upload artifacts on failure
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: fuzz-crashes-${{ matrix.fuzz_target }}
          path: crates/centauri/fuzz/artifacts/
```

---

## 📚 Best Practices

### 1. Keep Fuzz Targets Fast

❌ **Bad:**

```rust
// Slow: Writes to disk on every iteration
let _ = consensus.add_vertex(vertex);
consensus.persist_to_disk()?;  // ❌ Too slow!
```

✅ **Good:**

```rust
// Fast: In-memory only
let _ = consensus.add_vertex(vertex);  // ✅ No I/O
```

---

### 2. Validate Invariants, Not Just Crashes

❌ **Bad:**

```rust
// Only checks for panics
let vertex = DagVertex::new(...);
```

✅ **Good:**

```rust
// Checks logical invariants too
let vertex = DagVertex::new(...);
assert_eq!(vertex.round, expected_round);
assert!(!vertex.id.iter().all(|&b| b == 0));
```

---

### 3. Handle Expected Errors Gracefully

❌ **Bad:**

```rust
// Panics on invalid input
let result = sync.handle_sync_request(&request).unwrap();
```

✅ **Good:**

```rust
// Accepts both Ok and Err
let _ = sync.handle_sync_request(&request);
```

---

### 4. Use Realistic Input Distributions

❌ **Bad:**

```rust
// All zeros or all random
let round = 0u64;
```

✅ **Good:**

```rust
// Mix of edge cases and normal values
let round = u64::from_le_bytes(data[0..8].try_into().unwrap_or([0u8; 8]));
```

---

## 🎓 Learning Resources

- **[Rust Fuzz Book](https://rust-fuzz.github.io/book/)** - Official guide
- **[libFuzzer Tutorial](https://llvm.org/docs/LibFuzzer.html)** - LLVM documentation
- **[cargo-fuzz README](https://github.com/rust-fuzz/cargo-fuzz)** - Tool reference
- **[Proptest vs cargo-fuzz](https://proptest-rs.github.io/proptest/proptest-vs-other-crates.html)** - When to use what

---

## 📞 Support

**Found a bug through fuzzing?**

1. Save the crash input: `cp crash-* ../test_inputs/`
2. Create issue with stack trace
3. Add regression test to prevent recurrence

**Questions?**

- GitHub Issues: <https://github.com/KanariNetwork/kanari-sdk/issues>
- Discord: <https://discord.gg/kanarinetwork>

---

## 📝 Changelog

### v0.1.5 (2026-04-18)

- ✅ Initial fuzzing infrastructure created
- ✅ 5 fuzz targets implemented
- ✅ CI/CD integration guide added
- ✅ Comprehensive documentation

---

**Document Version:** 1.0  
**Maintained By:** KanariNetwork Engineering Team  
**Next Review Date:** 2026-07-18 (Quarterly)
