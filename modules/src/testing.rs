//! Testing and Debugging Module
//! 
//! Provides testing and debugging commands including TESTLINE and TESTMASK.
//! Based on Ratbox's testing modules.

use rustircd_core::{
    async_trait, Client, Error, Message, MessageType, Module,
    ModuleNumericManager, module::{ModuleResult, ModuleStatsResponse, ModuleContext},
    NumericReply, Result, User
};
use tracing::info;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Testing and debugging module
pub struct TestingModule {
    /// Test line results cache
    test_results: RwLock<HashMap<String, TestResult>>,
    /// Maximum cache size
    max_cache_size: usize,
    /// Test configuration
    config: TestConfig,
}

/// Test result for a line
#[derive(Debug, Clone)]
pub struct TestResult {
    pub line: String,
    pub result: TestLineResult,
    pub tested_at: u64,
    pub tested_by: String,
}

/// Result of a test line operation
#[derive(Debug, Clone, PartialEq)]
pub enum TestLineResult {
    Success {
        message: String,
        details: Option<String>,
    },
    Failure {
        error: String,
        details: Option<String>,
    },
    Timeout {
        duration: u64,
    },
    ConnectionRefused {
        reason: String,
    },
    InvalidTarget {
        reason: String,
    },
}

/// Configuration for testing module
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub max_test_duration: u64, // in seconds
    pub max_concurrent_tests: usize,
    pub cache_ttl: u64, // in seconds
    pub allow_remote_tests: bool,
    pub require_operator: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            max_test_duration: 30,
            max_concurrent_tests: 10,
            cache_ttl: 300, // 5 minutes
            allow_remote_tests: true,
            require_operator: true,
        }
    }
}

impl TestingModule {
    /// Create a new testing module
    pub fn new() -> Self {
        Self {
            test_results: RwLock::new(HashMap::new()),
            max_cache_size: 1000,
            config: TestConfig::default(),
        }
    }
    
    /// Create a new testing module with custom configuration
    pub fn with_config(config: TestConfig) -> Self {
        Self {
            test_results: RwLock::new(HashMap::new()),
            max_cache_size: 1000,
            config,
        }
    }
    
    /// Handle TESTLINE command
    async fn handle_testline(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if self.config.require_operator && !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["TESTLINE", "Not enough parameters"])?;
            return Ok(());
        }
        
        let target = &args[0];
        let port = if args.len() > 1 {
            args[1].parse::<u16>().unwrap_or(6667)
        } else {
            6667
        };
        
        // Check if we have a cached result
        let cache_key = format!("{}:{}", target, port);
        if let Some(cached_result) = self.get_cached_result(&cache_key).await {
            if self.is_cache_valid(&cached_result).await {
                self.send_test_result(client, &cached_result).await?;
                return Ok(());
            }
        }
        
        // Perform the test
        let test_result = self.perform_test_line(target, port, user).await?;
        
        // Cache the result
        self.cache_result(&cache_key, test_result.clone()).await?;
        
        // Send result to client
        self.send_test_result(client, &test_result).await?;
        
        Ok(())
    }
    
    /// Handle TESTMASK command
    async fn handle_testmask(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if self.config.require_operator && !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.len() < 2 {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["TESTMASK", "Not enough parameters"])?;
            return Ok(());
        }
        
        let mask = &args[0];
        let test_string = &args[1];
        
        // Test the mask against the string
        let result = self.test_mask_match(mask, test_string);
        
        // Send result
        if result {
            client.send_numeric(NumericReply::RplTestMask, &[mask, test_string, "MATCH"])?;
        } else {
            client.send_numeric(NumericReply::RplTestMask, &[mask, test_string, "NO MATCH"])?;
        }
        
        Ok(())
    }
    
    /// Perform a test line operation
    async fn perform_test_line(&self, target: &str, port: u16, user: &User) -> Result<TestResult> {
        let start_time = self.get_current_time();
        
        // Validate target
        if !self.is_valid_target(target) {
            return Ok(TestResult {
                line: format!("{}:{}", target, port),
                result: TestLineResult::InvalidTarget {
                    reason: "Invalid target format".to_string(),
                },
                tested_at: start_time,
                tested_by: user.nickname().to_string(),
            });
        }
        
        // Check if remote tests are allowed
        if !self.config.allow_remote_tests && !self.is_local_target(target) {
            return Ok(TestResult {
                line: format!("{}:{}", target, port),
                result: TestLineResult::Failure {
                    error: "Remote tests not allowed".to_string(),
                    details: None,
                },
                tested_at: start_time,
                tested_by: user.nickname().to_string(),
            });
        }
        
        // Perform the actual test
        match self.test_connection(target, port).await {
            Ok(message) => {
                let duration = self.get_current_time() - start_time;
                Ok(TestResult {
                    line: format!("{}:{}", target, port),
                    result: TestLineResult::Success {
                        message,
                        details: Some(format!("Connection successful in {}ms", duration * 1000)),
                    },
                    tested_at: start_time,
                    tested_by: user.nickname().to_string(),
                })
            }
            Err(error) => {
                let duration = self.get_current_time() - start_time;
                if duration >= self.config.max_test_duration {
                    Ok(TestResult {
                        line: format!("{}:{}", target, port),
                        result: TestLineResult::Timeout {
                            duration,
                        },
                        tested_at: start_time,
                        tested_by: user.nickname().to_string(),
                    })
                } else {
                    Ok(TestResult {
                        line: format!("{}:{}", target, port),
                        result: TestLineResult::Failure {
                            error: error.to_string(),
                            details: Some(format!("Failed after {}ms", duration * 1000)),
                        },
                        tested_at: start_time,
                        tested_by: user.nickname().to_string(),
                    })
                }
            }
        }
    }
    
    /// Test a connection to a target
    async fn test_connection(&self, target: &str, port: u16) -> Result<String> {
        use tokio::net::TcpStream;
        use tokio::time::{timeout, Duration};
        
        let address = format!("{}:{}", target, port);
        let duration = Duration::from_secs(self.config.max_test_duration);
        
        match timeout(duration, TcpStream::connect(&address)).await {
            Ok(Ok(_stream)) => {
                Ok(format!("Connection to {} successful", address))
            }
            Ok(Err(e)) => {
                Err(Error::Connection(format!("Connection failed: {}", e)))
            }
            Err(_) => {
                Err(Error::Connection("Connection timeout".to_string()))
            }
        }
    }
    
    /// Test if a mask matches a string
    fn test_mask_match(&self, mask: &str, test_string: &str) -> bool {
        // Convert IRC wildcards to regex patterns
        let pattern = mask
            .replace("*", ".*")
            .replace("?", ".");
        
        // Simple pattern matching - in production, use proper regex
        if mask.contains('*') || mask.contains('?') {
            self.simple_wildcard_match(&pattern, test_string)
        } else {
            mask == test_string
        }
    }
    
    /// Simple wildcard matching
    fn simple_wildcard_match(&self, pattern: &str, text: &str) -> bool {
        if pattern == ".*" {
            return true;
        }
        
        if pattern.starts_with(".*") && pattern.ends_with(".*") {
            let middle = &pattern[2..pattern.len()-2];
            return text.contains(middle);
        }
        
        if pattern.starts_with(".*") {
            return text.ends_with(&pattern[2..]);
        }
        
        if pattern.ends_with(".*") {
            return text.starts_with(&pattern[..pattern.len()-2]);
        }
        
        text == pattern
    }
    
    /// Check if a target is valid
    fn is_valid_target(&self, target: &str) -> bool {
        // Basic validation - check for valid hostname/IP format
        !target.is_empty() && target.len() <= 255
    }
    
    /// Check if a target is local
    fn is_local_target(&self, target: &str) -> bool {
        // Check if target is localhost or local IP
        target == "localhost" || 
        target == "127.0.0.1" || 
        target == "::1" ||
        target.starts_with("192.168.") ||
        target.starts_with("10.") ||
        target.starts_with("172.")
    }
    
    /// Get cached result
    async fn get_cached_result(&self, key: &str) -> Option<TestResult> {
        let results = self.test_results.read().await;
        results.get(key).cloned()
    }
    
    /// Check if cache entry is valid
    async fn is_cache_valid(&self, result: &TestResult) -> bool {
        let current_time = self.get_current_time();
        current_time - result.tested_at < self.config.cache_ttl
    }
    
    /// Cache a test result
    async fn cache_result(&self, key: &str, result: TestResult) -> Result<()> {
        let mut results = self.test_results.write().await;

        // Trim cache if too large
        if results.len() >= self.max_cache_size {
            // Filter expired cache entries based on timestamp check
            let current_time = self.get_current_time();
            let keys_to_remove: Vec<String> = results
                .iter()
                .filter(|(_, result)| {
                    // Check if cache is expired without await
                    current_time - result.tested_at >= self.config.cache_ttl
                })
                .map(|(key, _)| key.clone())
                .collect();

            for key in keys_to_remove {
                results.remove(&key);
            }
        }

        results.insert(key.to_string(), result);
        Ok(())
    }
    
    /// Send test result to client
    async fn send_test_result(&self, client: &Client, result: &TestResult) -> Result<()> {
        match &result.result {
            TestLineResult::Success { message, details } => {
                client.send_numeric(NumericReply::RplTestLine, &[&result.line, "SUCCESS", message])?;
                if let Some(details) = details {
                    client.send_numeric(NumericReply::RplTestLine, &[&result.line, "DETAILS", details])?;
                }
            }
            TestLineResult::Failure { error, details } => {
                client.send_numeric(NumericReply::RplTestLine, &[&result.line, "FAILURE", error])?;
                if let Some(details) = details {
                    client.send_numeric(NumericReply::RplTestLine, &[&result.line, "DETAILS", details])?;
                }
            }
            TestLineResult::Timeout { duration } => {
                client.send_numeric(NumericReply::RplTestLine, &[&result.line, "TIMEOUT", &format!("{} seconds", duration)])?;
            }
            TestLineResult::ConnectionRefused { reason } => {
                client.send_numeric(NumericReply::RplTestLine, &[&result.line, "REFUSED", reason])?;
            }
            TestLineResult::InvalidTarget { reason } => {
                client.send_numeric(NumericReply::RplTestLine, &[&result.line, "INVALID", reason])?;
            }
        }
        
        client.send_numeric(NumericReply::RplTestLine, &[&result.line, "TESTED_BY", &result.tested_by])?;
        client.send_numeric(NumericReply::RplTestLine, &[&result.line, "TESTED_AT", &self.format_time(result.tested_at)])?;
        
        Ok(())
    }
    
    /// Get current time as Unix timestamp
    fn get_current_time(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    /// Format time as readable string
    fn format_time(&self, timestamp: u64) -> String {
        use chrono::{DateTime, Utc};
        let naive = DateTime::from_timestamp(timestamp as i64, 0).unwrap_or_default().naive_utc();
        let datetime: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive, Utc);
        datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }
    
    /// Get test statistics
    pub async fn get_test_statistics(&self) -> TestStatistics {
        let results = self.test_results.read().await;
        let mut stats = TestStatistics::default();
        
        for result in results.values() {
            stats.total_tests += 1;
            
            match result.result {
                TestLineResult::Success { .. } => stats.successful_tests += 1,
                TestLineResult::Failure { .. } => stats.failed_tests += 1,
                TestLineResult::Timeout { .. } => stats.timeout_tests += 1,
                TestLineResult::ConnectionRefused { .. } => stats.refused_tests += 1,
                TestLineResult::InvalidTarget { .. } => stats.invalid_tests += 1,
            }
        }
        
        stats
    }
    
    /// Clear test cache
    pub async fn clear_cache(&self) {
        let mut results = self.test_results.write().await;
        results.clear();
    }
}

/// Test statistics
#[derive(Debug, Clone, Default)]
pub struct TestStatistics {
    pub total_tests: usize,
    pub successful_tests: usize,
    pub failed_tests: usize,
    pub timeout_tests: usize,
    pub refused_tests: usize,
    pub invalid_tests: usize,
}

#[async_trait]
impl Module for TestingModule {
    fn name(&self) -> &str {
        "testing"
    }
    
    fn description(&self) -> &str {
        "Provides testing and debugging commands including TESTLINE and TESTMASK"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    async fn init(&mut self) -> Result<()> {
        info!("{} module initialized", self.name());
        Ok(())
    }

    async fn handle_message(&mut self, client: &Client, message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
        let user = match &client.user {
            Some(u) => u,
            None => return Ok(ModuleResult::NotHandled),
        };

        match message.command {
            MessageType::Custom(ref cmd) if cmd == "TESTLINE" => {
                self.handle_testline(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "TESTMASK" => {
                self.handle_testmask(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            _ => Ok(ModuleResult::NotHandled),
        }
    }

    async fn handle_server_message(&mut self, _server: &str, _message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
        Ok(ModuleResult::NotHandled)
    }

    async fn handle_user_registration(&mut self, _user: &User, _context: &ModuleContext) -> Result<()> {
        Ok(())
    }

    async fn handle_user_disconnection(&mut self, _user: &User, _context: &ModuleContext) -> Result<()> {
        Ok(())
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec!["message_handler".to_string()]
    }

    fn supports_capability(&self, capability: &str) -> bool {
        capability == "message_handler"
    }

    fn get_numeric_replies(&self) -> Vec<u16> {
        vec![]
    }

    fn handles_numeric_reply(&self, _numeric: u16) -> bool {
        false
    }

    async fn handle_numeric_reply(&mut self, _numeric: u16, _params: Vec<String>) -> Result<()> {
        Ok(())
    }

    async fn handle_stats_query(&mut self, _query: &str, _client_id: uuid::Uuid, _server: Option<&rustircd_core::Server>) -> Result<Vec<ModuleStatsResponse>> {
        Ok(vec![])
    }

    fn get_stats_queries(&self) -> Vec<String> {
        vec![]
    }

    fn register_numerics(&self, _manager: &mut ModuleNumericManager) -> Result<()> {
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<()> {
        info!("Testing module cleaned up");
        Ok(())
    }
}

impl Default for TestingModule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_test_config_default() {
        let config = TestConfig::default();
        assert_eq!(config.max_test_duration, 30);
        assert_eq!(config.max_concurrent_tests, 10);
        assert_eq!(config.cache_ttl, 300);
        assert!(config.allow_remote_tests);
        assert!(config.require_operator);
    }
    
    #[test]
    fn test_testing_module_creation() {
        let module = TestingModule::new();
        assert_eq!(module.config.max_test_duration, 30);
        assert_eq!(module.max_cache_size, 1000);
    }
    
    #[test]
    fn test_mask_matching() {
        let module = TestingModule::new();
        
        assert!(module.test_mask_match("test", "test"));
        assert!(module.test_mask_match("test*", "testing"));
        assert!(module.test_mask_match("*test", "mytest"));
        assert!(module.test_mask_match("t*st", "test"));
        assert!(module.test_mask_match("t?st", "test"));
        
        assert!(!module.test_mask_match("test", "testing"));
        assert!(!module.test_mask_match("test*", "mytest"));
        assert!(!module.test_mask_match("*test", "testing"));
    }
    
    #[test]
    fn test_target_validation() {
        let module = TestingModule::new();
        
        assert!(module.is_valid_target("localhost"));
        assert!(module.is_valid_target("127.0.0.1"));
        assert!(module.is_valid_target("example.com"));
        
        assert!(!module.is_valid_target(""));
        assert!(!module.is_valid_target(&"a".repeat(256)));
    }
    
    #[test]
    fn test_local_target_detection() {
        let module = TestingModule::new();
        
        assert!(module.is_local_target("localhost"));
        assert!(module.is_local_target("127.0.0.1"));
        assert!(module.is_local_target("::1"));
        assert!(module.is_local_target("192.168.1.1"));
        assert!(module.is_local_target("10.0.0.1"));
        assert!(module.is_local_target("172.16.0.1"));
        
        assert!(!module.is_local_target("example.com"));
        assert!(!module.is_local_target("8.8.8.8"));
    }
    
    #[tokio::test]
    async fn test_test_statistics() {
        let module = TestingModule::new();
        let stats = module.get_test_statistics().await;
        
        assert_eq!(stats.total_tests, 0);
        assert_eq!(stats.successful_tests, 0);
        assert_eq!(stats.failed_tests, 0);
    }
}
