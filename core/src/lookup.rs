//! DNS and ident lookup functionality for RFC compliance

use crate::{Error, Result};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio::io::AsyncWriteExt;
use trust_dns_resolver::TokioAsyncResolver;

/// Result of a hostname lookup
#[derive(Debug, Clone)]
pub struct LookupResult {
    /// The original IP address
    pub original_ip: IpAddr,
    /// The resolved hostname (if successful)
    pub hostname: Option<String>,
    /// Whether the lookup was successful
    pub success: bool,
    /// Error message if lookup failed
    pub error: Option<String>,
}

/// Result of an ident lookup
#[derive(Debug, Clone)]
pub struct IdentResult {
    /// The username returned by ident
    pub username: Option<String>,
    /// Whether the lookup was successful
    pub success: bool,
    /// Error message if lookup failed
    pub error: Option<String>,
}

/// DNS resolver for hostname lookups
pub struct DnsResolver {
    resolver: TokioAsyncResolver,
    enabled: bool,
    reverse_enabled: bool,
    cache: Arc<crate::DnsCache>,
}

impl DnsResolver {
    /// Create a new DNS resolver
    pub async fn new(enable_dns: bool, enable_reverse_dns: bool) -> Result<Self> {
        let resolver = TokioAsyncResolver::tokio_from_system_conf()
            .map_err(|e| Error::Generic(format!("Failed to create DNS resolver: {}", e)))?;
        
        Ok(Self {
            resolver,
            enabled: enable_dns,
            reverse_enabled: enable_reverse_dns,
            cache: Arc::new(crate::DnsCache::new(std::time::Duration::from_secs(300))),
        })
    }

    /// Perform reverse DNS lookup (IP to hostname)
    pub async fn reverse_lookup(&self, ip: IpAddr) -> LookupResult {
        if !self.reverse_enabled {
            return LookupResult {
                original_ip: ip,
                hostname: None,
                success: false,
                error: Some("Reverse DNS lookup disabled".to_string()),
            };
        }

        // Check cache first
        let ip_str = ip.to_string();
        if let Some(cached_hostname) = self.cache.get_hostname(&ip_str) {
            return LookupResult {
                original_ip: ip,
                hostname: Some(cached_hostname),
                success: true,
                error: None,
            };
        }

        let lookup_result = timeout(
            Duration::from_secs(5),
            self.resolver.reverse_lookup(ip),
        ).await;

        match lookup_result {
            Ok(Ok(names)) => {
                // Get the first hostname from the result
                let hostname = names.iter().next()
                    .map(|name| name.to_string());
                
                // Cache the result
                if let Some(ref h) = hostname {
                    self.cache.cache_hostname(ip_str, h.clone());
                }
                
                LookupResult {
                    original_ip: ip,
                    hostname,
                    success: true,
                    error: None,
                }
            }
            Ok(Err(e)) => LookupResult {
                original_ip: ip,
                hostname: None,
                success: false,
                error: Some(format!("DNS lookup failed: {}", e)),
            },
            Err(_) => LookupResult {
                original_ip: ip,
                hostname: None,
                success: false,
                error: Some("DNS lookup timeout".to_string()),
            },
        }
    }

    /// Perform forward DNS lookup (hostname to IP)
    pub async fn forward_lookup(&self, hostname: &str) -> LookupResult {
        if !self.enabled {
            return LookupResult {
                original_ip: IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                hostname: None,
                success: false,
                error: Some("DNS lookup disabled".to_string()),
            };
        }

        // Check cache first
        if let Some(cached_ip) = self.cache.get_ip(hostname) {
            if let Ok(ip) = cached_ip.parse::<IpAddr>() {
                return LookupResult {
                    original_ip: ip,
                    hostname: Some(hostname.to_string()),
                    success: true,
                    error: None,
                };
            }
        }

        let lookup_result = timeout(
            Duration::from_secs(5),
            self.resolver.lookup_ip(hostname),
        ).await;

        match lookup_result {
            Ok(Ok(ips)) => {
                // Get the first IP from the result
                if let Some(ip) = ips.iter().next() {
                    // Cache the result
                    self.cache.cache_hostname(ip.to_string(), hostname.to_string());
                    
                    LookupResult {
                        original_ip: ip,
                        hostname: Some(hostname.to_string()),
                        success: true,
                        error: None,
                    }
                } else {
                    LookupResult {
                        original_ip: IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                        hostname: None,
                        success: false,
                        error: Some("No IP addresses found".to_string()),
                    }
                }
            }
            Ok(Err(e)) => LookupResult {
                original_ip: IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                hostname: None,
                success: false,
                error: Some(format!("DNS lookup failed: {}", e)),
            },
            Err(_) => LookupResult {
                original_ip: IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                hostname: None,
                success: false,
                error: Some("DNS lookup timeout".to_string()),
            },
        }
    }
}

/// Ident client for RFC 1413 ident lookups
pub struct IdentClient {
    enabled: bool,
    timeout: Duration,
}

impl IdentClient {
    /// Create a new ident client
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            timeout: Duration::from_secs(10),
        }
    }

    /// Perform ident lookup for a connection
    pub async fn lookup(&self, client_addr: SocketAddr, server_addr: SocketAddr) -> IdentResult {
        if !self.enabled {
            return IdentResult {
                username: None,
                success: false,
                error: Some("Ident lookup disabled".to_string()),
            };
        }

        // RFC 1413: Connect to the ident port (113) on the client's machine
        let ident_addr = SocketAddr::new(client_addr.ip(), 113);
        
        let connection_result = timeout(
            self.timeout,
            TcpStream::connect(ident_addr),
        ).await;

        match connection_result {
            Ok(Ok(mut stream)) => {
                // Send ident query according to RFC 1413
                let query = format!("{}, {}\r\n", server_addr.port(), client_addr.port());
                
                if let Err(e) = stream.write_all(query.as_bytes()).await {
                    return IdentResult {
                        username: None,
                        success: false,
                        error: Some(format!("Failed to send ident query: {}", e)),
                    };
                }

                // Read response
                let mut response = String::new();
                let read_result = timeout(
                    Duration::from_secs(5),
                    tokio::io::AsyncReadExt::read_to_string(&mut stream, &mut response),
                ).await;

                match read_result {
                    Ok(Ok(_)) => {
                        // Parse ident response
                        // Format: "port, port : USERID : OS : username"
                        if let Some(colon_pos) = response.find(':') {
                            if let Some(second_colon_pos) = response[colon_pos + 1..].find(':') {
                                let start = colon_pos + 1 + second_colon_pos + 1;
                                if let Some(third_colon_pos) = response[start..].find(':') {
                                    let username = response[start + third_colon_pos + 1..].trim().to_string();
                                    return IdentResult {
                                        username: Some(username),
                                        success: true,
                                        error: None,
                                    };
                                }
                            }
                        }
                        
                        IdentResult {
                            username: None,
                            success: false,
                            error: Some("Invalid ident response format".to_string()),
                        }
                    }
                    Ok(Err(e)) => IdentResult {
                        username: None,
                        success: false,
                        error: Some(format!("Failed to read ident response: {}", e)),
                    },
                    Err(_) => IdentResult {
                        username: None,
                        success: false,
                        error: Some("Ident response timeout".to_string()),
                    },
                }
            }
            Ok(Err(e)) => IdentResult {
                username: None,
                success: false,
                error: Some(format!("Failed to connect to ident service: {}", e)),
            },
            Err(_) => IdentResult {
                username: None,
                success: false,
                error: Some("Ident connection timeout".to_string()),
            },
        }
    }
}

/// Combined lookup service that handles both DNS and ident lookups
pub struct LookupService {
    dns_resolver: DnsResolver,
    ident_client: IdentClient,
}

impl LookupService {
    /// Create a new lookup service
    pub async fn new(
        enable_dns: bool,
        enable_reverse_dns: bool,
        enable_ident: bool,
    ) -> Result<Self> {
        let dns_resolver = DnsResolver::new(enable_dns, enable_reverse_dns).await?;
        let ident_client = IdentClient::new(enable_ident);
        
        Ok(Self {
            dns_resolver,
            ident_client,
        })
    }

    /// Perform reverse DNS lookup
    pub async fn reverse_dns_lookup(&self, ip: IpAddr) -> LookupResult {
        self.dns_resolver.reverse_lookup(ip).await
    }

    /// Perform forward DNS lookup
    pub async fn forward_dns_lookup(&self, hostname: &str) -> LookupResult {
        self.dns_resolver.forward_lookup(hostname).await
    }

    /// Perform ident lookup
    pub async fn ident_lookup(&self, client_addr: SocketAddr, server_addr: SocketAddr) -> IdentResult {
        self.ident_client.lookup(client_addr, server_addr).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr};

    #[tokio::test]
    async fn test_ident_client_disabled() {
        let client = IdentClient::new(false);
        let result = client.lookup(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1234),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6667),
        ).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("disabled"));
    }

    #[tokio::test]
    async fn test_dns_resolver_disabled() {
        let resolver = DnsResolver::new(false, false).await.unwrap();
        let result = resolver.reverse_lookup(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))).await;
        
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("disabled"));
    }
}
