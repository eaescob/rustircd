//! Example demonstrating STATS M command with enhanced command statistics
//!
//! This example shows how the STATS M command displays command usage statistics
//! including total count, average bytes per command, and remote vs local tracking.
//!
//! Run with: cargo run --example stats_commands_example

use rustircd_core::{ServerStatistics, CommandStats};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== STATS M Command - Command Usage Statistics Example ===\n");

    // Create a sample statistics instance
    let mut stats = ServerStatistics::new();
    
    // Simulate various command usage patterns
    
    // PRIVMSG - high usage, mixed local and remote
    for i in 0..150 {
        let is_remote = i % 3 == 0; // 1/3 from remote
        stats.record_message_received("PRIVMSG", 80 + (i % 20) as usize, is_remote);
    }
    
    // JOIN - moderate usage, mostly local
    for i in 0..75 {
        let is_remote = i % 5 == 0; // 1/5 from remote
        stats.record_message_received("JOIN", 45 + (i % 10) as usize, is_remote);
    }
    
    // PART - moderate usage, mostly local
    for i in 0..60 {
        let is_remote = i % 6 == 0; // 1/6 from remote
        stats.record_message_received("PART", 50 + (i % 10) as usize, is_remote);
    }
    
    // MODE - lower usage, mixed
    for i in 0..40 {
        let is_remote = i % 2 == 0; // 1/2 from remote
        stats.record_message_received("MODE", 35 + (i % 15) as usize, is_remote);
    }
    
    // NICK - lower usage, mostly local (initial registration)
    for i in 0..30 {
        let is_remote = i % 10 == 0; // 1/10 from remote
        stats.record_message_received("NICK", 25 + (i % 5) as usize, is_remote);
    }
    
    // TOPIC - lower usage
    for i in 0..20 {
        let is_remote = i % 4 == 0; // 1/4 from remote
        stats.record_message_received("TOPIC", 60 + (i % 20) as usize, is_remote);
    }
    
    // QUIT - moderate usage
    for i in 0..45 {
        let is_remote = i % 3 == 0; // 1/3 from remote
        stats.record_message_received("QUIT", 40 + (i % 10) as usize, is_remote);
    }
    
    // WHO - moderate usage, mostly local
    for i in 0..35 {
        let is_remote = i % 7 == 0; // 1/7 from remote
        stats.record_message_received("WHO", 30 + (i % 10) as usize, is_remote);
    }
    
    // WHOIS - moderate usage, mostly local
    for i in 0..50 {
        let is_remote = i % 5 == 0; // 1/5 from remote
        stats.record_message_received("WHOIS", 35 + (i % 10) as usize, is_remote);
    }
    
    // PING - high usage, mostly remote (server keepalives)
    for i in 0..100 {
        let is_remote = i % 2 == 1; // 1/2 from remote
        stats.record_message_received("PING", 20, is_remote);
    }

    // Display statistics
    println!("Top 10 Commands by Usage:");
    println!("{}", "=".repeat(100));
    println!("{:<15} {:>10} {:>12} {:>12} {:>15} {:>15}", 
        "Command", "Total", "Local", "Remote", "Avg Bytes", "Total Bytes");
    println!("{}", "-".repeat(100));
    
    let top_commands = stats.get_top_commands(10);
    
    for (command, cmd_stats) in top_commands {
        let avg_bytes = if cmd_stats.total_count() > 0 {
            cmd_stats.total_bytes / cmd_stats.total_count()
        } else {
            0
        };
        
        println!("{:<15} {:>10} {:>12} {:>12} {:>15} {:>15}",
            command,
            cmd_stats.total_count(),
            cmd_stats.local_count,
            cmd_stats.remote_count,
            avg_bytes,
            cmd_stats.total_bytes
        );
    }

    println!("\n{}", "=".repeat(100));
    println!("\nOverall Statistics:");
    println!("  Total Messages: {}", stats.total_messages_received);
    println!("  Total Bytes: {}", stats.total_bytes_received);
    println!("  Average Bytes per Message: {}", 
        if stats.total_messages_received > 0 {
            stats.total_bytes_received / stats.total_messages_received
        } else {
            0
        }
    );

    println!("\n{}", "=".repeat(100));
    println!("\n=== STATS M IRC Output Format ===\n");
    println!("In IRC, the STATS M command would return:");
    println!();
    
    for (command, cmd_stats) in stats.get_top_commands(10).iter().take(5) {
        let avg_bytes = if cmd_stats.total_count() > 0 {
            (cmd_stats.total_bytes / cmd_stats.total_count()) as u32
        } else {
            0
        };
        
        println!(":server.name 212 yournick {} {} {} {}",
            command,
            cmd_stats.total_count(),
            avg_bytes,
            cmd_stats.remote_count
        );
    }
    println!(":server.name 219 yournick m :End of STATS report");

    println!("\n{}", "=".repeat(100));
    println!("\n=== Understanding the Output ===\n");
    println!("Format: <command> <total_count> <avg_bytes> <remote_count>");
    println!();
    println!("  • total_count: Total number of times this command was executed");
    println!("  • avg_bytes: Average message size in bytes");
    println!("  • remote_count: How many came from remote servers");
    println!();
    println!("High remote_count indicates:");
    println!("  - Command is being propagated across the network");
    println!("  - Could indicate server-to-server traffic patterns");
    println!();
    println!("High avg_bytes with high count:");
    println!("  - Commands consuming significant bandwidth");
    println!("  - May need optimization or rate limiting");

    Ok(())
}

