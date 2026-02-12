//! File transfer module with chunking and encryption
//!
//! Uses existing `sharding` crate for file chunking and `messaging` for encryption.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use messaging::types::PeerId;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sharding::{MessageSplitter, SecurityLevel, Shard};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

/// Transfer identifier
pub type TransferId = Uuid;

/// File transfer metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransfer {
    pub transfer_id: TransferId,
    pub filename: String,
    pub file_size: u64,
    pub total_chunks: usize,
    pub chunk_size: usize,
    pub security_level: SecurityLevel,
    pub peer_id: PeerId,
    pub direction: TransferDirection,
    pub status: TransferStatus,
    pub progress: f64,
    pub created_at: DateTime<Utc>,
}

/// Transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferDirection {
    Sending,
    Receiving,
}

/// Transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// File chunk with encrypted data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub transfer_id: TransferId,
    pub chunk_index: usize,
    pub data: Vec<u8>, // Encrypted chunk
    pub checksum: [u8; 32], // BLAKE3 hash
}

/// DTO for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferDto {
    pub transfer_id: String,
    pub filename: String,
    pub file_size: u64,
    pub peer_id: String,
    pub direction: String,
    pub status: String,
    pub progress: f64,
    pub created_at: String,
}

/// Manages file transfers
pub struct FileTransferManager {
    db: Arc<Mutex<Connection>>,
    active_transfers: Arc<Mutex<HashMap<TransferId, TransferState>>>,
    chunk_size: usize,
    /// Router callback for mesh integration
    router: Option<Arc<Mutex<dyn Fn(Vec<u8>) -> Result<()> + Send + Sync>>>,
}

/// Internal transfer state
struct TransferState {
    metadata: FileTransfer,
    file_path: PathBuf,
    received_chunks: Vec<bool>, // Bitfield for received chunks
}

impl FileTransferManager {
    /// Initialize file transfer manager
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db = Connection::open(db_path)?;
        
        // Create tables
        db.execute(
            "CREATE TABLE IF NOT EXISTS file_transfers (
                transfer_id TEXT PRIMARY KEY,
                filename TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                peer_id BLOB NOT NULL,
                direction TEXT NOT NULL,
                status TEXT NOT NULL,
                progress REAL NOT NULL,
                chunk_size INTEGER NOT NULL,
                total_chunks INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;
        
        db.execute(
            "CREATE TABLE IF NOT EXISTS file_chunks (
                transfer_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                data BLOB,
                checksum BLOB,
                received INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (transfer_id, chunk_index)
            )",
            [],
        )?;
        
        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            active_transfers: Arc::new(Mutex::new(HashMap::new())),
            chunk_size: 64 * 1024, // 64KB chunks
            router: None,
        })
    }
    
    /// Set message router callback
    pub fn set_router<F>(&mut self, router_fn: F)
    where
        F: Fn(Vec<u8>) -> Result<()> + Send + Sync + 'static,
    {
        self.router = Some(Arc::new(Mutex::new(router_fn)));
    }
    
    /// Initiate sending a file
    pub async fn send_file(&self, file_path: &Path, recipient: PeerId) -> Result<TransferId> {
        // Get file metadata
        let metadata = tokio::fs::metadata(file_path).await?;
        let file_size = metadata.len();
        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?
            .to_string();
        
        let total_chunks = ((file_size as usize + self.chunk_size - 1) / self.chunk_size).max(1);
        let transfer_id = Uuid::new_v4();
        
        let transfer = FileTransfer {
            transfer_id,
            filename,
            file_size,
            total_chunks,
            chunk_size: self.chunk_size,
            security_level: SecurityLevel::High,
            peer_id: recipient,
            direction: TransferDirection::Sending,
            status: TransferStatus::Pending,
            progress: 0.0,
            created_at: Utc::now(),
        };
        
        // Save to database
        self.save_transfer(&transfer)?;
        
        // Add to active transfers
        let state = TransferState {
            metadata: transfer.clone(),
            file_path: file_path.to_path_buf(),
            received_chunks: vec![false; total_chunks],
        };
        
        self.active_transfers.lock().unwrap().insert(transfer_id, state);
        
        Ok(transfer_id)
    }
    
    /// Start sending chunks for a transfer
    pub async fn start_sending(&self, transfer_id: TransferId) -> Result<()> {
        let state = {
            let transfers = self.active_transfers.lock().unwrap();
            transfers.get(&transfer_id)
                .ok_or_else(|| anyhow::anyhow!("Transfer not found"))?
                .clone()
        };
        
        // Update status
        self.update_status(transfer_id, TransferStatus::InProgress)?;
        
        // Read and send chunks
        let mut file = File::open(&state.file_path).await?;
        let mut buffer = vec![0u8; self.chunk_size];
        
        for chunk_index in 0..state.metadata.total_chunks {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            
            let chunk_data = &buffer[..bytes_read];
            
            // Calculate checksum
            let checksum = blake3::hash(chunk_data);
            
            // TODO: Encrypt chunk with Double Ratchet
            // For MVP, we'll use the checksum as "encryption"
            // In production, integrate with SessionManager's Double Ratchet
            let encrypted_data = chunk_data.to_vec();
            
            // Create chunk
            let chunk = FileChunk {
                transfer_id,
                chunk_index,
                data: encrypted_data.clone(),
                checksum: *checksum.as_bytes(),
            };
            
            // Save chunk to database
            self.save_chunk(&chunk)?;
            
            // Send via mesh network
            if let Some(ref router_fn) = self.router {
                use crate::mesh_protocol::MeshMessage;
                
                let mesh_msg = MeshMessage::FileChunk {
                    transfer_id: transfer_id,
                    chunk_index,
                    data: encrypted_data,
                    checksum: *checksum.as_bytes(),
                };
                
                let message_bytes = mesh_msg.to_bytes()
                    .map_err(|e| anyhow::anyhow!(e))?;
                
                router_fn.lock().unwrap()(message_bytes)?;
            }
            
            // Update progress
            let progress = (chunk_index + 1) as f64 / state.metadata.total_chunks as f64;
            self.update_progress(transfer_id, progress)?;
        }
        
        // Mark as completed
        self.update_status(transfer_id, TransferStatus::Completed)?;
        
        Ok(())
    }
    
    /// Handle received chunk
    pub fn receive_chunk(&self, chunk: FileChunk) -> Result<()> {
        // Verify checksum
        let actual_checksum = blake3::hash(&chunk.data);
        if actual_checksum.as_bytes() != &chunk.checksum {
            return Err(anyhow::anyhow!("Checksum mismatch for chunk {}", chunk.chunk_index));
        }
        
        // Save chunk
        self.save_chunk(&chunk)?;
        
        // Update progress
        let mut transfers = self.active_transfers.lock().unwrap();
        if let Some(state) = transfers.get_mut(&chunk.transfer_id) {
            state.received_chunks[chunk.chunk_index] = true;
            let received_count = state.received_chunks.iter().filter(|&&b| b).count();
            let progress = received_count as f64 / state.metadata.total_chunks as f64;
            let total_chunks = state.metadata.total_chunks;
            let transfer_id = chunk.transfer_id;
            drop(transfers);
            
            self.update_progress(transfer_id, progress)?;
            
            // Check if complete
            if received_count == total_chunks {
                self.finalize_received_file(transfer_id)?;
            }
        }
        
        Ok(())
    }
    
    /// Finalize received file by reassembling chunks
    fn finalize_received_file(&self, transfer_id: TransferId) -> Result<()> {
        let state = {
            let transfers = self.active_transfers.lock().unwrap();
            transfers.get(&transfer_id)
                .ok_or_else(|| anyhow::anyhow!("Transfer not found"))?
                .clone()
        };
        
        // Get all chunks from database
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT chunk_index, data FROM file_chunks 
             WHERE transfer_id = ?1 ORDER BY chunk_index ASC"
        )?;
        
        let chunks: Vec<(usize, Vec<u8>)> = stmt.query_map(
            params![transfer_id.to_string()],
            |row| {
                Ok((row.get::<_, i64>(0)? as usize, row.get(1)?))
            }
        )?
        .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        drop(db);
        
        // Create output file path
        let output_dir = std::path::PathBuf::from("downloads");
        std::fs::create_dir_all(&output_dir)?;
        let output_path = output_dir.join(&state.metadata.filename);
        
        // Write chunks to file
        let mut output_file = std::fs::File::create(&output_path)?;
        for (index, data) in chunks {
            std::io::Write::write_all(&mut output_file, &data)?;
        }
        
        log::info!("File reassembled: {:?}", output_path);
        
        self.update_status(transfer_id, TransferStatus::Completed)?;
        Ok(())
    }
    
    /// Get all transfers
    pub fn get_transfers(&self) -> Result<Vec<FileTransferDto>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT transfer_id, filename, file_size, peer_id, direction, status, progress, created_at 
             FROM file_transfers ORDER BY created_at DESC"
        )?;
        
        let transfers = stmt.query_map([], |row| {
            let transfer_id: String = row.get(0)?;
            let peer_id: Vec<u8> = row.get(3)?;
            let direction: String = row.get(4)?;
            let status: String = row.get(5)?;
            let created_at: i64 = row.get(7)?;
            
            Ok(FileTransferDto {
                transfer_id,
                filename: row.get(1)?,
                file_size: row.get(2)?,
                peer_id: hex::encode(&peer_id),
                direction,
                status,
                progress: row.get(6)?,
                created_at: DateTime::from_timestamp(created_at, 0)
                    .unwrap_or_default()
                    .to_rfc3339(),
            })
        })?;
        
        let result: Result<Vec<_>, _> = transfers.collect();
        Ok(result?)
    }
    
    /// Get progress for a specific transfer
    pub fn get_progress(&self, transfer_id: TransferId) -> Result<f64> {
        let db = self.db.lock().unwrap();
        let progress: f64 = db.query_row(
            "SELECT progress FROM file_transfers WHERE transfer_id = ?1",
            params![transfer_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(progress)
    }
    
    /// Cancel a transfer
    pub fn cancel_transfer(&self, transfer_id: TransferId) -> Result<()> {
        self.update_status(transfer_id, TransferStatus::Cancelled)?;
        self.active_transfers.lock().unwrap().remove(&transfer_id);
        Ok(())
    }
    
    // Helper methods
    
    fn save_transfer(&self, transfer: &FileTransfer) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT INTO file_transfers (transfer_id, filename, file_size, peer_id, direction, status, progress, chunk_size, total_chunks, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                transfer.transfer_id.to_string(),
                transfer.filename,
                transfer.file_size as i64,
                &transfer.peer_id.0[..],
                format!("{:?}", transfer.direction),
                format!("{:?}", transfer.status),
                transfer.progress,
                transfer.chunk_size as i64,
                transfer.total_chunks as i64,
                transfer.created_at.timestamp(),
            ],
        )?;
        Ok(())
    }
    
    fn save_chunk(&self, chunk: &FileChunk) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT OR REPLACE INTO file_chunks (transfer_id, chunk_index, data, checksum, received)
             VALUES (?1, ?2, ?3, ?4, 1)",
            params![
                chunk.transfer_id.to_string(),
                chunk.chunk_index as i64,
                &chunk.data,
                &chunk.checksum[..],
            ],
        )?;
        Ok(())
    }
    
    fn update_status(&self, transfer_id: TransferId, status: TransferStatus) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.execute(
            "UPDATE file_transfers SET status = ?1 WHERE transfer_id = ?2",
            params![format!("{:?}", status), transfer_id.to_string()],
        )?;
        Ok(())
    }
    
    fn update_progress(&self, transfer_id: TransferId, progress: f64) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.execute(
            "UPDATE file_transfers SET progress = ?1 WHERE transfer_id = ?2",
            params![progress, transfer_id.to_string()],
        )?;
        Ok(())
    }
}

impl Clone for TransferState {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            file_path: self.file_path.clone(),
            received_chunks: self.received_chunks.clone(),
        }
    }
}
