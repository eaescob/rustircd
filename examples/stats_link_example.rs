//! Example demonstrating STATS L command with sendq/recvq statistics
//!
//! This example shows how the STATS L command displays detailed server link information
//! including send queue and receive queue statistics.
//!
//! Run with: cargo run --example stats_link_example

use rustircd_core::server_connection::ServerConnectionStats;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== STATS L Command - Server Link Statistics Example ===\n");

    // Example 1: Server with low sendq usage
    println!("Example 1: Hub server with normal operation");
    println!("{}", "-".repeat(80));
    
    let mut stats1 = ServerConnectionStats::default();
    stats1.sendq_current = 1024;        // 1KB used
    stats1.sendq_max = 10485760;        // 10MB max
    stats1.sendq_dropped = 0;           // No drops
    stats1.recvq_current = 512;         // 512B used
    stats1.recvq_max = 32768;           // 32KB max
    stats1.messages_sent = 1543;
    stats1.messages_received = 1892;
    stats1.bytes_sent = 245678;
    stats1.bytes_received = 389012;
    
    display_stats("hub.example.com", &stats1, 3600);
    println!();

    // Example 2: Server with high sendq usage (congestion)
    println!("Example 2: Leaf server with congestion");
    println!("{}", "-".repeat(80));
    
    let mut stats2 = ServerConnectionStats::default();
    stats2.sendq_current = 9437184;     // ~9MB used (90% capacity)
    stats2.sendq_max = 10485760;        // 10MB max
    stats2.sendq_dropped = 127;         // Dropped messages!
    stats2.recvq_current = 28000;       // 28KB used (85% capacity)
    stats2.recvq_max = 32768;           // 32KB max
    stats2.messages_sent = 98234;
    stats2.messages_received = 102341;
    stats2.bytes_sent = 12456789;
    stats2.bytes_received = 15678901;
    
    display_stats("congested.example.com", &stats2, 7200);
    println!();
    
    // Example 3: Idle server connection
    println!("Example 3: Idle server connection");
    println!("{}", "-".repeat(80));
    
    let mut stats3 = ServerConnectionStats::default();
    stats3.sendq_current = 0;
    stats3.sendq_max = 10485760;
    stats3.sendq_dropped = 0;
    stats3.recvq_current = 0;
    stats3.recvq_max = 32768;
    stats3.messages_sent = 50;
    stats3.messages_received = 52;
    stats3.bytes_sent = 5432;
    stats3.bytes_received = 5891;
    
    display_stats("idle.example.com", &stats3, 86400);
    println!();

    println!("\n=== Understanding the Output ===\n");
    println!("SendQ (Send Queue):");
    println!("  - Current: Bytes currently buffered for sending");
    println!("  - Max: Maximum buffer capacity (from connection class)");
    println!("  - Percentage: Usage percentage (high % = congestion)");
    println!();
    println!("RecvQ (Receive Queue):");
    println!("  - Current: Bytes currently buffered from receiving");
    println!("  - Max: Maximum buffer capacity (from connection class)");
    println!("  - Percentage: Usage percentage");
    println!();
    println!("Msgs (Messages):");
    println!("  - s: Messages sent to this server");
    println!("  - r: Messages received from this server");
    println!();
    println!("Bytes:");
    println!("  - s: Bytes sent to this server");
    println!("  - r: Bytes received from this server");
    println!();
    println!("Time:");
    println!("  - Seconds this server has been connected");
    println!();
    println!("Dropped:");
    println!("  - Number of messages dropped due to sendq being full");
    println!("  - Non-zero value indicates serious congestion!");
    println!();

    println!("\n=== Interpreting Results ===\n");
    println!("✓ Good: SendQ < 50%, no dropped messages");
    println!("⚠ Warning: SendQ 50-90%, monitor for congestion");
    println!("✗ Critical: SendQ > 90% or any dropped messages");
    println!("    Action: Check network, increase sendq_max, or investigate server load");

    Ok(())
}

fn display_stats(server_name: &str, stats: &ServerConnectionStats, time_online: u64) {
    let sendq_percent = if stats.sendq_max > 0 {
        (stats.sendq_current as f32 / stats.sendq_max as f32 * 100.0) as u32
    } else {
        0
    };
    
    let recvq_percent = if stats.recvq_max > 0 {
        (stats.recvq_current as f32 / stats.recvq_max as f32 * 100.0) as u32
    } else {
        0
    };
    
    // Format like actual STATS L output
    let output = format!(
        "{} SendQ:{}/{}({}%) RecvQ:{}/{}({}%) Msgs:{}s/{}r Bytes:{}s/{}r Time:{}s Dropped:{}",
        server_name,
        stats.sendq_current, stats.sendq_max, sendq_percent,
        stats.recvq_current, stats.recvq_max, recvq_percent,
        stats.messages_sent, stats.messages_received,
        stats.bytes_sent, stats.bytes_received,
        time_online,
        stats.sendq_dropped
    );
    
    println!("{}", output);
    
    // Add interpretation
    if stats.sendq_dropped > 0 {
        println!("  ✗ CRITICAL: {} messages dropped due to buffer full!", stats.sendq_dropped);
    } else if sendq_percent >= 90 {
        println!("  ⚠ WARNING: SendQ at {}% capacity - approaching limit!", sendq_percent);
    } else if sendq_percent >= 50 {
        println!("  ⚠ NOTICE: SendQ at {}% capacity - monitor for issues", sendq_percent);
    } else {
        println!("  ✓ OK: SendQ at {}% capacity - normal operation", sendq_percent);
    }
    
    // Time online interpretation
    let hours = time_online / 3600;
    let minutes = (time_online % 3600) / 60;
    println!("  Connected for: {}h {}m", hours, minutes);
}

