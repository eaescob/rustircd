//! Buffer management for send and receive queues
//!
//! This module implements bounded buffers for managing outgoing (sendq) and incoming (recvq)
//! data for IRC connections. These are critical for preventing resource exhaustion and
//! implementing per-class buffer limits.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Send queue - manages outgoing data with size limits
#[derive(Debug)]
pub struct SendQueue {
    /// Buffer of outgoing messages
    buffer: VecDeque<String>,
    /// Current size in bytes
    current_size: usize,
    /// Maximum size in bytes
    max_size: usize,
    /// Number of messages dropped due to buffer overflow
    dropped_messages: u64,
    /// Last time a message was added
    last_write: Option<Instant>,
}

impl SendQueue {
    /// Create a new send queue with specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: VecDeque::new(),
            current_size: 0,
            max_size,
            dropped_messages: 0,
            last_write: None,
        }
    }

    /// Add a message to the send queue
    /// Returns true if added, false if dropped due to buffer full
    pub fn push(&mut self, message: String) -> bool {
        let message_size = message.len();
        
        // Check if adding this message would exceed the buffer limit
        if self.current_size + message_size > self.max_size {
            self.dropped_messages += 1;
            tracing::warn!(
                "SendQueue full ({}/{}), dropping message",
                self.current_size,
                self.max_size
            );
            return false;
        }

        self.current_size += message_size;
        self.buffer.push_back(message);
        self.last_write = Some(Instant::now());
        true
    }

    /// Remove and return the next message from the queue
    pub fn pop(&mut self) -> Option<String> {
        if let Some(message) = self.buffer.pop_front() {
            self.current_size = self.current_size.saturating_sub(message.len());
            Some(message)
        } else {
            None
        }
    }

    /// Get the number of messages in the queue
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Get current buffer size in bytes
    pub fn current_size(&self) -> usize {
        self.current_size
    }

    /// Get maximum buffer size in bytes
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Get number of dropped messages
    pub fn dropped_messages(&self) -> u64 {
        self.dropped_messages
    }

    /// Check if the buffer is near capacity (>90% full)
    pub fn is_near_capacity(&self) -> bool {
        self.current_size > (self.max_size * 9 / 10)
    }

    /// Clear all messages from the queue
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.current_size = 0;
    }

    /// Get time since last write
    pub fn time_since_last_write(&self) -> Option<Duration> {
        self.last_write.map(|t| t.elapsed())
    }

    /// Update maximum buffer size (useful for rehash/config updates)
    pub fn set_max_size(&mut self, new_max_size: usize) {
        self.max_size = new_max_size;
        
        // If new size is smaller and we're over capacity, we need to handle it
        if self.current_size > self.max_size {
            tracing::warn!(
                "SendQueue resized to {} bytes, currently at {} bytes - may drop messages",
                self.max_size,
                self.current_size
            );
        }
    }
}

/// Receive queue - manages incoming data with size limits
#[derive(Debug)]
pub struct RecvQueue {
    /// Buffer of incoming data (not yet parsed into complete messages)
    buffer: String,
    /// Maximum size in bytes
    max_size: usize,
    /// Number of bytes dropped due to buffer overflow
    dropped_bytes: u64,
    /// Last time data was received
    last_read: Option<Instant>,
}

impl RecvQueue {
    /// Create a new receive queue with specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: String::new(),
            max_size,
            dropped_bytes: 0,
            last_read: None,
        }
    }

    /// Append data to the receive buffer
    /// Returns true if added, false if would exceed buffer limit
    pub fn append(&mut self, data: &str) -> bool {
        let data_len = data.len();
        
        // Check if adding this data would exceed the buffer limit
        if self.buffer.len() + data_len > self.max_size {
            self.dropped_bytes += data_len as u64;
            tracing::warn!(
                "RecvQueue full ({}/{}), dropping {} bytes",
                self.buffer.len(),
                self.max_size,
                data_len
            );
            return false;
        }

        self.buffer.push_str(data);
        self.last_read = Some(Instant::now());
        true
    }

    /// Extract complete IRC messages (lines ending with \r\n)
    /// Returns a vector of complete messages and retains incomplete data
    pub fn extract_messages(&mut self) -> Vec<String> {
        let mut messages = Vec::new();
        
        while let Some(pos) = self.buffer.find("\r\n") {
            let message = self.buffer.drain(..=pos + 1).collect::<String>();
            messages.push(message);
        }
        
        messages
    }

    /// Get current buffer size in bytes
    pub fn current_size(&self) -> usize {
        self.buffer.len()
    }

    /// Get maximum buffer size in bytes
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Get number of dropped bytes
    pub fn dropped_bytes(&self) -> u64 {
        self.dropped_bytes
    }

    /// Check if the buffer is near capacity (>90% full)
    pub fn is_near_capacity(&self) -> bool {
        self.buffer.len() > (self.max_size * 9 / 10)
    }

    /// Clear the receive buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Get time since last read
    pub fn time_since_last_read(&self) -> Option<Duration> {
        self.last_read.map(|t| t.elapsed())
    }

    /// Update maximum buffer size (useful for rehash/config updates)
    pub fn set_max_size(&mut self, new_max_size: usize) {
        self.max_size = new_max_size;
        
        // If new size is smaller and we're over capacity, truncate
        if self.buffer.len() > self.max_size {
            tracing::warn!(
                "RecvQueue resized to {} bytes, truncating from {} bytes",
                self.max_size,
                self.buffer.len()
            );
            self.buffer.truncate(self.max_size);
            self.dropped_bytes += (self.buffer.len() - self.max_size) as u64;
        }
    }

    /// Check if buffer contains any incomplete data
    pub fn has_incomplete_data(&self) -> bool {
        !self.buffer.is_empty()
    }
}

/// Connection timing information for tracking timeouts and ping frequency
#[derive(Debug, Clone)]
pub struct ConnectionTiming {
    /// When the connection was established
    pub connected_at: Instant,
    /// Last time we received data from this connection
    pub last_activity: Instant,
    /// Last time we sent a PING
    pub last_ping_sent: Option<Instant>,
    /// Last time we received a PONG response
    pub last_pong_received: Option<Instant>,
    /// Number of PINGs sent without PONG response
    pub unanswered_pings: u32,
    /// Ping frequency in seconds (from connection class)
    pub ping_frequency: u64,
    /// Connection timeout in seconds (from connection class)
    pub connection_timeout: u64,
}

impl ConnectionTiming {
    /// Create new connection timing with default values
    pub fn new(ping_frequency: u64, connection_timeout: u64) -> Self {
        let now = Instant::now();
        Self {
            connected_at: now,
            last_activity: now,
            last_ping_sent: None,
            last_pong_received: None,
            unanswered_pings: 0,
            ping_frequency,
            connection_timeout,
        }
    }

    /// Update last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Record that we sent a PING
    pub fn record_ping_sent(&mut self) {
        self.last_ping_sent = Some(Instant::now());
        self.unanswered_pings += 1;
    }

    /// Record that we received a PONG
    pub fn record_pong_received(&mut self) {
        self.last_pong_received = Some(Instant::now());
        self.unanswered_pings = 0;
        self.update_activity();
    }

    /// Check if it's time to send a PING
    pub fn should_send_ping(&self) -> bool {
        match self.last_ping_sent {
            Some(last_ping) => {
                last_ping.elapsed() >= Duration::from_secs(self.ping_frequency)
            }
            None => {
                // Never sent a ping, check if enough time has passed since last activity
                self.last_activity.elapsed() >= Duration::from_secs(self.ping_frequency)
            }
        }
    }

    /// Check if connection has timed out
    pub fn is_timed_out(&self) -> bool {
        self.last_activity.elapsed() >= Duration::from_secs(self.connection_timeout)
    }

    /// Get connection age
    pub fn connection_age(&self) -> Duration {
        self.connected_at.elapsed()
    }

    /// Get time since last activity
    pub fn time_since_activity(&self) -> Duration {
        self.last_activity.elapsed()
    }

    /// Update timing parameters (useful for rehash/config updates)
    pub fn update_parameters(&mut self, ping_frequency: u64, connection_timeout: u64) {
        self.ping_frequency = ping_frequency;
        self.connection_timeout = connection_timeout;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sendq_basic() {
        let mut sendq = SendQueue::new(100);
        
        assert!(sendq.push("PRIVMSG #test :Hello\r\n".to_string()));
        assert_eq!(sendq.len(), 1);
        assert!(sendq.current_size() > 0);
        
        let msg = sendq.pop();
        assert_eq!(msg, Some("PRIVMSG #test :Hello\r\n".to_string()));
        assert_eq!(sendq.len(), 0);
        assert_eq!(sendq.current_size(), 0);
    }

    #[test]
    fn test_sendq_overflow() {
        let mut sendq = SendQueue::new(50);
        
        // This should succeed
        assert!(sendq.push("PRIVMSG #test :Hello\r\n".to_string()));
        
        // This should fail (would exceed buffer)
        assert!(!sendq.push("PRIVMSG #test :This is a very long message\r\n".to_string()));
        
        assert_eq!(sendq.dropped_messages(), 1);
    }

    #[test]
    fn test_recvq_basic() {
        let mut recvq = RecvQueue::new(1000);
        
        assert!(recvq.append("NICK test\r\n"));
        let messages = recvq.extract_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], "NICK test\r\n");
    }

    #[test]
    fn test_recvq_partial() {
        let mut recvq = RecvQueue::new(1000);
        
        // Add partial message
        assert!(recvq.append("NICK te"));
        let messages = recvq.extract_messages();
        assert_eq!(messages.len(), 0);
        
        // Complete the message
        assert!(recvq.append("st\r\n"));
        let messages = recvq.extract_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], "NICK test\r\n");
    }

    #[test]
    fn test_connection_timing() {
        let mut timing = ConnectionTiming::new(120, 300);
        
        assert!(!timing.is_timed_out());
        assert_eq!(timing.unanswered_pings, 0);
        
        timing.record_ping_sent();
        assert_eq!(timing.unanswered_pings, 1);
        
        timing.record_pong_received();
        assert_eq!(timing.unanswered_pings, 0);
    }
}

