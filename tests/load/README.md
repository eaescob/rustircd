# RustIRCd Load Testing Scripts

This directory contains load testing scripts for RustIRCd performance testing.

## Prerequisites

Python 3.7 or higher is required. No additional packages needed (uses only standard library).

## Available Tests

### 1. Connection Stress Test (`connection_stress.py`)

Tests the server's ability to handle many concurrent connections.

```bash
./connection_stress.py --host localhost --port 6667 --clients 1000 --rate 50
```

**Options:**
- `--host`: IRC server hostname (default: localhost)
- `--port`: IRC server port (default: 6667)
- `--clients`: Number of concurrent clients (default: 100)
- `--rate`: Connections per second (default: 50)

**Metrics:**
- Total connections established
- Connection success/failure rate
- Average connection time
- Connections per second

**Example Output:**
```
Progress: 1000/1000 (50.2 conn/sec, 998 success, 2 failed)

==========================================================
RESULTS
==========================================================
Total clients:      1000
Successful:         998
Failed:             2
Total time:         19.87s
Connections/sec:    50.2
Avg connect time:   145.3ms
```

### 2. Message Throughput Test (`message_throughput.py`)

Tests message processing capacity and latency.

```bash
./message_throughput.py --host localhost --port 6667 --rate 10000 --duration 60
```

**Options:**
- `--host`: IRC server hostname (default: localhost)
- `--port`: IRC server port (default: 6667)
- `--rate`: Messages per second (default: 1000)
- `--duration`: Test duration in seconds (default: 60)

**Metrics:**
- Messages sent/failed
- Actual message rate achieved
- Latency statistics (min, max, mean, median, stdev, percentiles)

**Example Output:**
```
Sent: 60000, Rate: 1000/sec, Failed: 0

==========================================================
RESULTS
==========================================================
Messages sent:      60000
Messages failed:    0
Total time:         60.05s
Actual rate:        999.2/sec

Latency statistics:
  Min:     0.15ms
  Max:     8.32ms
  Mean:    0.42ms
  Median:  0.38ms
  Stdev:   0.28ms
  P50:     0.38ms
  P95:     0.95ms
  P99:     1.42ms
```

### 3. Channel Load Test (`channel_load.py`)

Tests channel-specific performance with varying channel sizes.

```bash
./channel_load.py --channels 50 --max-users 500 --duration 180
```

**Options:**
- `--host`: IRC server hostname (default: localhost)
- `--port`: IRC server port (default: 6667)
- `--channels`: Number of channels (default: 20)
- `--max-users`: Maximum users per channel (default: 100)
- `--duration`: Test duration in seconds (default: 180)

**Metrics:**
- Channel broadcast latency by size
- JOIN performance
- Message distribution time
- Success rate

**Example Output:**
```
Channel Size         Count      Min        Avg        P50        P95        P99        Max       
--------------------------------------------------------------------------------
small (10-50)        150        0.12       0.45       0.42       0.85       1.20       2.10
medium (50-200)      100        0.25       0.78       0.75       1.50       2.10       3.50
large (200-1000)     50         0.45       1.25       1.20       2.80       4.50       6.20
```

### 4. Mixed Workload Test (`mixed_workload.py`)

Simulates realistic IRC traffic patterns with mixed operations.

```bash
./mixed_workload.py --users 100 --channels 20 --duration 300
```

**Options:**
- `--host`: IRC server hostname (default: localhost)
- `--port`: IRC server port (default: 6667)
- `--users`: Number of users (default: 50)
- `--channels`: Number of channels (default: 10)
- `--duration`: Test duration in seconds (default: 300)

**Traffic Distribution:**
- 70% channel messages
- 20% private messages
- 5% joins/parts
- 3% mode changes
- 2% operator commands

**Metrics:**
- Operations per second
- Success rate
- Traffic distribution validation

**Example Output:**
```
Duration:            300.05s
Active clients:      98/100

Message Statistics:
  Channel messages:  21000
  Private messages:  6000
  Joins:             750
  Parts:             750

Traffic Distribution:
  Channel messages:  70.2%
  Private messages:  20.1%
  Joins/Parts:       5.0%

Operations/second:   100.5
Success rate:        99.8%
```

## Running Tests

### Basic Usage

1. **Start RustIRCd server:**
   ```bash
   cd /path/to/rustircd
   cargo run --release
   ```

2. **Run connection stress test:**
   ```bash
   cd tests/load
   ./connection_stress.py --clients 500
   ```

3. **Run throughput test:**
   ```bash
   ./message_throughput.py --rate 5000 --duration 30
   ```

### Progressive Load Testing

Test with increasing load to find limits:

```bash
# Start with low load
./connection_stress.py --clients 100
./message_throughput.py --rate 1000 --duration 30

# Increase gradually
./connection_stress.py --clients 500
./message_throughput.py --rate 5000 --duration 30

./connection_stress.py --clients 1000
./message_throughput.py --rate 10000 --duration 30

# Find maximum capacity
./connection_stress.py --clients 5000
./message_throughput.py --rate 50000 --duration 30
```

### Monitoring During Tests

While tests are running, monitor server performance:

```bash
# CPU and memory usage
top -p $(pgrep rustircd)

# Network statistics
netstat -an | grep :6667 | wc -l  # Count connections

# Server statistics (via IRC client)
/STATS m   # Command statistics
/STATS l   # Server link statistics
```

## Interpreting Results

### Connection Test Results

**Good Performance:**
- Success rate: >99%
- Connections/sec: Match target rate
- Avg connect time: <200ms

**Issues to Investigate:**
- High failure rate: Check connection class limits, file descriptors
- Low conn/sec: Check throttling, system limits
- High connect time: Check DNS resolution, network latency

### Throughput Test Results

**Good Performance:**
- Message rate: Match target rate (±5%)
- P50 latency: <1ms
- P99 latency: <5ms
- No failures

**Issues to Investigate:**
- Low message rate: Check CPU usage, message batching settings
- High latency: Check buffer sizes, network conditions
- High failure rate: Check buffer overflows, timeouts

## Performance Targets

Based on the PERFORMANCE.md documentation:

### Connections
- **Target**: 10,000+ concurrent connections
- **Memory**: ~10KB per connection
- **Accept rate**: 100+ connections/second

### Messages
- **Throughput**: 100,000+ messages/second
- **Latency P50**: <1ms
- **Latency P99**: <5ms

### Channels
- **Broadcast**: <10ms for 1000-member channel
- **Join/Part**: <1ms for <1000 members

## Troubleshooting

### "Connection refused" errors

Server might not be running or listening on wrong port:
```bash
# Check if server is running
ps aux | grep rustircd

# Check listening ports
netstat -tulpn | grep 6667
```

### "Too many open files" errors

Increase file descriptor limits:
```bash
# Temporary (current session)
ulimit -n 65535

# Permanent (add to /etc/security/limits.conf)
* soft nofile 65535
* hard nofile 65535
```

### Connection timeouts

- Increase timeout in scripts
- Check firewall rules
- Verify server is not overloaded
- Check throttling settings in server config

### Memory issues

Monitor memory usage:
```bash
# Memory usage of rustircd process
ps aux | grep rustircd | awk '{print $6/1024 " MB"}'

# System memory
free -h
```

## Custom Tests

You can modify these scripts or create new ones. Key IRC commands for testing:

```python
# Connection
socket.send(b"NICK testnick\r\n")
socket.send(b"USER testuser 0 * :Test User\r\n")

# Messages
socket.send(b"PRIVMSG #channel :Hello world\r\n")
socket.send(b"NOTICE testnick :Notice message\r\n")

# Channels
socket.send(b"JOIN #channel\r\n")
socket.send(b"PART #channel :Goodbye\r\n")

# Cleanup
socket.send(b"QUIT :Test complete\r\n")
```

## CI/CD Integration

These scripts can be integrated into CI/CD pipelines:

```yaml
# Example GitHub Actions workflow
- name: Start IRC server
  run: cargo run --release &
  
- name: Wait for server
  run: sleep 5
  
- name: Run load tests
  run: |
    ./tests/load/connection_stress.py --clients 100
    ./tests/load/message_throughput.py --rate 1000 --duration 10
```

## Contributing

When adding new load tests:

1. Follow the existing script structure
2. Include comprehensive metrics
3. Add documentation to this README
4. Test with various load levels
5. Verify resource cleanup (connections, threads)

## Support

For issues or questions:
1. Check server logs: Look for errors or warnings
2. Review PERFORMANCE.md: Compare with expected performance
3. Open an issue: Include test output and server logs


