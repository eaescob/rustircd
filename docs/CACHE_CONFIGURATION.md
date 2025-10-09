# Cache Configuration Guide

## Overview

The rustircd server includes a high-performance caching system that can be configured via the `config.toml` file. This guide explains the available cache settings and how to tune them for your network.

## Configuration Options

### Location

Cache settings are located in the `[database]` section of your `config.toml`:

```toml
[database]
max_history_size = 10000
history_retention_days = 30
enable_channel_tracking = true
enable_activity_tracking = true

# Cache configuration (optional)
user_cache_size = 10000              # Max cached nickname→UUID mappings
user_cache_ttl_seconds = 300         # 5 minutes user cache TTL
channel_cache_ttl_seconds = 30       # 30 seconds channel member cache TTL
```

### Cache Parameters

#### `user_cache_size` (Optional)

**Type:** Integer  
**Default:** `10000`  
**Description:** Maximum number of nickname→UUID mappings to cache in memory.

**What it does:**
- Caches the most recently used user lookups
- Uses LRU (Least Recently Used) eviction when capacity is reached
- Improves user lookup performance by 10-100x

**Tuning guidelines:**
- Set to ~1.5x your peak concurrent user count
- Minimum recommended: 1000
- Maximum practical: 1,000,000 (consumes ~100MB memory)

**Examples:**
```toml
# Small network (<1000 users)
user_cache_size = 10000

# Medium network (1000-5000 users)
user_cache_size = 25000

# Large network (5000-10000 users)
user_cache_size = 50000

# Very large network (>10000 users)
user_cache_size = 100000
```

#### `user_cache_ttl_seconds` (Optional)

**Type:** Integer (seconds)  
**Default:** `300` (5 minutes)  
**Description:** How long cached user entries remain valid before expiring.

**What it does:**
- Automatically removes stale cache entries
- Balances memory usage vs cache hit rate
- Ensures cache doesn't serve very old data

**Tuning guidelines:**
- Higher TTL = better cache hit rate, more memory usage
- Lower TTL = lower cache hit rate, less memory usage
- Should be longer than average user session query frequency

**Examples:**
```toml
# Aggressive caching (high hit rate)
user_cache_ttl_seconds = 900    # 15 minutes

# Balanced (default)
user_cache_ttl_seconds = 300    # 5 minutes

# Conservative (lower memory)
user_cache_ttl_seconds = 120    # 2 minutes
```

#### `channel_cache_ttl_seconds` (Optional)

**Type:** Integer (seconds)  
**Default:** `30` (30 seconds)  
**Description:** How long cached channel member lists remain valid before expiring.

**What it does:**
- Caches channel member lists for fast queries
- Automatically invalidated when members join/part
- Improves channel member query performance by 5-50x

**Tuning guidelines:**
- Shorter TTL than user cache (channels change more frequently)
- Should be longer than burst operation duration
- Balance between freshness and cache hits

**Examples:**
```toml
# Longer caching (stable channels)
channel_cache_ttl_seconds = 60   # 1 minute

# Balanced (default)
channel_cache_ttl_seconds = 30   # 30 seconds

# Shorter caching (dynamic channels)
channel_cache_ttl_seconds = 15   # 15 seconds
```

## Default Configuration

If you don't specify cache settings, these defaults are used:

```toml
[database]
# ... other settings ...

# Implicit defaults (no need to specify):
# user_cache_size = 10000
# user_cache_ttl_seconds = 300
# channel_cache_ttl_seconds = 30
```

## Network Size Recommendations

### Small Networks (< 1000 users)

**Use default settings** - they're optimized for this size:

```toml
[database]
max_history_size = 10000
history_retention_days = 30
enable_channel_tracking = true
enable_activity_tracking = true
# Cache settings: Use defaults (omit or comment out)
```

**Expected performance:**
- User lookups: 1-5μs (cached)
- Channel queries: 2-10μs (cached)
- Cache hit rate: 95-99%
- Memory overhead: ~10-20MB

### Medium Networks (1000-5000 users)

**Increase cache size and TTL:**

```toml
[database]
max_history_size = 10000
history_retention_days = 30
enable_channel_tracking = true
enable_activity_tracking = true

# Cache configuration for medium networks
user_cache_size = 25000
user_cache_ttl_seconds = 600        # 10 minutes
channel_cache_ttl_seconds = 60      # 1 minute
```

**Expected performance:**
- User lookups: 1-5μs (cached)
- Channel queries: 2-10μs (cached)
- Cache hit rate: 97-99%
- Memory overhead: ~25-50MB

### Large Networks (5000-10000 users)

**Further increase cache capacity:**

```toml
[database]
max_history_size = 10000
history_retention_days = 30
enable_channel_tracking = true
enable_activity_tracking = true

# Cache configuration for large networks
user_cache_size = 50000
user_cache_ttl_seconds = 900        # 15 minutes
channel_cache_ttl_seconds = 90      # 1.5 minutes
```

**Expected performance:**
- User lookups: 1-5μs (cached)
- Channel queries: 2-10μs (cached)
- Cache hit rate: 98-99%
- Memory overhead: ~50-100MB

### Very Large Networks (> 10000 users)

**Maximum caching configuration:**

```toml
[database]
max_history_size = 10000
history_retention_days = 30
enable_channel_tracking = true
enable_activity_tracking = true

# Cache configuration for very large networks
user_cache_size = 100000
user_cache_ttl_seconds = 1800       # 30 minutes
channel_cache_ttl_seconds = 120     # 2 minutes
```

**Expected performance:**
- User lookups: 1-5μs (cached)
- Channel queries: 2-10μs (cached)
- Cache hit rate: 99%+
- Memory overhead: ~100-200MB

## Memory Usage Estimation

### User Cache Memory Usage

Each cached entry stores:
- Nickname string (~20 bytes)
- UUID (16 bytes)
- Cache metadata (~50 bytes)

**Approximate calculation:**
```
Memory (MB) = user_cache_size * 86 bytes / 1,048,576
```

**Examples:**
- 10,000 entries ≈ 0.8 MB
- 25,000 entries ≈ 2.0 MB
- 50,000 entries ≈ 4.1 MB
- 100,000 entries ≈ 8.2 MB

### Channel Cache Memory Usage

Channel cache is unlimited size but auto-expires. Memory usage depends on:
- Number of channels
- Average members per channel
- Query frequency

**Typical usage:**
- Small network: 1-5 MB
- Medium network: 5-20 MB
- Large network: 20-50 MB

## Performance Monitoring

### Using Server Statistics

Monitor cache performance using the database statistics API:

```rust
let user_stats = database.get_user_cache_stats();
let channel_stats = database.get_channel_cache_stats();

println!("User Cache:");
println!("  Size: {}/{}", user_stats.size, user_stats.capacity);
println!("  Hits: {}", user_stats.total_hits);

println!("Channel Cache:");
println!("  Size: {}", channel_stats.size);
println!("  Hits: {}", channel_stats.total_hits);
```

### Ideal Metrics

For optimal performance, aim for:
- **User cache hit rate**: > 95%
- **User cache utilization**: 50-80% of capacity
- **Channel cache hit rate**: > 90%

### Signs of Poor Configuration

**Cache too small:**
- User cache size near capacity
- Cache hit rate < 90%
- Frequent LRU evictions

**Solution:** Increase `user_cache_size`

**TTL too short:**
- Low cache hit rate despite adequate size
- Cache size much smaller than capacity

**Solution:** Increase TTL settings

**TTL too long:**
- High memory usage
- Cache size near capacity

**Solution:** Decrease TTL settings

## Tuning for Burst Operations

During server bursting (when servers synchronize state), cache behavior is important.

### Burst-Optimized Settings

```toml
[database]
# ... other settings ...

# Larger cache to handle burst
user_cache_size = 50000

# Longer TTL to keep burst users cached
user_cache_ttl_seconds = 600

# Shorter channel TTL (invalidated during burst anyway)
channel_cache_ttl_seconds = 30
```

### Why This Works

- **User burst** populates cache automatically
- **Large cache size** prevents eviction during burst
- **Long TTL** keeps burst users cached after sync
- **Short channel TTL** doesn't matter (invalidated anyway)

See [CACHE_BURST_COMPATIBILITY.md](CACHE_BURST_COMPATIBILITY.md) for detailed burst behavior.

## Advanced Configuration

### Disabling Cache

To disable caching (not recommended):

```toml
[database]
# ... other settings ...

# Effectively disable caching
user_cache_size = 1                # Minimum size
user_cache_ttl_seconds = 1         # Immediate expiration
channel_cache_ttl_seconds = 1
```

**Warning:** This will significantly reduce performance!

### Dynamic Configuration

Cache settings are loaded at startup. To change settings:

1. Edit `config.toml`
2. Restart the server or use REHASH command (if supported)

**Note:** Some configurations support runtime cache clearing:
```rust
database.clear_all_caches();
```

## Troubleshooting

### Issue: Low Cache Hit Rate

**Symptoms:**
- Cache hits < 90%
- Slower than expected performance

**Solutions:**
1. Increase `user_cache_size`
2. Increase `user_cache_ttl_seconds`
3. Check for unusual access patterns

### Issue: High Memory Usage

**Symptoms:**
- Server using too much memory
- Cache size near capacity

**Solutions:**
1. Decrease `user_cache_size`
2. Decrease TTL values
3. Manually clear cache periodically

### Issue: Stale Data

**Symptoms:**
- Users report seeing old data
- Cache showing disconnected users

**Solutions:**
1. Check cache invalidation is working
2. Decrease TTL values
3. Verify server code properly invalidates cache

## Best Practices

1. **Start with defaults** - They're optimized for most networks
2. **Monitor cache stats** - Make data-driven tuning decisions
3. **Tune gradually** - Change one parameter at a time
4. **Test under load** - Verify changes during peak times
5. **Document changes** - Note why you changed settings

## Configuration Examples

### Development Server

```toml
[database]
max_history_size = 1000
history_retention_days = 7
enable_channel_tracking = true
enable_activity_tracking = true

# Small cache for development
user_cache_size = 1000
user_cache_ttl_seconds = 120
channel_cache_ttl_seconds = 30
```

### Production Server

```toml
[database]
max_history_size = 10000
history_retention_days = 30
enable_channel_tracking = true
enable_activity_tracking = true

# Optimized for production
user_cache_size = 25000
user_cache_ttl_seconds = 600
channel_cache_ttl_seconds = 60
```

### High-Performance Server

```toml
[database]
max_history_size = 10000
history_retention_days = 30
enable_channel_tracking = true
enable_activity_tracking = true

# Maximum performance
user_cache_size = 100000
user_cache_ttl_seconds = 1800
channel_cache_ttl_seconds = 120
```

## Summary

Cache configuration provides fine-grained control over performance and memory usage:

- **`user_cache_size`**: Controls how many users to cache (default: 10000)
- **`user_cache_ttl_seconds`**: Controls cache freshness (default: 300)
- **`channel_cache_ttl_seconds`**: Controls channel cache TTL (default: 30)

For most servers, **default settings work well**. Tune based on monitoring and network size.

For more information:
- [CACHE_INTEGRATION.md](CACHE_INTEGRATION.md) - General cache usage
- [CACHE_BURST_COMPATIBILITY.md](CACHE_BURST_COMPATIBILITY.md) - Burst behavior
- [CACHE_IMPLEMENTATION_SUMMARY.md](../CACHE_IMPLEMENTATION_SUMMARY.md) - Technical details

