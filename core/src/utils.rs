//! Utility functions and helpers

use crate::Error;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

/// DNS and network utilities
pub mod dns {
    use super::*;
    use std::net::ToSocketAddrs;
    use tokio::net::lookup_host;
    
    /// Resolve hostname to IP address
    pub async fn resolve_hostname(hostname: &str) -> Result<Option<IpAddr>, Box<dyn std::error::Error + Send + Sync>> {
        let addrs: Vec<_> = lookup_host(hostname).await
            .map_err(|e| Error::Generic(format!("DNS lookup failed: {}", e)))?
            .collect();
        
        addrs.first().map(|addr| Ok(addr.ip())).transpose()
    }
    
    /// Reverse DNS lookup
    pub async fn reverse_lookup(ip: IpAddr) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        // This is a simplified implementation
        // In a real implementation, you'd use a proper reverse DNS lookup
        match ip {
            IpAddr::V4(ipv4) => {
                if ipv4.is_private() {
                    Ok(Some(format!("{}.local", ipv4)))
                } else {
                    // For public IPs, you'd do a real reverse DNS lookup
                    Ok(Some(format!("{}.in-addr.arpa", ipv4)))
                }
            }
            IpAddr::V6(ipv6) => {
                if ipv6.is_unicast_link_local() {
                    Ok(Some(format!("{}.local", ipv6)))
                } else {
                    // For public IPv6s, you'd do a real reverse DNS lookup
                    Ok(Some(format!("{}.ip6.arpa", ipv6)))
                }
            }
        }
    }
}

/// Ident protocol utilities
pub mod ident {
    use super::*;
    use std::net::{TcpStream, SocketAddr};
    use std::io::{Read, Write};
    use tokio::io::{AsyncWriteExt, AsyncBufReadExt};
    use std::time::Duration;
    
    /// Perform ident lookup
    pub async fn lookup_ident(local_port: u16, remote_port: u16, remote_addr: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let ident_addr = format!("{}:113", remote_addr);
        
        // Try to connect to ident port with timeout
        let stream = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::net::TcpStream::connect(&ident_addr)
        ).await
        .map_err(|_| Error::Generic("Ident connection timeout".to_string()))?
        .map_err(|e| Error::Generic(format!("Failed to connect to ident port: {}", e)))?;
        
        // Send ident request
        let request = format!("{}:{}\r\n", remote_port, local_port);
        let mut stream = tokio::io::BufWriter::new(stream);
        
        stream.write_all(request.as_bytes()).await
            .map_err(|e| Error::Generic(format!("Failed to send ident request: {}", e)))?;
        
        stream.flush().await
            .map_err(|e| Error::Generic(format!("Failed to flush ident request: {}", e)))?;
        
        // Read response
        let mut response = String::new();
        let mut reader = tokio::io::BufReader::new(stream);
        
        tokio::time::timeout(
            Duration::from_secs(5),
            reader.read_line(&mut response)
        ).await
        .map_err(|_| Error::Generic("Ident response timeout".to_string()))?
        .map_err(|e| Error::Generic(format!("Failed to read ident response: {}", e)))?;
        
        // Parse response
        let response = response.trim();
        if response.is_empty() {
            return Ok(None);
        }
        
        // Parse format: port,port:userid
        if let Some(colon_pos) = response.find(':') {
            let userid = &response[colon_pos + 1..].trim();
            if !userid.is_empty() && *userid != "ERROR" {
                return Ok(Some(userid.to_string()));
            }
        }
        
        Ok(None)
    }
}

/// String utilities
pub mod string {
    use super::*;
    
    /// Check if a string is a valid IRC channel name
    pub fn is_valid_channel_name(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        
        let first_char = name.chars().next().unwrap();
        if !"#&+!".contains(first_char) {
            return false;
        }
        
        // Channel name should not contain spaces or control characters
        name.chars().all(|c| c.is_ascii() && !c.is_control() && c != ' ' && c != ',' && c != ':')
    }
    
    /// Check if a string is a valid IRC nickname
    pub fn is_valid_nickname(nick: &str, max_length: usize) -> bool {
        if nick.is_empty() || nick.len() > max_length {
            return false;
        }
        
        let chars: Vec<char> = nick.chars().collect();
        
        // First character must be letter or special character
        let first_char = chars[0];
        if !first_char.is_ascii_alphabetic() && !"[]\\`_^{|}~".contains(first_char) {
            return false;
        }
        
        // Remaining characters must be letter, digit, or special character
        for &c in &chars[1..] {
            if !c.is_ascii_alphanumeric() && !"-[]\\`_^{|}~".contains(c) {
                return false;
            }
        }
        
        true
    }
    
    /// Check if a string is a valid IRC username
    pub fn is_valid_username(username: &str) -> bool {
        if username.is_empty() || username.len() > 9 {
            return false;
        }
        
        // Username should not contain spaces or control characters
        username.chars().all(|c| c.is_ascii() && !c.is_control() && c != ' ' && c != '@')
    }
    
    /// Check if a string is a valid IRC hostname
    pub fn is_valid_hostname(hostname: &str) -> bool {
        if hostname.is_empty() {
            return false;
        }
        
        // Basic hostname validation
        hostname.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    }
    
    /// Escape IRC message content
    pub fn escape_message(content: &str) -> String {
        content
            .replace('\r', "")
            .replace('\n', "")
            .replace('\0', "")
    }
    
    /// Unescape IRC message content
    pub fn unescape_message(content: &str) -> String {
        content.to_string()
    }
}

/// Time utilities
pub mod time {
    use chrono::{DateTime, Utc};
    
    /// Get current timestamp as string
    pub fn current_timestamp() -> String {
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }
    
    /// Get current timestamp as Unix timestamp
    pub fn current_unix_timestamp() -> i64 {
        Utc::now().timestamp()
    }
    
    /// Format duration as human readable string
    pub fn format_duration(seconds: u64) -> String {
        let days = seconds / 86400;
        let hours = (seconds % 86400) / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;
        
        if days > 0 {
            format!("{}d {}h {}m {}s", days, hours, minutes, secs)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, secs)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, secs)
        } else {
            format!("{}s", secs)
        }
    }
}

/// Network utilities
pub mod network {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    
    /// Check if an IP address is private
    pub fn is_private_ip(ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => ipv4.is_private(),
            IpAddr::V6(ipv6) => ipv6.is_unicast_link_local(),
        }
    }
    
    /// Check if an IP address is loopback
    pub fn is_loopback_ip(ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => ipv4.is_loopback(),
            IpAddr::V6(ipv6) => ipv6.is_loopback(),
        }
    }
    
    /// Check if an IP address is link-local
    pub fn is_link_local_ip(ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => ipv4.is_link_local(),
            IpAddr::V6(ipv6) => ipv6.is_unicast_link_local(),
        }
    }
    
    /// Parse IP address from string
    pub fn parse_ip(ip_str: &str) -> Result<IpAddr, Box<dyn std::error::Error + Send + Sync>> {
        Ok(IpAddr::from_str(ip_str)
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Invalid IP address: {}", e))) as Box<dyn std::error::Error + Send + Sync>)?)
    }
}

/// Hash utilities
pub mod hash {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    /// Calculate hash of a string
    pub fn hash_string(s: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Calculate hash of multiple strings
    pub fn hash_strings(strings: &[&str]) -> u64 {
        let mut hasher = DefaultHasher::new();
        for s in strings {
            s.hash(&mut hasher);
        }
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_channel_name() {
        assert!(string::is_valid_channel_name("#channel"));
        assert!(string::is_valid_channel_name("&channel"));
        assert!(string::is_valid_channel_name("+channel"));
        assert!(string::is_valid_channel_name("!channel"));
        assert!(!string::is_valid_channel_name("channel"));
        assert!(!string::is_valid_channel_name(""));
        assert!(!string::is_valid_channel_name("#chan nel"));
    }
    
    #[test]
    fn test_valid_nickname() {
        assert!(string::is_valid_nickname("alice", 9));
        assert!(string::is_valid_nickname("alice123", 9));
        assert!(string::is_valid_nickname("alice_", 9));
        assert!(string::is_valid_nickname("alice-", 9));
        assert!(!string::is_valid_nickname("", 9));
        assert!(!string::is_valid_nickname("123alice", 9));
        assert!(!string::is_valid_nickname("alice space", 9));
    }
    
    #[test]
    fn test_private_ip() {
        assert!(network::is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(network::is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(network::is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(!network::is_private_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    }
}
