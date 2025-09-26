//! Network-wide query system for IRC daemon

use crate::{Message, User, Error, Result, Database, ServerInfo};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};

/// Network query types
#[derive(Debug, Clone)]
pub enum NetworkQuery {
    /// WHO query across network
    Who {
        pattern: String,
        requestor: Uuid,
        request_id: String,
    },
    /// WHOIS query across network
    Whois {
        nickname: String,
        requestor: Uuid,
        request_id: String,
    },
    /// WHOWAS query across network
    Whowas {
        nickname: String,
        requestor: Uuid,
        request_id: String,
    },
    /// User count query
    UserCount {
        requestor: Uuid,
        request_id: String,
    },
    /// Server list query
    ServerList {
        requestor: Uuid,
        request_id: String,
    },
}

/// Network query response
#[derive(Debug, Clone)]
pub enum NetworkResponse {
    /// WHO response
    WhoResponse {
        request_id: String,
        server: String,
        users: Vec<User>,
    },
    /// WHOIS response
    WhoisResponse {
        request_id: String,
        server: String,
        user: Option<User>,
    },
    /// WHOWAS response
    WhowasResponse {
        request_id: String,
        server: String,
        users: Vec<User>,
    },
    /// User count response
    UserCountResponse {
        request_id: String,
        server: String,
        count: u32,
    },
    /// Server list response
    ServerListResponse {
        request_id: String,
        server: String,
        servers: Vec<ServerInfo>,
    },
    /// Error response
    ErrorResponse {
        request_id: String,
        server: String,
        error: String,
    },
}

/// Pending network query
#[derive(Debug)]
pub struct PendingQuery {
    pub query: NetworkQuery,
    pub created_at: DateTime<Utc>,
    pub timeout: Duration,
    pub responses: Vec<NetworkResponse>,
    pub expected_servers: Vec<String>,
}

/// Network query manager
#[derive(Debug)]
pub struct NetworkQueryManager {
    /// Pending queries by request ID
    pending_queries: Arc<RwLock<HashMap<String, PendingQuery>>>,
    /// Query timeout duration
    default_timeout: Duration,
    /// Maximum concurrent queries
    max_concurrent_queries: usize,
}

impl NetworkQueryManager {
    /// Create a new network query manager
    pub fn new(default_timeout_seconds: u64, max_concurrent_queries: usize) -> Self {
        Self {
            pending_queries: Arc::new(RwLock::new(HashMap::new())),
            default_timeout: Duration::seconds(default_timeout_seconds as i64),
            max_concurrent_queries,
        }
    }

    /// Submit a network query
    pub async fn submit_query(
        &self,
        query: NetworkQuery,
        expected_servers: Vec<String>,
    ) -> Result<String> {
        let request_id = Uuid::new_v4().to_string();
        
        // Check if we have too many pending queries
        let pending_count = {
            let queries = self.pending_queries.read().await;
            queries.len()
        };
        
        if pending_count >= self.max_concurrent_queries {
            return Err(Error::User("Too many pending queries".to_string()));
        }

        let pending_query = PendingQuery {
            query: query.clone(),
            created_at: Utc::now(),
            timeout: self.default_timeout,
            responses: Vec::new(),
            expected_servers,
        };

        {
            let mut queries = self.pending_queries.write().await;
            queries.insert(request_id.clone(), pending_query);
        }

        // Start timeout task
        self.start_timeout_task(request_id.clone()).await;

        Ok(request_id)
    }

    /// Handle a network response
    pub async fn handle_response(&self, response: NetworkResponse) -> Result<()> {
        let request_id = match &response {
            NetworkResponse::WhoResponse { request_id, .. } => request_id,
            NetworkResponse::WhoisResponse { request_id, .. } => request_id,
            NetworkResponse::WhowasResponse { request_id, .. } => request_id,
            NetworkResponse::UserCountResponse { request_id, .. } => request_id,
            NetworkResponse::ServerListResponse { request_id, .. } => request_id,
            NetworkResponse::ErrorResponse { request_id, .. } => request_id,
        };

        let mut queries = self.pending_queries.write().await;
        if let Some(pending_query) = queries.get_mut(request_id) {
            pending_query.responses.push(response);
        }

        Ok(())
    }

    /// Get query results
    pub async fn get_query_results(&self, request_id: &str) -> Result<Vec<NetworkResponse>> {
        let queries = self.pending_queries.read().await;
        if let Some(pending_query) = queries.get(request_id) {
            Ok(pending_query.responses.clone())
        } else {
            Err(Error::User("Query not found".to_string()))
        }
    }

    /// Check if query is complete
    pub async fn is_query_complete(&self, request_id: &str) -> Result<bool> {
        let queries = self.pending_queries.read().await;
        if let Some(pending_query) = queries.get(request_id) {
            let expected_count = pending_query.expected_servers.len();
            let response_count = pending_query.responses.len();
            Ok(response_count >= expected_count)
        } else {
            Err(Error::User("Query not found".to_string()))
        }
    }

    /// Remove completed query
    pub async fn remove_query(&self, request_id: &str) -> Result<()> {
        let mut queries = self.pending_queries.write().await;
        queries.remove(request_id);
        Ok(())
    }

    /// Start timeout task for a query
    async fn start_timeout_task(&self, request_id: String) {
        let queries = self.pending_queries.clone();
        let timeout = self.default_timeout;

        tokio::spawn(async move {
            tokio::time::sleep(timeout.to_std().unwrap()).await;
            
            let mut queries = queries.write().await;
            if let Some(pending_query) = queries.get(&request_id) {
                // Check if query has timed out
                if Utc::now() - pending_query.created_at > timeout {
                    queries.remove(&request_id);
                    tracing::warn!("Query {} timed out", request_id);
                }
            }
        });
    }

    /// Clean up expired queries
    pub async fn cleanup_expired_queries(&self) -> Result<()> {
        let now = Utc::now();
        let mut queries = self.pending_queries.write().await;
        
        queries.retain(|request_id, pending_query| {
            let is_expired = now - pending_query.created_at > pending_query.timeout;
            if is_expired {
                tracing::debug!("Cleaning up expired query: {}", request_id);
            }
            !is_expired
        });

        Ok(())
    }

    /// Get pending query count
    pub async fn pending_query_count(&self) -> usize {
        let queries = self.pending_queries.read().await;
        queries.len()
    }
}

/// Network message types for server-to-server communication
#[derive(Debug, Clone)]
pub enum NetworkMessage {
    /// User information
    UserInfo {
        user: User,
        server: String,
    },
    /// User quit
    UserQuit {
        nickname: String,
        reason: Option<String>,
        server: String,
    },
    /// User join channel
    UserJoin {
        nickname: String,
        channel: String,
        server: String,
    },
    /// User part channel
    UserPart {
        nickname: String,
        channel: String,
        reason: Option<String>,
        server: String,
    },
    /// Channel message
    ChannelMessage {
        nickname: String,
        channel: String,
        message: String,
        server: String,
    },
    /// Private message
    PrivateMessage {
        from_nick: String,
        to_nick: String,
        message: String,
        server: String,
    },
    /// Server introduction
    ServerIntro {
        server: ServerInfo,
    },
    /// Server quit
    ServerQuit {
        server: String,
        reason: Option<String>,
    },
    /// Network query
    Query {
        query: NetworkQuery,
        from_server: String,
    },
    /// Network response
    Response {
        response: NetworkResponse,
        from_server: String,
    },
}

/// Network message handler
pub struct NetworkMessageHandler {
    database: Arc<Database>,
    query_manager: Arc<NetworkQueryManager>,
}

impl NetworkMessageHandler {
    /// Create a new network message handler
    pub fn new(database: Arc<Database>, query_manager: Arc<NetworkQueryManager>) -> Self {
        Self {
            database,
            query_manager,
        }
    }

    /// Handle incoming network message
    pub async fn handle_message(&self, message: NetworkMessage) -> Result<()> {
        match message {
            NetworkMessage::UserInfo { user, server } => {
                self.handle_user_info(user, server).await?;
            }
            NetworkMessage::UserQuit { nickname, reason, server } => {
                self.handle_user_quit(nickname, reason, server).await?;
            }
            NetworkMessage::UserJoin { nickname, channel, server } => {
                self.handle_user_join(nickname, channel, server).await?;
            }
            NetworkMessage::UserPart { nickname, channel, reason, server } => {
                self.handle_user_part(nickname, channel, reason, server).await?;
            }
            NetworkMessage::ChannelMessage { nickname, channel, message, server } => {
                self.handle_channel_message(nickname, channel, message, server).await?;
            }
            NetworkMessage::PrivateMessage { from_nick, to_nick, message, server } => {
                self.handle_private_message(from_nick, to_nick, message, server).await?;
            }
            NetworkMessage::ServerIntro { server } => {
                self.handle_server_intro(server).await?;
            }
            NetworkMessage::ServerQuit { server, reason } => {
                self.handle_server_quit(server, reason).await?;
            }
            NetworkMessage::Query { query, from_server } => {
                self.handle_network_query(query, from_server).await?;
            }
            NetworkMessage::Response { response, from_server } => {
                self.handle_network_response(response, from_server).await?;
            }
        }
        Ok(())
    }

    async fn handle_user_info(&self, user: User, server: String) -> Result<()> {
        // Add user to database with server information
        let mut user_with_server = user;
        user_with_server.server = server;
        self.database.add_user(user_with_server)?;
        Ok(())
    }

    async fn handle_user_quit(&self, nickname: String, reason: Option<String>, server: String) -> Result<()> {
        // Remove user from database
        if let Some(user) = self.database.get_user_by_nick(&nickname) {
            self.database.remove_user(user.id)?;
        }
        Ok(())
    }

    async fn handle_user_join(&self, nickname: String, channel: String, server: String) -> Result<()> {
        // Add user to channel
        self.database.add_user_to_channel(&nickname, &channel)?;
        Ok(())
    }

    async fn handle_user_part(&self, nickname: String, channel: String, reason: Option<String>, server: String) -> Result<()> {
        // Remove user from channel
        self.database.remove_user_from_channel(&nickname, &channel)?;
        Ok(())
    }

    async fn handle_channel_message(&self, nickname: String, channel: String, message: String, server: String) -> Result<()> {
        // Forward channel message to local users in channel
        // This would integrate with the broadcast system
        tracing::debug!("Channel message from {} in {}: {}", nickname, channel, message);
        Ok(())
    }

    async fn handle_private_message(&self, from_nick: String, to_nick: String, message: String, server: String) -> Result<()> {
        // Forward private message to local user
        tracing::debug!("Private message from {} to {}: {}", from_nick, to_nick, message);
        Ok(())
    }

    async fn handle_server_intro(&self, server: ServerInfo) -> Result<()> {
        // Add server to database
        self.database.add_server(server)?;
        Ok(())
    }

    async fn handle_server_quit(&self, server: String, reason: Option<String>) -> Result<()> {
        // Remove server from database
        self.database.remove_server(&server);
        Ok(())
    }

    async fn handle_network_query(&self, query: NetworkQuery, from_server: String) -> Result<()> {
        // Process network query and send response
        match query {
            NetworkQuery::Who { pattern, requestor, request_id } => {
                let users = self.database.search_users(&pattern);
                let response = NetworkResponse::WhoResponse {
                    request_id,
                    server: "localhost".to_string(), // TODO: Get actual server name
                    users,
                };
                // Send response back to requesting server
                self.send_network_response(response, from_server).await?;
            }
            NetworkQuery::Whois { nickname, requestor, request_id } => {
                let user = self.database.get_user_by_nick(&nickname);
                let response = NetworkResponse::WhoisResponse {
                    request_id,
                    server: "localhost".to_string(), // TODO: Get actual server name
                    user,
                };
                self.send_network_response(response, from_server).await?;
            }
            NetworkQuery::Whowas { nickname, requestor, request_id } => {
                let users = self.database.get_user_history(&nickname).await;
                let response = NetworkResponse::WhowasResponse {
                    request_id,
                    server: "localhost".to_string(), // TODO: Get actual server name
                    users: users.into_iter().map(|entry| entry.user).collect(),
                };
                self.send_network_response(response, from_server).await?;
            }
            NetworkQuery::UserCount { requestor, request_id } => {
                let count = self.database.user_count() as u32;
                let response = NetworkResponse::UserCountResponse {
                    request_id,
                    server: "localhost".to_string(), // TODO: Get actual server name
                    count,
                };
                self.send_network_response(response, from_server).await?;
            }
            NetworkQuery::ServerList { requestor, request_id } => {
                let servers = self.database.get_all_servers();
                let response = NetworkResponse::ServerListResponse {
                    request_id,
                    server: "localhost".to_string(), // TODO: Get actual server name
                    servers,
                };
                self.send_network_response(response, from_server).await?;
            }
        }
        Ok(())
    }

    async fn handle_network_response(&self, response: NetworkResponse, from_server: String) -> Result<()> {
        // Handle network response
        self.query_manager.handle_response(response).await?;
        Ok(())
    }

    async fn send_network_response(&self, response: NetworkResponse, to_server: String) -> Result<()> {
        // Send response to server
        // This would integrate with the server connection system
        tracing::debug!("Sending response to server {}: {:?}", to_server, response);
        Ok(())
    }
}

/// Helper functions for network queries
impl NetworkQueryManager {
    /// Submit a WHO query across the network
    pub async fn query_who(&self, pattern: String, requestor: Uuid, servers: Vec<String>) -> Result<String> {
        let query = NetworkQuery::Who {
            pattern,
            requestor,
            request_id: Uuid::new_v4().to_string(),
        };
        self.submit_query(query, servers).await
    }

    /// Submit a WHOIS query across the network
    pub async fn query_whois(&self, nickname: String, requestor: Uuid, servers: Vec<String>) -> Result<String> {
        let query = NetworkQuery::Whois {
            nickname,
            requestor,
            request_id: Uuid::new_v4().to_string(),
        };
        self.submit_query(query, servers).await
    }

    /// Submit a WHOWAS query across the network
    pub async fn query_whowas(&self, nickname: String, requestor: Uuid, servers: Vec<String>) -> Result<String> {
        let query = NetworkQuery::Whowas {
            nickname,
            requestor,
            request_id: Uuid::new_v4().to_string(),
        };
        self.submit_query(query, servers).await
    }
}
