//! Mesh network protocol definitions
//!
//! Defines all message types that can be sent over the mesh network.

use messaging::types::{GroupId, MemberId, PeerId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Transfer ID for file transfers
pub type TransferId = Uuid;

/// All message types that can be sent over the mesh network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeshMessage {
    /// Group chat message
    GroupMessage {
        group_id: GroupId,
        sender_id: MemberId,
        encrypted_content: Vec<u8>,
        timestamp: i64,
    },
    
    /// File transfer initiation
    FileTransferInit {
        transfer_id: TransferId,
        filename: String,
        file_size: u64,
        peer_id: PeerId,
    },
    
    /// File chunk data
    FileChunk {
        transfer_id: TransferId,
        chunk_index: usize,
        data: Vec<u8>,
        checksum: [u8; 32],
    },
    
    /// Acknowledgment for received chunk
    ChunkAck {
        transfer_id: TransferId,
        chunk_index: usize,
    },
    
    /// Onion-routed packet (future use)
    CircuitPacket {
        circuit_id: u64,
        encrypted_payload: Vec<u8>,
    },
    
    /// 1:1 Direct message (existing)
    DirectMessage {
        recipient: PeerId,
        encrypted_content: Vec<u8>,
        timestamp: i64,
    },
}

impl MeshMessage {
    /// Serialize message to bytes for transmission
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        bincode::serialize(self)
            .map_err(|e| format!("Serialization error: {}", e))
    }
    
    /// Deserialize message from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        bincode::deserialize(data)
            .map_err(|e| format!("Deserialization error: {}", e))
    }
    
    /// Get message type as string for logging
    pub fn message_type(&self) -> &str {
        match self {
            MeshMessage::GroupMessage { .. } => "GroupMessage",
            MeshMessage::FileTransferInit { .. } => "FileTransferInit",
            MeshMessage::FileChunk { .. } => "FileChunk",
            MeshMessage::ChunkAck { .. } => "ChunkAck",
            MeshMessage::CircuitPacket { .. } => "CircuitPacket",
            MeshMessage::DirectMessage { .. } => "DirectMessage",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_serialization() {
        let msg = MeshMessage::GroupMessage {
            group_id: GroupId([1u8; 32]),
            sender_id: MemberId([2u8; 32]),
            encrypted_content: vec![1, 2, 3, 4],
            timestamp: 1234567890,
        };
        
        let bytes = msg.to_bytes().unwrap();
        let decoded = MeshMessage::from_bytes(&bytes).unwrap();
        
        assert_eq!(msg.message_type(), decoded.message_type());
    }
    
    #[test]
    fn test_message_type() {
        let msg = MeshMessage::FileChunk {
            transfer_id: Uuid::new_v4(),
            chunk_index: 0,
            data: vec![],
            checksum: [0u8; 32],
        };
        
        assert_eq!(msg.message_type(), "FileChunk");
    }
}
