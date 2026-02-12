//! Message router for dispatching mesh network messages
//!
//! Routes incoming messages to appropriate managers and sends outgoing messages via mesh.

use crate::circuit_manager::CircuitManager;
use crate::file_transfer::FileTransferManager;
use crate::group_manager::GroupManager;
use crate::mesh_protocol::MeshMessage;
use crate::simple_mesh::SimpleMeshNetwork;
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

/// Routes messages between mesh network and feature managers
pub struct MessageRouter {
    group_manager: Arc<TokioMutex<Option<GroupManager>>>,
    file_transfer_manager: Arc<TokioMutex<Option<FileTransferManager>>>,
    circuit_manager: Arc<TokioMutex<Option<CircuitManager>>>,
    mesh_network: Option<Arc<TokioMutex<Option<SimpleMeshNetwork>>>>,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new(
        group_manager: Arc<TokioMutex<Option<GroupManager>>>,
        file_transfer_manager: Arc<TokioMutex<Option<FileTransferManager>>>,
        circuit_manager: Arc<TokioMutex<Option<CircuitManager>>>,
    ) -> Self {
        Self {
            group_manager,
            file_transfer_manager,
            circuit_manager,
            mesh_network: None,
        }
    }
    
    /// Set mesh network for outgoing messages
    pub fn set_mesh_network(&mut self, network: Arc<TokioMutex<Option<SimpleMeshNetwork>>>) {
        self.mesh_network = Some(network);
        log::info!("Mesh network connected to MessageRouter");
    }
    
    /// Route incoming message from mesh network
    pub fn route_incoming<'a>(&'a self, message_bytes: &'a [u8]) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
        // Deserialize message
        let message = MeshMessage::from_bytes(message_bytes)
            .map_err(|e| anyhow::anyhow!(e))?;
        
        log::debug!("Routing incoming message: {}", message.message_type());
        
        // Dispatch to appropriate handler
        match message {
            MeshMessage::GroupMessage { group_id, sender_id, encrypted_content, timestamp } => {
                self.handle_group_message(group_id, sender_id, encrypted_content, timestamp).await?;
            }
            
            MeshMessage::FileTransferInit { transfer_id, filename, file_size, peer_id } => {
                self.handle_file_transfer_init(transfer_id, filename, file_size, peer_id).await?;
            }
            
            MeshMessage::FileChunk { transfer_id, chunk_index, data, checksum } => {
                self.handle_file_chunk(transfer_id, chunk_index, data, checksum).await?;
            }
            
            MeshMessage::ChunkAck { transfer_id, chunk_index } => {
                self.handle_chunk_ack(transfer_id, chunk_index).await?;
            }
            
            MeshMessage::CircuitPacket { circuit_id, encrypted_payload } => {
                self.handle_circuit_packet(circuit_id, encrypted_payload).await?;
            }
            
            MeshMessage::DirectMessage { recipient, encrypted_content, timestamp } => {
                self.handle_direct_message(recipient, encrypted_content, timestamp).await?;
            }
        }
        
        Ok(())
        })
    }
    
    /// Send outgoing message via mesh network
    pub fn send_outgoing(&self, message: MeshMessage) -> Result<Vec<u8>> {
        log::debug!("Sending outgoing message: {}", message.message_type());
        
        // Serialize message
        let bytes = message.to_bytes()
            .map_err(|e| anyhow::anyhow!(e))?;
        
        self.send_outgoing_bytes(&bytes)?;
        
        Ok(bytes)
    }
    
    /// Send raw message bytes via mesh network
    pub fn send_outgoing_bytes(&self, bytes: &[u8]) -> Result<()> {
        if let Some(mesh_ref) = &self.mesh_network {
            // Async operation - need to use tokio runtime
            let mesh_clone = mesh_ref.clone();
            let bytes_vec = bytes.to_vec();
            
            tokio::spawn(async move {
                let mesh_lock = mesh_clone.lock().await;
                if let Some(mesh) = mesh_lock.as_ref() {
                    match mesh.broadcast(&bytes_vec).await {
                        Ok(count) => {
                            log::info!("Broadcast {} bytes to {} peers", bytes_vec.len(), count);
                        }
                        Err(e) => {
                            log::error!("Failed to broadcast: {}", e);
                        }
                    }
                }
            });
            
            Ok(())
        } else {
            log::warn!("Mesh network not connected - message not sent");
            Ok(())
        }
    }
    
    // Handler methods
    
    async fn handle_group_message(
        &self,
        group_id: messaging::types::GroupId,
        sender_id: messaging::types::MemberId,
        encrypted_content: Vec<u8>,
        timestamp: i64,
    ) -> Result<()> {
        let manager_lock = self.group_manager.lock().await;
        let manager = manager_lock.as_ref()
            .context("Group manager not initialized")?;
        
        // Store the message
        manager.receive_message(
            group_id,
            sender_id,
            &encrypted_content,
            timestamp
        )?;
        
        log::info!("Routed group message for group: {:?}", hex::encode(&group_id));
        
        Ok(())
    }
    
    async fn handle_file_transfer_init(
        &self,
        transfer_id: uuid::Uuid,
        filename: String,
        file_size: u64,
        peer_id: messaging::types::PeerId,
    ) -> Result<()> {
        let manager_lock = self.file_transfer_manager.lock().await;
        let _manager = manager_lock.as_ref()
            .context("File transfer manager not initialized")?;
        
        // TODO: Create incoming transfer entry
        log::info!(
            "Received file transfer init: {} ({} bytes) from {:?}",
            filename, file_size, hex::encode(peer_id.0)
        );
        
        Ok(())
    }
    
    async fn handle_file_chunk(
        &self,
        transfer_id: uuid::Uuid,
        chunk_index: usize,
        data: Vec<u8>,
        checksum: [u8; 32],
    ) -> Result<()> {
        use crate::file_transfer::FileChunk;
        
        let manager_lock = self.file_transfer_manager.lock().await;
        let manager = manager_lock.as_ref()
            .context("File transfer manager not initialized")?;
        
        let chunk = FileChunk {
            transfer_id,
            chunk_index,
            data,
            checksum,
        };
        
        manager.receive_chunk(chunk)?;
        
        log::debug!("Received file chunk {} for transfer {}", chunk_index, transfer_id);
        
        Ok(())
    }
    
    async fn handle_chunk_ack(
        &self,
        transfer_id: uuid::Uuid,
        chunk_index: usize,
    ) -> Result<()> {
        log::debug!("Received chunk ack {} for transfer {}", chunk_index, transfer_id);
        
        // TODO: Update sender's transfer state to mark chunk as acknowledged
        
        Ok(())
    }
    
    async fn handle_circuit_packet(
        &self,
        circuit_id: u64,
        encrypted_payload: Vec<u8>,
    ) -> Result<()> {
        let manager_lock = self.circuit_manager.lock().await;
        let manager = manager_lock.as_ref()
            .context("Circuit manager not initialized")?;
        
        // Decrypt onion layers
        let plaintext = manager.decrypt_from_circuit(circuit_id, &encrypted_payload)?;
        
        drop(manager_lock);
        
        // Route decrypted inner message
        self.route_incoming(&plaintext).await?;
        
        log::info!("Routed circuit packet for circuit {}", circuit_id);
        
        Ok(())
    }
    
    async fn handle_direct_message(
        &self,
        recipient: messaging::types::PeerId,
        encrypted_content: Vec<u8>,
        timestamp: i64,
    ) -> Result<()> {
        log::info!(
            "Received direct message for {:?} at {}",
            hex::encode(recipient.0), timestamp
        );
        
        // TODO: Integrate with SessionManager for 1:1 messages
        
        Ok(())
    }
    
    /// Send message through onion circuit
    pub async fn send_via_circuit(
        &self,
        circuit_id: u64,
        message: MeshMessage,
    ) -> Result<()> {
        // Serialize message
        let message_bytes = message.to_bytes()
            .map_err(|e| anyhow::anyhow!(e))?;
        
        // Get circuit manager
        let manager_lock = self.circuit_manager.lock().await;
        let manager = manager_lock.as_ref()
            .context("Circuit manager not initialized")?;
        
        // Encrypt with onion layers
        let onion_packet = manager.send_through_circuit(circuit_id, &message_bytes)?;
        
        drop(manager_lock);
        
        // Create circuit packet message
        let circuit_msg = MeshMessage::CircuitPacket {
            circuit_id,
            encrypted_payload: onion_packet,
        };
        
        // Send via mesh
        self.send_outgoing(circuit_msg)?;
        
        log::info!("Sent message through circuit {}", circuit_id);
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh_protocol::MeshMessage;
    
    #[test]
    fn test_message_serialization() {
        let msg = MeshMessage::GroupMessage {
            group_id: messaging::types::GroupId([1u8; 32]),
            sender_id: messaging::types::MemberId([2u8; 32]),
            encrypted_content: vec![1, 2, 3],
            timestamp: 1234567890,
        };
        
        let bytes = msg.to_bytes().unwrap();
        assert!(bytes.len() > 0);
        
        let decoded = MeshMessage::from_bytes(&bytes).unwrap();
        assert_eq!(msg.message_type(), decoded.message_type());
    }
}
