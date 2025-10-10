# RustIRCd Performance Guide

## Overview

RustIRCd is designed for high performance and low resource usage while maintaining IRC protocol compliance and network stability. This guide covers performance characteristics, benchmarking, optimization, monitoring, and troubleshooting.

## Performance Characteristics

### Design Goals

- **High Throughput**: 100,000+ messages/second on modern hardware
- **Low Latency**: Sub-millisecond median latency for most operations
- **Scalability**: Support 10,000+ concurrent connections per server
- **Efficiency**: 30-50% less memory and CPU than traditional IRCd implementations
- **Predictability**: Consistent performance under load with minimal variance

### Architecture Advantages

**Rust Language Benefits:**
- Zero-cost abstractions with compile-time guarantees
- Memory safety without garbage collection pauses
- Efficient async/await with tokio runtime
- Lock-free data structures where possible

**Async I/O:**
- Non-blocking operations throughout
- Efficient task scheduling with tokio
- Minimal context switching overhead
- Scalable to thousands of concurrent connections

**Optimized Data Structures:**
- DashMap for concurrent access without locks
- LRU caches for frequently accessed data
- Message batching to reduce network overhead
- Connection pooling for server-to-server communication

## Performance Targets

### Connection Handling

| Metric | Target | Notes |
|--------|--------|-------|
| Concurrent connections | 10,000+ | ~10KB memory per connection |
| New connections/sec | 100+ | Limited by system accept() rate |
| Connection latency | <200ms | Including DNS/ident lookup |
| File descriptors | 1 per connection | Plus ~100 for internal use |

### Message Processing

| Metric | Target | Notes |
|--------|--------|-------|
| Message throughput | 100,000+ msg/s | Single server |
| Message parse time | 1-5 µs | Per message |
| Message serialize time | 2-8 µs | Per message |
| Latency P50 | <1ms | End-to-end |
| Latency P95 | <3ms | End-to-end |
| Latency P99 | <5ms | End-to-end |

### Database Operations

| Operation | Target | Notes |
|-----------|--------|-------|
| Add user | 5-15 µs | In-memory insert |
| Lookup by nick | 1-3 µs | Hash table lookup |
| Update user | 8-20 µs | Atomic update |
| Channel lookup | 1-5 µs | Hash table lookup |
| User in channel check | <1 µs | HashSet membership |

### Channel Operations

| Operation | Target | Notes |
|-----------|--------|-------|
| JOIN (10 members) | <1ms | Including broadcast |
| JOIN (100 members) | <5ms | Including broadcast |
| JOIN (1000 members) | <10ms | Including broadcast |
| PRIVMSG to channel | <10ms | For 1000-member channel |
| Channel broadcast | 10,000+ msg/s | Per channel |
| NAMES generation | <5ms | For 1000-member channel |

### Cache Performance

| Operation | Target | Notes |
|-----------|--------|-------|
| LRU insert | 2-5 µs | Thread-safe |
| LRU get (hit) | 200-500 ns | Lock-free read |
| Message cache hit | 1-3 µs | Pre-formatted messages |
| DNS cache hit | <1 µs | Cached resolution |

### Memory Usage

| Component | Per-Unit Memory | Notes |
|-----------|----------------|-------|
| Connection | ~10KB | Including buffers |
| User | ~1KB | In-memory state |
| Channel | ~2KB | Base overhead |
| Channel member | ~100 bytes | Per user in channel |
| Cache entry | ~200 bytes | Average size |

### Server-to-Server

| Operation | Target | Notes |
|-----------|--------|-------|
| Server burst (1000 users) | <1s | Initial sync |
| Server burst (10000 users) | <5s | Initial sync |
| Message propagation | <1ms | Hop delay |
| Netsplit detection | <100ms | Including cleanup |
| Netsplit recovery | <5s | Including burst |

## Benchmarking

### Running Micro-Benchmarks

RustIRCd uses Criterion for micro-benchmarks:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench message_parsing
cargo bench database
cargo bench cache

# Save baseline for comparison
cargo bench --save-baseline main

# Compare against baseline
cargo bench --baseline main

# Generate detailed report
cargo bench --verbose
```

### Benchmark Groups

**Available benchmarks:**
- `message_parsing` - IRC message parsing performance
- `message_serialization` - Message to string conversion
- `database` - User/channel database operations
- `cache` - LRU and message cache performance
- `broadcast` - Channel subscription and broadcasting
- `batch_optimizer` - Message batching performance
- `validation` - Nickname/channel name validation
- `user_modes` - User mode operations

### Running Load Tests

Python-based load testing scripts test real-world scenarios:

```bash
cd tests/load

# Connection stress test
./connection_stress.py --clients 1000 --rate 50

# Message throughput test
./message_throughput.py --rate 10000 --duration 60

# Channel-specific load test
./channel_load.py --channels 100 --users-per-channel 50

# Mixed workload (realistic traffic)
./mixed_workload.py --duration 300
```

### Interpreting Benchmark Results

**Good Performance Indicators:**
- Throughput meets or exceeds targets
- Latency P99 < 5ms
- Memory usage grows linearly with load
- CPU usage < 80% at target load
- No error rates or dropped messages

**Performance Issues:**
- Throughput significantly below target (>20% difference)
- High latency variance (P99 >> P50)
- Memory usage grows super-linearly
- High CPU usage at low load
- Increasing error rates under load

## Profiling

### CPU Profiling

**Using cargo-flamegraph (Recommended):**

```bash
# Install flamegraph
cargo install flamegraph

# Profile the server
cargo flamegraph --bin rustircd

# Opens flamegraph SVG in browser
# Hot paths shown in red/orange
```

**Using perf (Linux):**

```bash
# Build with debug symbols
cargo build --release

# Record profile
sudo perf record --call-graph dwarf -F 999 ./target/release/rustircd

# View report
sudo perf report

# Generate flamegraph from perf data
git clone https://github.com/brendangregg/FlameGraph
perf script | FlameGraph/stackcollapse-perf.pl | FlameGraph/flamegraph.pl > flame.svg
```

**Using Instruments (macOS):**

```bash
# Build with debug symbols
cargo build --release

# Open in Instruments
instruments -t "Time Profiler" ./target/release/rustircd
```

### Memory Profiling

**Using heaptrack (Linux):**

```bash
# Install heaptrack
sudo apt install heaptrack heaptrack-gui  # Debian/Ubuntu

# Profile memory usage
heaptrack ./target/release/rustircd

# View results
heaptrack_gui heaptrack.rustircd.*.gz
```

**Using valgrind massif:**

```bash
# Run with massif
valgrind --tool=massif \
  --massif-out-file=massif.out \
  ./target/release/rustircd

# View results
ms_print massif.out
```

**Memory leak detection:**

```bash
# Run long-term stability test
python tests/stability/memory_leak_test.py \
  --duration 86400  # 24 hours
```

### Async Profiling

**Using tokio-console:**

```toml
# Add to Cargo.toml
[dependencies]
console-subscriber = "0.1"
```

```rust
// Add to main.rs
#[tokio::main]
async fn main() {
    console_subscriber::init();
    // ... rest of main
}
```

```bash
# Install and run console
cargo install tokio-console
tokio-console
```

Shows:
- Active tasks and their states
- Task durations and poll times
- Resource usage per task
- Async bottlenecks

## Optimization Guide

### Configuration Tuning

**Connection Classes:**

```toml
[[classes]]
name = "default"
sendq = 1048576      # 1MB - increase for high throughput
recvq = 8192         # 8KB - increase if message loss
ping_frequency = 120 # Decrease for faster timeout detection
connection_timeout = 600  # Increase for slower networks
```

**Cache Tuning:**

```toml
[database]
user_cache_size = 10000          # Increase for more users
user_cache_ttl_seconds = 300     # Increase for stable networks
channel_cache_ttl_seconds = 30   # Tune based on channel churn
```

**Batch Optimization:**

```toml
[batch]
max_batch_size = 50          # Messages per batch
max_batch_delay = 10         # Milliseconds
max_batch_bytes = 4096       # Bytes per batch
```

**Netsplit Recovery:**

```toml
[netsplit]
split_user_grace_period = 60      # Seconds - balance memory vs recovery
burst_optimization_enabled = true  # Enable for frequent netsplits
burst_optimization_window = 300    # Seconds
```

### System Tuning

**File Descriptor Limits:**

```bash
# Check current limits
ulimit -n

# Increase for current session
ulimit -n 65535

# Permanent increase (add to /etc/security/limits.conf)
* soft nofile 65535
* hard nofile 65535
```

**TCP Tuning (Linux):**

```bash
# Increase TCP buffer sizes
sudo sysctl -w net.core.rmem_max=16777216
sudo sysctl -w net.core.wmem_max=16777216
sudo sysctl -w net.ipv4.tcp_rmem="4096 87380 16777216"
sudo sysctl -w net.ipv4.tcp_wmem="4096 65536 16777216"

# Enable TCP fast open
sudo sysctl -w net.ipv4.tcp_fastopen=3

# Increase connection backlog
sudo sysctl -w net.core.somaxconn=4096
sudo sysctl -w net.ipv4.tcp_max_syn_backlog=8192

# Make permanent (add to /etc/sysctl.conf)
```

**Kernel Parameters:**

```bash
# Increase ephemeral port range
sudo sysctl -w net.ipv4.ip_local_port_range="10000 65535"

# Reduce TIME_WAIT timeout
sudo sysctl -w net.ipv4.tcp_fin_timeout=30

# Enable connection recycling
sudo sysctl -w net.ipv4.tcp_tw_reuse=1
```

### Code-Level Optimizations

**Hot Path Optimization:**

1. **Avoid Allocations**: Use `&str` instead of `String` where possible
2. **Cache Frequently Used Data**: Pre-format common messages
3. **Batch Operations**: Combine multiple operations when possible
4. **Use References**: Avoid cloning large structures
5. **Lock-Free When Possible**: Prefer atomic operations

**Example - Message Caching:**

```rust
// Instead of:
let msg = Message::new(MessageType::Ping, vec![token]);
client.send(msg.to_string());

// Cache common messages:
let cached_msg = message_cache.get_or_insert(
    &format!("PING {}", token),
    || format!("PING {}\r\n", token)
);
client.send_raw(cached_msg);
```

## Monitoring

### Metrics to Track

**Connection Metrics:**
- Active connections (gauge)
- New connections per second (counter)
- Connection failures per second (counter)
- Average connection lifetime (histogram)

**Message Metrics:**
- Messages per second (counter)
- Message latency P50/P95/P99 (histogram)
- Parse errors per second (counter)
- Dropped messages per second (counter)

**Channel Metrics:**
- Active channels (gauge)
- Channel members (histogram)
- Broadcast latency by channel size (histogram)
- JOIN/PART rate (counter)

**Resource Metrics:**
- CPU usage percentage (gauge)
- Memory usage MB (gauge)
- File descriptors open (gauge)
- Network bytes in/out (counter)

**Server-to-Server Metrics:**
- Connected servers (gauge)
- Burst duration (histogram)
- Message propagation latency (histogram)
- Netsplit events (counter)

### Using STATS Commands

Monitor performance through IRC STATS commands:

```
/STATS m  - Command usage statistics
/STATS l  - Server link statistics (sendq/recvq)
/STATS u  - Server uptime
/STATS c  - Connection statistics
/STATS y  - Connection class statistics
```

### External Monitoring

**Prometheus Integration:**

```rust
// Add to Cargo.toml
[dependencies]
prometheus = "0.13"
```

Expose metrics on `/metrics` endpoint for Prometheus scraping.

**Log Analysis:**

```bash
# Monitor connection rate
grep "Client connected" rustircd.log | wc -l

# Check for errors
grep "ERROR" rustircd.log | tail -20

# Monitor memory usage
ps aux | grep rustircd | awk '{print $6/1024 " MB"}'
```

## Troubleshooting

### High CPU Usage

**Symptoms:**
- CPU usage >80% at moderate load
- Increased latency
- Dropped connections

**Diagnosis:**
```bash
# Profile CPU usage
cargo flamegraph --bin rustircd

# Check for hot loops
perf top -p $(pgrep rustircd)
```

**Solutions:**
- Enable message batching
- Increase cache sizes
- Reduce ping frequency
- Check for inefficient modules
- Review custom configurations

### High Memory Usage

**Symptoms:**
- Memory usage grows continuously
- OOM errors
- Swapping

**Diagnosis:**
```bash
# Profile memory
heaptrack ./target/release/rustircd

# Monitor memory growth
watch -n 1 'ps aux | grep rustircd'
```

**Solutions:**
- Reduce cache sizes
- Enable cache TTL
- Check for memory leaks (run leak test)
- Review user/channel limits
- Adjust grace periods

### High Latency

**Symptoms:**
- P99 latency >10ms
- Message delays
- Slow channel broadcasts

**Diagnosis:**
```bash
# Check STATS
/STATS l  # Look for sendq buildup
/STATS m  # Check command distribution

# Profile with perf
perf record -e cycles:pp -g ./target/release/rustircd
```

**Solutions:**
- Increase sendq/recvq sizes
- Enable burst optimization
- Check network conditions
- Review throttling settings
- Optimize channel sizes

### Connection Drops

**Symptoms:**
- Frequent disconnections
- Timeout errors
- High connection churn

**Diagnosis:**
```bash
# Check logs
grep "timeout\|disconnect" rustircd.log

# Monitor connections
netstat -an | grep :6667 | wc -l
watch -n 1 'netstat -an | grep :6667 | wc -l'
```

**Solutions:**
- Increase connection timeout
- Adjust ping frequency
- Check network stability
- Review buffer sizes
- Disable aggressive throttling

### Slow Server Burst

**Symptoms:**
- Server linking takes >10s
- High burst CPU usage
- Delayed network sync

**Diagnosis:**
```bash
# Check burst size
grep "Server burst" rustircd.log

# Profile burst processing
perf record -e cycles -g ./target/release/rustircd
```

**Solutions:**
- Enable burst optimization
- Increase burst timeout
- Check network bandwidth
- Review user count
- Optimize database queries

### Memory Leaks

**Symptoms:**
- Memory grows indefinitely
- No correlation with user count
- Eventually crashes

**Diagnosis:**
```bash
# Run leak detection
python tests/stability/memory_leak_test.py --duration 3600

# Use valgrind
valgrind --leak-check=full ./target/release/rustircd
```

**Solutions:**
- Update to latest version
- Report issue with leak test results
- Check for cyclic references
- Review custom modules
- Monitor with heaptrack

## Performance Checklist

### Pre-Production

- [ ] Run full benchmark suite
- [ ] Load test with expected traffic +50%
- [ ] 24-hour stability test
- [ ] Memory leak detection
- [ ] Profile CPU hotspots
- [ ] Test netsplit recovery
- [ ] Verify latency targets
- [ ] Test with production config

### Production Monitoring

- [ ] CPU usage < 70% at peak
- [ ] Memory usage stable
- [ ] Latency P99 < 5ms
- [ ] No connection drops
- [ ] Sendq/recvq not saturated
- [ ] No error log growth
- [ ] Successful netsplit recovery
- [ ] Cache hit rates >80%

### Regular Maintenance

- [ ] Review STATS monthly
- [ ] Benchmark against baseline quarterly
- [ ] Update performance targets
- [ ] Profile after major changes
- [ ] Test at scale before deploy
- [ ] Monitor resource trends
- [ ] Capacity planning
- [ ] Performance regression testing

## Performance Regression Testing

### Automated Testing

```bash
# Compare against baseline
./scripts/bench-compare.sh --threshold 5

# Fail if >5% regression
# Generate comparison report
# Email results to team
```

### Manual Testing

```bash
# Save baseline
cargo bench --save-baseline production-v1.0

# After changes
cargo bench --baseline production-v1.0

# Review differences
criterion-compare production-v1.0 current
```

## Best Practices

1. **Measure First**: Profile before optimizing
2. **Set Targets**: Define acceptable performance levels
3. **Test Regularly**: Continuous benchmarking
4. **Monitor Production**: Track metrics in real-time
5. **Capacity Planning**: Test beyond expected load
6. **Document Changes**: Track performance impact
7. **Optimize Hot Paths**: Focus on high-frequency operations
8. **Balance Tradeoffs**: Memory vs CPU, latency vs throughput
9. **Stay Updated**: Keep dependencies current
10. **Test Realistic Workloads**: Synthetic benchmarks aren't enough

## Additional Resources

- **Criterion Documentation**: https://bheisler.github.io/criterion.rs/
- **Tokio Performance**: https://tokio.rs/tokio/topics/performance
- **Rust Performance Book**: https://nnethercote.github.io/perf-book/
- **FlameGraph Guide**: http://www.brendangregg.com/flamegraphs.html
- **Linux Performance Tools**: http://www.brendangregg.com/linuxperf.html

## Contributing

When optimizing RustIRCd:

1. Benchmark before and after changes
2. Document performance impact in PR
3. Add benchmarks for new features
4. Test at scale
5. Profile hot paths
6. Consider multiple scenarios
7. Update this guide as needed

---

**Last Updated**: January 2025
**Targets Verified**: January 2025

