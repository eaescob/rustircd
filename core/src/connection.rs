//! Connection handling and management

use crate::{Client, Message, Error, Result};
use std::net::SocketAddr;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc,
};
use tokio_rustls::{TlsAcceptor, TlsStream};
use uuid::Uuid;

/// Connection handler for managing client connections
pub struct ConnectionHandler {
    /// Client ID to client mapping
    clients: std::collections::HashMap<Uuid, Client>,
    /// Nickname to client ID mapping
    nick_to_id: std::collections::HashMap<String, Uuid>,
    /// Message receiver for incoming messages
    message_receiver: mpsc::UnboundedReceiver<(Uuid, Message)>,
    /// Message sender for outgoing messages
    message_sender: mpsc::UnboundedSender<(Uuid, Message)>,
}

impl ConnectionHandler {
    /// Create a new connection handler
    pub fn new() -> (Self, mpsc::UnboundedSender<(Uuid, Message)>) {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        
        let handler = Self {
            clients: std::collections::HashMap::new(),
            nick_to_id: std::collections::HashMap::new(),
            message_receiver,
            message_sender: message_sender.clone(),
        };
        
        (handler, message_sender)
    }
    
    /// Handle a new client connection
    pub async fn handle_connection(
        &mut self,
        stream: TcpStream,
        remote_addr: SocketAddr,
        tls_acceptor: Option<TlsAcceptor>,
    ) -> Result<()> {
        let local_addr = stream.local_addr()?;
        let client_id = Uuid::new_v4();
        
        // Create message channel for this client
        let (client_sender, client_receiver) = mpsc::unbounded_channel();
        
        // Create client
        let client = Client::new(
            client_id,
            remote_addr.to_string(),
            local_addr.to_string(),
            client_sender,
        );
        
        // Store client
        self.clients.insert(client_id, client);
        
        // Handle TLS if acceptor is provided
        let stream = if let Some(acceptor) = tls_acceptor {
            let tls_stream = acceptor.accept(stream).await?;
            Box::new(tls_stream) as Box<dyn ConnectionStream>
        } else {
            Box::new(stream) as Box<dyn ConnectionStream>
        };
        
        // Spawn connection handler task
        let message_sender = self.message_sender.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::handle_client_connection(
                client_id,
                stream,
                client_receiver,
                message_sender,
            ).await {
                tracing::error!("Error handling client connection {}: {}", client_id, e);
            }
        });
        
        Ok(())
    }
    
    /// Handle individual client connection
    async fn handle_client_connection(
        client_id: Uuid,
        stream: Box<dyn ConnectionStream>,
        mut client_receiver: mpsc::UnboundedReceiver<Message>,
        message_sender: mpsc::UnboundedSender<(Uuid, Message)>,
    ) -> Result<()> {
        let (read_half, mut write_half) = stream.split();
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        
        // Send messages to client
        let _message_sender_clone = message_sender.clone();
        tokio::spawn(async move {
            while let Some(message) = client_receiver.recv().await {
                if let Err(e) = write_half.write_all(message.to_string().as_bytes()).await {
                    tracing::error!("Error writing to client {}: {}", client_id, e);
                    break;
                }
            }
        });
        
        // Read messages from client
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // Connection closed
                    break;
                }
                Ok(_) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    
                    match Message::parse(line) {
                        Ok(message) => {
                            if let Err(e) = message_sender.send((client_id, message)) {
                                tracing::error!("Error sending message: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Error parsing message from client {}: {}", client_id, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error reading from client {}: {}", client_id, e);
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Get client by ID
    pub fn get_client(&self, id: &Uuid) -> Option<&Client> {
        self.clients.get(id)
    }
    
    /// Get mutable client by ID
    pub fn get_client_mut(&mut self, id: &Uuid) -> Option<&mut Client> {
        self.clients.get_mut(id)
    }
    
    /// Get client by nickname
    pub fn get_client_by_nick(&self, nick: &str) -> Option<&Client> {
        self.nick_to_id.get(nick).and_then(|id| self.clients.get(id))
    }
    
    /// Get mutable client by nickname
    pub fn get_client_mut_by_nick(&mut self, nick: &str) -> Option<&mut Client> {
        self.nick_to_id.get(nick).and_then(|id| self.clients.get_mut(id))
    }
    
    /// Register client nickname
    pub fn register_nickname(&mut self, client_id: Uuid, nick: String) -> Result<()> {
        // Check if nickname is already in use
        if self.nick_to_id.contains_key(&nick) {
            return Err(Error::User("Nickname already in use".to_string()));
        }
        
        // Remove old nickname if exists
        if let Some(client) = self.clients.get(&client_id) {
            if let Some(old_nick) = client.nickname() {
                self.nick_to_id.remove(old_nick);
            }
        }
        
        // Register new nickname
        self.nick_to_id.insert(nick, client_id);
        Ok(())
    }
    
    /// Remove client
    pub fn remove_client(&mut self, client_id: &Uuid) {
        if let Some(client) = self.clients.remove(client_id) {
            if let Some(nick) = client.nickname() {
                self.nick_to_id.remove(nick);
            }
        }
    }
    
    /// Get all clients
    pub fn get_all_clients(&self) -> &std::collections::HashMap<Uuid, Client> {
        &self.clients
    }
    
    /// Get all registered clients
    pub fn get_registered_clients(&self) -> Vec<&Client> {
        self.clients.values().filter(|c| c.is_registered()).collect()
    }
    
    /// Broadcast message to all clients
    pub fn broadcast(&self, message: Message) -> Result<()> {
        for client in self.clients.values() {
            if let Err(e) = client.send(message.clone()) {
                tracing::warn!("Error broadcasting to client {}: {}", client.id, e);
            }
        }
        Ok(())
    }
    
    /// Broadcast message to all registered clients
    pub fn broadcast_registered(&self, message: Message) -> Result<()> {
        for client in self.clients.values() {
            if client.is_registered() {
                if let Err(e) = client.send(message.clone()) {
                    tracing::warn!("Error broadcasting to client {}: {}", client.id, e);
                }
            }
        }
        Ok(())
    }
}

/// Trait for connection streams (TCP or TLS)
pub trait ConnectionStream: Send + Sync {
    fn split(self: Box<Self>) -> (Box<dyn ConnectionReadHalf>, Box<dyn ConnectionWriteHalf>);
}

/// Trait for connection read half
pub trait ConnectionReadHalf: Send + Sync + tokio::io::AsyncRead + Unpin {
    // This will be implemented by tokio's AsyncRead
}

/// Trait for connection write half
pub trait ConnectionWriteHalf: Send + Sync + tokio::io::AsyncWrite + Unpin {
    // This will be implemented by tokio's AsyncWrite
}

// Implement traits for TcpStream
impl ConnectionStream for TcpStream {
    fn split(self: Box<Self>) -> (Box<dyn ConnectionReadHalf>, Box<dyn ConnectionWriteHalf>) {
        let (read, write) = tokio::io::split(*self);
        (Box::new(read), Box::new(write))
    }
}

impl ConnectionReadHalf for tokio::io::ReadHalf<TcpStream> {}
impl ConnectionWriteHalf for tokio::io::WriteHalf<TcpStream> {}

// Implement traits for TlsStream
impl<T> ConnectionStream for TlsStream<T>
where
    T: Send + Sync + tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + 'static,
{
    fn split(self: Box<Self>) -> (Box<dyn ConnectionReadHalf>, Box<dyn ConnectionWriteHalf>) {
        let (read, write) = tokio::io::split(*self);
        (Box::new(read), Box::new(write))
    }
}

impl<T> ConnectionReadHalf for tokio::io::ReadHalf<TlsStream<T>> 
where 
    T: Send + Sync + tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + 'static
{}

impl<T> ConnectionWriteHalf for tokio::io::WriteHalf<TlsStream<T>> 
where 
    T: Send + Sync + tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + 'static
{}

// Implement traits for tokio_rustls::server::TlsStream
impl ConnectionStream for tokio_rustls::server::TlsStream<tokio::net::TcpStream> {
    fn split(self: Box<Self>) -> (Box<dyn ConnectionReadHalf>, Box<dyn ConnectionWriteHalf>) {
        let (read, write) = tokio::io::split(*self);
        (Box::new(read), Box::new(write))
    }
}

impl ConnectionReadHalf for tokio::io::ReadHalf<tokio_rustls::server::TlsStream<tokio::net::TcpStream>> {}
impl ConnectionWriteHalf for tokio::io::WriteHalf<tokio_rustls::server::TlsStream<tokio::net::TcpStream>> {}
