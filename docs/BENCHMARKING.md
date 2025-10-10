# RustIRCd Benchmarking Guide

## Overview

This guide covers all aspects of benchmarking RustIRCd, from micro-benchmarks to load testing to long-term stability testing. Use this guide to verify performance, detect regressions, and optimize the server.

## Quick Start

```bash
# Run all micro-benchmarks
cargo bench

# Run specific benchmark group
cargo bench netsplit

# Run load tests
cd tests/load
./connection_stress.py --clients 1000
./message_throughput.py --rate 10000 --duration 60
./mixed_workload.py --users 100 --duration 300

# Check for performance regressions
./scripts/bench-compare.sh --threshold 5

# Long-term stability test (24 hours)
./tests/stability/memory_leak_test.py --duration 86400
```

## Benchmark Types

### 1. Micro-Benchmarks (Criterion)

**Location:** `core/benches/benchmarks.rs`

**Benchmark Groups:**
- `message_parsing` - IRC message parsing (1-5 µs)
- `message_serialization` - Message to string conversion (2-8 µs)
- `database` - User/channel database operations (1-20 µs)
- `cache` - LRU and message cache performance (200ns-5µs)
- `broadcast` - Channel subscriptions and broadcasting
- `batch_optimizer` - Message batching (1-2 µs)
- `validation` - Nickname/channel validation
- `user_modes` - User mode operations
- `netsplit` - Netsplit recovery operations
- `server_to_server` - Server message propagation
- `network_topology` - Split severity calculations

**Running:**
```bash
# All benchmarks
cargo bench

# Specific group
cargo bench message_parsing
cargo bench netsplit

# With verbose output
cargo bench -- --verbose

# Save baseline
cargo bench -- --save-baseline v1.0

# Compare against baseline
cargo bench -- --baseline v1.0
```

**Output:**
```
message_parsing/NICK alice
                        time:   [2.1234 µs 2.2345 µs 2.3456 µs]
                        change: [-1.5% -0.5% +0.5%]
```

### 2. Load Tests (Python)

**Location:** `tests/load/`

**Available Tests:**

**a) Connection Stress (`connection_stress.py`)**
- Tests concurrent connection handling
- Target: 10,000+ connections
- Metrics: Conn/sec, success rate, connect time

```bash
./connection_stress.py --clients 1000 --rate 50
```

**b) Message Throughput (`message_throughput.py`)**
- Tests message processing capacity
- Target: 100,000+ messages/second
- Metrics: Latency P50/P95/P99, throughput

```bash
./message_throughput.py --rate 10000 --duration 60
```

**c) Channel Load (`channel_load.py`)**
- Tests channel-specific performance
- Varying channel sizes (10-1000+ members)
- Metrics: Broadcast latency by size, JOIN performance

```bash
./channel_load.py --channels 50 --max-users 500
```

**d) Mixed Workload (`mixed_workload.py`)**
- Realistic traffic simulation
- 70% channel messages, 20% PMs, 5% joins/parts, 3% modes, 2% oper
- Metrics: Overall throughput, operation distribution

```bash
./mixed_workload.py --users 100 --channels 20 --duration 300
```

### 3. Stability Tests

**Location:** `tests/stability/`

**Memory Leak Detection (`memory_leak_test.py`)**
- Long-running test (1-24+ hours)
- Monitors memory growth
- Detects leaks and generates graphs

```bash
# 1 hour test
./tests/stability/memory_leak_test.py --duration 3600

# 24 hour test
./tests/stability/memory_leak_test.py --duration 86400 --interval 300
```

**Output:**
- Memory usage samples (RSS/VMS)
- Growth rate analysis
- Leak detection (>50% growth flagged)
- JSON results with full data
- CSV for plotting

### 4. Regression Testing

**Location:** `scripts/bench-compare.sh`

**Features:**
- Compares current branch vs baseline (main)
- Uses Criterion for accurate comparison
- Configurable regression threshold
- Generates detailed reports

```bash
# Default (5% threshold)
./scripts/bench-compare.sh

# Custom threshold (10%)
./scripts/bench-compare.sh --threshold 10

# Compare against specific branch
./scripts/bench-compare.sh --baseline production-v1.0
```

**Output:**
```
Current branch: feature/optimization
Baseline branch: main
Regression threshold: 5%

Step 1: Running benchmarks on current branch...
Step 2: Switching to baseline branch...
Step 3: Running benchmarks on baseline...
Step 4: Comparing results...

COMPARISON RESULTS
==================
IMPROVEMENT: message_parsing/NICK -8.5%
IMPROVEMENT: database/get_user_by_nick -12.3%
REGRESSION: broadcast/subscribe_to_channel +3.2%

SUMMARY
=======
Improvements: 15
Regressions: 1
Max regression: 3.2%

✓ BENCHMARK COMPARISON PASSED
```

## Performance Targets

### Connections
- **Concurrent**: 10,000+ connections
- **Accept rate**: 100+ connections/second
- **Connect latency**: <200ms (with DNS/ident)
- **Memory**: ~10KB per connection

### Messages
- **Throughput**: 100,000+ messages/second
- **Latency P50**: <1ms
- **Latency P95**: <3ms
- **Latency P99**: <5ms

### Channels
- **JOIN (10 members)**: <1ms
- **JOIN (100 members)**: <5ms
- **JOIN (1000 members)**: <10ms
- **Broadcast**: <10ms for 1000-member channel

### Database
- **Add user**: 5-15 µs
- **Lookup by nick**: 1-3 µs
- **Update user**: 8-20 µs

### Cache
- **LRU insert**: 2-5 µs
- **LRU get (hit)**: 200-500 ns
- **Message cache**: 1-3 µs

### Server-to-Server
- **Burst (1000 users)**: <1s
- **Burst (10000 users)**: <5s
- **Message propagation**: <1ms per hop
- **Netsplit recovery**: <5s

## Benchmark Workflows

### Development Workflow

```bash
# 1. Before making changes
cargo bench -- --save-baseline before

# 2. Make your changes
# ... code changes ...

# 3. Run benchmarks and compare
cargo bench -- --baseline before

# 4. If performance improved, update baseline
cargo bench -- --save-baseline after
```

### Pre-Commit Workflow

```bash
# Quick sanity check
cargo bench message_parsing
cargo bench database

# Full benchmark suite
cargo bench

# Regression check
./scripts/bench-compare.sh
```

### Release Workflow

```bash
# 1. Full benchmark suite
cargo bench -- --save-baseline v1.0.0

# 2. Load tests
cd tests/load
./connection_stress.py --clients 5000
./message_throughput.py --rate 50000 --duration 120
./channel_load.py --channels 100 --max-users 500
./mixed_workload.py --users 200 --duration 600

# 3. Stability test (overnight)
./tests/stability/memory_leak_test.py --duration 28800  # 8 hours

# 4. Document results
cat target/bench-compare/summary.txt >> RELEASE_NOTES.md
```

### CI/CD Integration

```yaml
# Example GitHub Actions workflow
name: Performance Tests

on: [pull_request]

jobs:
  benchmarks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Run regression tests
        run: ./scripts/bench-compare.sh --threshold 5
      
      - name: Upload results
        uses: actions/upload-artifact@v2
        with:
          name: benchmark-results
          path: target/bench-compare/
```

## Interpreting Results

### Criterion Output

```
message_parsing/NICK alice
                        time:   [2.1234 µs 2.2345 µs 2.3456 µs]
                        change: [-1.5% -0.5% +0.5%]
```

- **time**: [lower bound, estimate, upper bound] - 95% confidence interval
- **change**: Performance change vs baseline
  - Negative = faster (good!)
  - Positive = slower (potential regression)
  - Within ±2% = noise, not significant

### Load Test Results

**Good Performance:**
- Success rate > 99%
- Actual rate matches target (±5%)
- Latency P99 < 5ms
- No timeouts or errors

**Performance Issues:**
- Success rate < 95% - Check connection limits, buffer sizes
- Low actual rate - Check CPU usage, enable batching
- High latency - Check sendq/recvq, network conditions
- Timeouts - Increase connection timeout, check load

### Memory Leak Test

**Healthy:**
- Memory growth < 20% over time
- Stable memory usage after warmup
- No continuous growth trend

**Potential Leak:**
- Memory growth > 50%
- Continuous upward trend
- No stabilization over time

**Action:**
- Review memory profiles (heaptrack/valgrind)
- Check for cyclic references
- Review resource cleanup
- Report issue with leak test results

## Best Practices

### Benchmarking

1. **Consistent Environment:**
   - Use same hardware for comparisons
   - Close other applications
   - Disable CPU frequency scaling
   - Run multiple times for confidence

2. **Baseline Management:**
   - Save baselines for each release
   - Compare like with like (same Rust version)
   - Document hardware specs with results

3. **Realistic Workloads:**
   - Test with production-like data
   - Mix of operations like real usage
   - Include error scenarios

4. **Regression Detection:**
   - Run on every PR
   - Set appropriate thresholds (5-10%)
   - Investigate all regressions
   - Don't merge regressive changes without justification

### Load Testing

1. **Progressive Loading:**
   - Start with low load
   - Gradually increase
   - Find breaking point
   - Test at 150% of expected peak

2. **Sustained Testing:**
   - Run for extended periods (hours)
   - Look for degradation over time
   - Monitor resource usage
   - Check for memory leaks

3. **Realistic Scenarios:**
   - Mix of message types
   - Varying channel sizes
   - Client churn (joins/quits)
   - Server-to-server traffic

## Optimization Tips

If benchmarks show performance issues:

### High CPU Usage
- Enable message batching
- Increase cache sizes
- Reduce ping frequency
- Profile with flamegraph
- Check for inefficient loops

### High Memory Usage
- Reduce cache sizes
- Enable cache TTL
- Adjust grace periods
- Check for leaks
- Review buffer sizes

### High Latency
- Increase sendq/recvq
- Enable burst optimization
- Check network conditions
- Review throttling settings
- Optimize channel sizes

### Low Throughput
- Enable batch optimizer
- Increase worker threads
- Review connection classes
- Check for bottlenecks
- Profile with perf

## Troubleshooting

### Benchmark Failures

**Criterion compilation errors:**
```bash
# Update dependencies
cargo update
cargo clean
cargo bench
```

**Timeout errors:**
```bash
# Increase timeout
cargo bench -- --measurement-time 20
```

### Load Test Issues

**Connection refused:**
```bash
# Check server is running
ps aux | grep rustircd
netstat -tulpn | grep 6667

# Start server
cargo run --release
```

**Too many open files:**
```bash
# Increase file descriptor limit
ulimit -n 65535
```

**Connection timeout:**
```bash
# Check server load
top -p $(pgrep rustircd)

# Review server logs
tail -f rustircd.log
```

## Additional Tools

### Flamegraph

```bash
cargo install flamegraph
cargo flamegraph --bin rustircd
# Opens flame.svg in browser
```

### Criterion Report

```bash
cargo bench
open target/criterion/report/index.html
```

### Memory Profiling

```bash
# Linux
heaptrack ./target/release/rustircd

# macOS
instruments -t "Leaks" ./target/release/rustircd
```

## Performance Checklist

Before release:
- [ ] All micro-benchmarks pass
- [ ] No regressions vs previous release
- [ ] Load tests meet targets
- [ ] 24-hour stability test passed
- [ ] No memory leaks detected
- [ ] Profile reviewed for hotspots
- [ ] Documentation updated
- [ ] Performance results archived

## References

- **Criterion Documentation**: https://bheisler.github.io/criterion.rs/
- **Performance Guide**: See `docs/PERFORMANCE.md`
- **Load Testing**: See `tests/load/README.md`
- **Netsplit Recovery**: See `docs/NETSPLIT_RECOVERY.md`

---

**Last Updated**: January 2025

