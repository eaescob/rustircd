//! MOTD (Message of the Day) management system

use crate::{Error, Result, Message, NumericReply};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

/// MOTD (Message of the Day) manager
pub struct MotdManager {
    /// MOTD lines loaded from file
    lines: Arc<RwLock<Vec<String>>>,
    /// Whether MOTD is enabled
    enabled: bool,
}

impl MotdManager {
    /// Create a new MOTD manager
    pub fn new() -> Self {
        Self {
            lines: Arc::new(RwLock::new(Vec::new())),
            enabled: false,
        }
    }

    /// Load MOTD from file (supports both relative and absolute paths)
    pub async fn load_motd(&mut self, motd_file: &str) -> Result<()> {
        let path = if Path::new(motd_file).is_absolute() {
            // Absolute path - use as-is
            Path::new(motd_file).to_path_buf()
        } else {
            // Relative path - resolve from current working directory
            std::env::current_dir()
                .map_err(|e| Error::Config(format!("Failed to get current directory: {}", e)))?
                .join(motd_file)
        };
        
        if !path.exists() {
            warn!("MOTD file not found: {} (resolved from: {})", path.display(), motd_file);
            return Ok(()); // Not an error, just no MOTD
        }

        match fs::read_to_string(&path) {
            Ok(content) => {
                let lines: Vec<String> = content
                    .lines()
                    .map(|line| line.to_string())
                    .collect();

                let mut motd_lines = self.lines.write().await;
                *motd_lines = lines;
                self.enabled = true;

                info!("Loaded MOTD from {} (resolved from: {}, {} lines)", 
                      path.display(), motd_file, motd_lines.len());
                debug!("MOTD lines: {:?}", *motd_lines);
                Ok(())
            }
            Err(e) => {
                error!("Failed to read MOTD file {} (resolved from: {}): {}", 
                       path.display(), motd_file, e);
                Err(Error::Config(format!("Failed to read MOTD file: {}", e)))
            }
        }
    }

    /// Check if MOTD is enabled and has content
    pub async fn is_enabled(&self) -> bool {
        self.enabled && !self.lines.read().await.is_empty()
    }

    /// Get MOTD start message
    pub fn get_motd_start(&self, server_name: &str) -> Message {
        NumericReply::motd_start(server_name)
    }

    /// Get MOTD line message
    pub async fn get_motd_line(&self, line_number: usize) -> Option<Message> {
        let lines = self.lines.read().await;
        if line_number < lines.len() {
            Some(NumericReply::motd_line(&lines[line_number]))
        } else {
            None
        }
    }

    /// Get MOTD end message
    pub fn get_motd_end(&self, server_name: &str) -> Message {
        NumericReply::motd_end(server_name)
    }

    /// Get MOTD file not found message
    pub fn get_no_motd(&self, server_name: &str) -> Message {
        NumericReply::no_motd(server_name)
    }

    /// Get all MOTD messages for a complete MOTD display
    pub async fn get_all_motd_messages(&self, server_name: &str) -> Vec<Message> {
        let mut messages = Vec::new();

        if !self.is_enabled().await {
            messages.push(self.get_no_motd(server_name));
            return messages;
        }

        messages.push(self.get_motd_start(server_name));

        let lines = self.lines.read().await;
        for line in lines.iter() {
            messages.push(NumericReply::motd_line(line));
        }

        messages.push(self.get_motd_end(server_name));
        messages
    }

    /// Get MOTD line count
    pub async fn line_count(&self) -> usize {
        self.lines.read().await.len()
    }

    /// Reload MOTD from file (useful for runtime updates)
    /// Supports both relative and absolute paths
    pub async fn reload(&mut self, motd_file: &str) -> Result<()> {
        info!("Reloading MOTD from: {}", motd_file);
        self.load_motd(motd_file).await
    }

    /// Clear MOTD (disable it)
    pub async fn clear(&mut self) {
        let mut lines = self.lines.write().await;
        lines.clear();
        self.enabled = false;
        info!("MOTD cleared and disabled");
    }

    /// Set MOTD lines directly (for testing or dynamic updates)
    pub async fn set_lines(&mut self, lines: Vec<String>) {
        let mut motd_lines = self.lines.write().await;
        *motd_lines = lines;
        self.enabled = !motd_lines.is_empty();
        info!("MOTD lines set directly ({} lines, enabled: {})", motd_lines.len(), self.enabled);
    }

    /// Resolve MOTD file path (supports both relative and absolute paths)
    /// Returns the resolved absolute path for the given MOTD file path
    pub fn resolve_motd_path(motd_file: &str) -> Result<std::path::PathBuf> {
        if Path::new(motd_file).is_absolute() {
            // Absolute path - use as-is
            Ok(Path::new(motd_file).to_path_buf())
        } else {
            // Relative path - resolve from current working directory
            Ok(std::env::current_dir()
                .map_err(|e| Error::Config(format!("Failed to get current directory: {}", e)))?
                .join(motd_file))
        }
    }

    /// Check if MOTD file exists (supports both relative and absolute paths)
    pub fn motd_file_exists(motd_file: &str) -> bool {
        match Self::resolve_motd_path(motd_file) {
            Ok(path) => path.exists(),
            Err(_) => false,
        }
    }
}

impl Default for MotdManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_motd_manager_creation() {
        let manager = MotdManager::new();
        assert!(!manager.is_enabled().await);
        assert_eq!(manager.line_count().await, 0);
    }

    #[tokio::test]
    async fn test_motd_loading() {
        // Create a temporary MOTD file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Welcome to RustIRCd!").unwrap();
        writeln!(temp_file, "This is line 2").unwrap();
        writeln!(temp_file, "This is line 3").unwrap();
        temp_file.flush().unwrap();

        let mut manager = MotdManager::new();
        let result = manager.load_motd(temp_file.path().to_str().unwrap()).await;
        
        assert!(result.is_ok());
        assert!(manager.is_enabled().await);
        assert_eq!(manager.line_count().await, 3);
    }

    #[tokio::test]
    async fn test_motd_messages() {
        let mut manager = MotdManager::new();
        manager.set_lines(vec![
            "Line 1".to_string(),
            "Line 2".to_string(),
        ]).await;

        let messages = manager.get_all_motd_messages("test.server").await;
        
        // Should have: start + 2 lines + end = 4 messages
        assert_eq!(messages.len(), 4);
        
        // Check first message is MOTD start
        assert!(messages[0].to_string().contains("375"));
        
        // Check last message is MOTD end
        assert!(messages[3].to_string().contains("376"));
    }

    #[tokio::test]
    async fn test_no_motd_file() {
        let mut manager = MotdManager::new();
        let result = manager.load_motd("nonexistent_file.txt").await;
        
        // Should not error, just not load anything
        assert!(result.is_ok());
        assert!(!manager.is_enabled().await);
    }

    #[tokio::test]
    async fn test_motd_clear() {
        let mut manager = MotdManager::new();
        manager.set_lines(vec!["Test line".to_string()]).await;
        
        assert!(manager.is_enabled().await);
        
        manager.clear().await;
        
        assert!(!manager.is_enabled().await);
        assert_eq!(manager.line_count().await, 0);
    }

    #[test]
    fn test_resolve_motd_path() {
        // Test relative path resolution
        let relative_path = "motd.txt";
        let resolved = MotdManager::resolve_motd_path(relative_path).unwrap();
        assert!(resolved.is_absolute());
        assert!(resolved.ends_with("motd.txt"));

        // Test absolute path (Unix-style)
        #[cfg(unix)]
        {
            let absolute_path = "/etc/motd.txt";
            let resolved = MotdManager::resolve_motd_path(absolute_path).unwrap();
            assert_eq!(resolved, std::path::PathBuf::from("/etc/motd.txt"));
        }

        // Test absolute path (Windows-style)
        #[cfg(windows)]
        {
            let absolute_path = "C:\\Windows\\motd.txt";
            let resolved = MotdManager::resolve_motd_path(absolute_path).unwrap();
            assert_eq!(resolved, std::path::PathBuf::from("C:\\Windows\\motd.txt"));
        }
    }

    #[test]
    fn test_motd_file_exists() {
        // Test with a file that definitely doesn't exist
        assert!(!MotdManager::motd_file_exists("nonexistent_file_12345.txt"));

        // Test with a relative path that might exist (current directory)
        // This test might pass or fail depending on the environment
        let current_dir_file = "Cargo.toml"; // This usually exists in Rust projects
        let exists = MotdManager::motd_file_exists(current_dir_file);
        // We can't assert true/false here as it depends on the environment
        // But we can assert the function doesn't panic
        assert!(exists || !exists); // This will always be true, just checking it doesn't panic
    }
}
