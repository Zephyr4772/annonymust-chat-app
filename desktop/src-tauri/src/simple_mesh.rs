//! Simplified mesh network handler for MVP
//! Uses TCP for simple node-to-node communication
//! Can be upgraded to full offline_mesh later

use anyhow::{Context, Result};
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::message_router::MessageRouter;

/// Simple mesh network for MVP
#[derive(Clone)]
pub struct SimpleMeshNetwork {
    listen_addr: SocketAddr,
    router: Arc<Mutex<Option<MessageRouter>>>,
    running: Arc<AtomicBool>,
    peers: Arc<Mutex<Vec<SocketAddr>>>,
}

impl SimpleMeshNetwork {
    /// Create new mesh network
    pub fn new(listen_addr: SocketAddr, router: Arc<Mutex<Option<MessageRouter>>>) -> Self {
        Self {
            listen_addr,
            router,
            running: Arc::new(AtomicBool::new(false)),
            peers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Add a known peer
    pub async fn add_peer(&self, peer_addr: SocketAddr) {
        let mut peers = self.peers.lock().await;
        if !peers.contains(&peer_addr) {
            peers.push(peer_addr);
            log::info!("Added peer: {}", peer_addr);
        }
    }
    
    /// Broadcast message to all peers
    pub async fn broadcast(&self, message: &[u8]) -> Result<usize> {
        let peers = self.peers.lock().await.clone();
        let mut sent_count = 0;
        
        for peer in peers {
            match self.send_to_peer(&peer, message).await {
                Ok(_) => {
                    sent_count += 1;
                    log::debug!("Sent {} bytes to {}", message.len(), peer);
                }
                Err(e) => {
                    log::warn!("Failed to send to {}: {}", peer, e);
                }
            }
        }
        
        Ok(sent_count)
    }
    
    async fn send_to_peer(&self, peer: &SocketAddr, message: &[u8]) -> Result<()> {
        let mut stream = tokio::net::TcpStream::connect(peer).await
            .context("Failed to connect to peer")?;
        
        // Send length prefix (4 bytes)
        let len = message.len() as u32;
        stream.write_all(&len.to_be_bytes()).await?;
        
        // Send message
        stream.write_all(message).await?;
        stream.flush().await?;
        
        Ok(())
    }
    
    /// Start listening for incoming connections
    pub async fn start(&self) -> Result<()> {
        self.running.store(true, Ordering::SeqCst);
        
        let listener = tokio::net::TcpListener::bind(self.listen_addr).await
            .context("Failed to bind listener")?;
        
        log::info!("Mesh network listening on {}", self.listen_addr);
        
        let router = self.router.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        log::debug!("Accepted connection from {}", addr);
                        let router_clone = router.clone();
                        
                        tokio::spawn(async move {
                            if let Err(e) = Self::handle_connection(stream, router_clone).await {
                                log::error!("Connection error from {}: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        log::error!("Accept error: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    async fn handle_connection(
        mut stream: tokio::net::TcpStream,
        router: Arc<tokio::sync::Mutex<Option<MessageRouter>>>,
    ) -> Result<()> {
        // Read length prefix
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        // Sanity check (max 10MB)
        if len > 10_000_000 {
            return Err(anyhow::anyhow!("Message too large: {} bytes", len));
        }
        
        // Read message
        let mut buffer = vec![0u8; len];
        stream.read_exact(&mut buffer).await?;
        
        // Route to MessageRouter
        let router_lock = router.lock().await;
        if let Some(router_ref) = router_lock.as_ref() {
            router_ref.route_incoming(&buffer).await?;
        }
        
        log::debug!("Received and routed {} bytes", len);
        
        Ok(())
    }
    
    /// Stop the network
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        log::info!("Mesh network stopped");
    }
    
    /// Get number of connected peers
    pub async fn peer_count(&self) -> usize {
        self.peers.lock().await.len()
    }
}
