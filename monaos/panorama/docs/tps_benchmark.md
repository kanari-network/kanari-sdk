# TPS Benchmark Tool - User Guide

The TPS (Transactions Per Second) Benchmark Tool is designed to measure and evaluate the performance of the Panorama blockchain system under various transaction processing conditions.

## Overview

This tool provides multiple testing modes to evaluate different aspects of transaction processing performance:

- Single-threaded processing
- Multi-threaded processing
- Transaction generation efficiency
- Performance under simulated network conditions

## Installation

To build the benchmark tool:

```bash
cargo build --bin tps_benchmark --release
```

## Basic Command Format

```bash
cargo run --bin tps_benchmark -- [OPTIONS]
```

Or if you've already built the binary:

```bash
./target/release/tps_benchmark [OPTIONS]
```

## Testing Modes

The benchmark tool offers the following modes of operation to test different performance aspects:

| Mode | Description |
|------|-------------|
| `single` | Tests transaction processing in a single-threaded environment |
| `multi` | Tests transaction processing using multiple threads (default) |
| `generate` | Measures only the transaction generation performance |
| `network` | Tests performance under simulated network conditions (latency and failure rates) |

## Supported Parameters

### Basic Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `-n`, `--transactions` | Number of transactions to test | 100,000 |
| `-j`, `--threads` | Number of threads for multi-threaded mode | 8 |
| `-m`, `--mode` | Testing mode (single, multi, generate, network) | multi |
| `-t`, `--target-tps` | Target TPS to achieve | 100,000 |

### Advanced Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `--tx-type` | Transaction type (transfer, complex, mixed) | transfer |
| `--complexity` | Transaction complexity factor (0-100) | 0 |
| `--accounts` | Number of test accounts to use | 1 |
| `--latency` | Simulated network latency in milliseconds | 0 |
| `--failure-rate` | Transaction failure rate (0-100%) | 0.0 |

## Usage Examples

### 1. Basic Multi-threaded Test

Run a standard benchmark with default parameters:

```bash
cargo run --bin tps_benchmark
```

### 2. Single-threaded Test

Test performance in a single-threaded environment:

```bash
cargo run --bin tps_benchmark -- -m single
```

### 3. Transaction Generation Performance

Measure only the transaction generation speed without submission:

```bash
cargo run --bin tps_benchmark -- -m generate -n 1000000
```

### 4. Simulated Network Conditions

Test with network latency and failure rate:

```bash
cargo run --bin tps_benchmark -- -m network --latency 100 --failure-rate 10
```

### 5. Complex Transaction Testing

Test with complex transactions:

```bash
cargo run --bin tps_benchmark -- --tx-type complex --complexity 50
```

### 6. Multiple Account Testing

Test with multiple accounts:

```bash
cargo run --bin tps_benchmark -- --accounts 100
```

### 7. High-Performance Testing

Test with many threads and transactions:

```bash
cargo run --bin tps_benchmark -- --mode multi --threads 32 --transactions 500000
```

### 8. Custom TPS Target

Set a specific TPS target:

```bash
cargo run --bin tps_benchmark -- --target-tps 75000
```

## Interpreting Results

After the benchmark completes, a summary is displayed in the following format:

```
=== MULTI-THREADED TPS BENCHMARK ===
Transaction count: 100000 (successful: 95026)
Thread count: 8
Transaction type: Transfer (complexity: 0)
Account count: 1
Generation time: 1.23s (81300.81 tx/s)
Submission time: 0.89s (106770.79 tx/s)
Total time: 2.12s
====================================

✅ TARGET ACHIEVED: The benchmark met or exceeded the target of 100000 TPS!
```

The output includes:
- Total transaction count and successful transactions
- Thread count (for multi-threaded mode)
- Transaction type and complexity level
- Number of accounts used
- Generation time and rate
- Submission time and TPS achieved (the primary performance metric)
- Total execution time
- Success status compared to the target TPS

## Best Practices

1. **Start with basic tests** - Begin with default settings to understand your system's baseline performance.

2. **Isolate generation vs. processing** - Use the `generate` mode to measure transaction creation separately.

3. **Scale complexity incrementally** - Progress from simple to complex scenarios:
   - Increase transaction count
   - Increase transaction complexity
   - Add more accounts
   - Introduce network conditions

4. **Find the optimal thread count** - Test with different thread counts to find the sweet spot for your hardware.

5. **Compare against realistic targets** - Set TPS targets based on your production requirements.

## Troubleshooting

### Performance Lower Than Expected

- Check system resources (CPU, memory utilization)
- Reduce transaction count or complexity
- Ensure no other resource-intensive applications are running

### Transaction Generation Failures

- Check logs for specific error messages
- Verify that address formats are correct
- Ensure sufficient memory is available

### Program Crashes

- May be caused by memory limitations - try reducing transaction batch size
- Increase log verbosity for more detailed error information
