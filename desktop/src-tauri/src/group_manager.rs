//! Group chat management with database persistence
//!
//! Integrates `messaging::group::GroupState` with SQLite for persistent group storage.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use messaging::group::{GroupInfo, GroupState, SenderKeyState};
use messaging::{MemberRole};
use messaging::types::{GroupId, MemberId};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Serializable group info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupDto {
    pub group_id: String,
    pub name: String,
    pub created_at: String,
    pub member_count: usize,
    pub is_admin: bool,
}

/// Serializable group message for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMessageDto {
    pub group_id: String,
    pub sender_id: String,
    pub content: String,
    pub timestamp: i64,
}

/// Manages all groups for this user
pub struct GroupManager {
    db: Arc<Mutex<Connection>>,
    /// Active groups (loaded in memory)
    groups: Arc<Mutex<HashMap<GroupId, GroupState>>>,
    /// Our member ID
    my_member_id: MemberId,
    /// Optional message router for mesh integration
    router: Option<Arc<Mutex<dyn Fn(Vec<u8>) -> Result<()> + Send + Sync>>>,
}

impl GroupManager {
    /// Initialize group manager with database
    pub fn new<P: AsRef<Path>>(db_path: P, my_member_id: MemberId) -> Result<Self> {
        let db = Connection::open(db_path)?;
        
        // Create tables
        db.execute(
            "CREATE TABLE IF NOT EXISTS groups (
                group_id BLOB PRIMARY KEY,
                name TEXT NOT NULL,
                admin_id BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                state_blob BLOB NOT NULL
            )",
            [],
        )?;
        
        db.execute(
            "CREATE TABLE IF NOT EXISTS group_members (
                group_id BLOB NOT NULL,
                member_id BLOB NOT NULL,
                role INTEGER NOT NULL,
                joined_at INTEGER NOT NULL,
                PRIMARY KEY (group_id, member_id)
            )",
            [],
        )?;
        
        db.execute(
            "CREATE TABLE IF NOT EXISTS group_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id BLOB NOT NULL,
                sender_id BLOB NOT NULL,
                content BLOB NOT NULL,
                timestamp INTEGER NOt NULL,x`
            )",
            [],
        )?;
        
        let manager = Self {
            db: Arc::new(Mutex::new(db)),
            groups: Arc::new(Mutex::new(HashMap::new())),
            my_member_id,
            router: None,
        };
        
        // Load existing groups from database
        manager.load_groups()?;
        
        Ok(manager)
    }
    
    /// Load all groups from database into memory
    fn load_groups(&self) -> Result<()> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare("SELECT group_id, state_blob FROM groups")?;
        
        let group_entries = stmt.query_map([], |row| {
            let group_id: Vec<u8> = row.get(0)?;
            let state_blob: Vec<u8> = row.get(1)?;
            let mut id_arr = [0u8; 32];
            id_arr.copy_from_slice(&group_id);
            Ok((id_arr, state_blob))
        })?;
        
        let mut groups = self.groups.lock().unwrap();
        for entry in group_entries {
            let (group_id, state_blob) = entry?;
            // Deserialize GroupState from blob
            let group_state: GroupState = bincode::deserialize(&state_blob)
                .context("Failed to deserialize group state")?;
            groups.insert(group_id, group_state);
        }
        
        Ok(())
    }
    
    /// Create a new group
    pub fn create_group(&self, name: &str) -> Result<String> {
        let mut groups = self.groups.lock().unwrap();
        let group_state = GroupState::create(name, self.my_member_id)?;
        let group_id = *group_state.group_id();
        
        // Serialize and save to database
        let state_blob = bincode::serialize(&group_state)?;
        
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT INTO groups (group_id, name, admin_id, created_at, state_blob) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &group_id[..],
                name,
                &self.my_member_id[..],
                Utc::now().timestamp(),
                state_blob,
            ],
        )?;
        
        groups.insert(group_id, group_state);
        
        Ok(hex::encode(&group_id))
    }
    
    /// Set the message router callback for mesh integration
    pub fn set_router<F>(&mut self, router_fn: F)
    where
        F: Fn(Vec<u8>) -> Result<()> + Send + Sync + 'static,
    {
        self.router = Some(Arc::new(Mutex::new(router_fn)));
    }
    
    /// Add a member to a group
    pub fn add_member(
        &self,
        group_id_hex: &str,
        member_id_hex: &str,
        sender_key_state: SenderKeyState,
    ) -> Result<()> {
        let group_id = hex::decode_group_id(group_id_hex)?;
        let member_id = hex::decode_member_id(member_id_hex)?;
        
        let mut groups = self.groups.lock().unwrap();
        let group = groups.get_mut(&group_id)
            .context("Group not found")?;
        
        group.add_member(member_id, MemberRole::Member, sender_key_state)?;
        
        // Update database
        let state_blob = bincode::serialize(&*group)?;
        let db = self.db.lock().unwrap();
        
        db.execute(
            "UPDATE groups SET state_blob = ?1 WHERE group_id = ?2",
            params![state_blob, &group_id[..]],
        )?;
        
        db.execute(
            "INSERT INTO group_members (group_id, member_id, role, joined_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                &group_id[..],
                &member_id[..],
                0, // MemberRole::Member
                Utc::now().timestamp(),
            ],
        )?;
        
        Ok(())
    }
    
    /// Get all groups as DTOs for frontend
    pub fn get_groups(&self) -> Result<Vec<GroupDto>> {
        let groups = self.groups.lock().unwrap();
        let result: Vec<GroupDto> = groups
            .values()
            .map(|group| GroupDto {
                group_id: hex::encode(group.group_id()),
                name: group.info.name.clone(),
                created_at: group.info.created_at.to_rfc3339(),
                member_count: group.member_count(),
                is_admin: group.is_admin(),
            })
            .collect();
        Ok(result)
    }
    
    /// Send a message to a group
    pub fn send_message(&self, group_id_hex: &str, content: &str) -> Result<()> {
        let group_id = hex::decode_group_id(group_id_hex)?;
        
        let mut groups = self.groups.lock().unwrap();
        let group = groups.get_mut(&group_id)
            .context("Group not found")?;
        
        let encrypted_msg = group.encrypt(content.as_bytes())?;
        
        // Save to database
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT INTO group_messages (group_id, sender_id, content, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![
                &group_id[..],
                &self.my_member_id[..],
                bincode::serialize(&encrypted_msg)?,
                Utc::now().timestamp(),
            ],
        )?;
        
        // Update group state in database
        let state_blob = bincode::serialize(&*group)?;
        db.execute(
            "UPDATE groups SET state_blob = ?1 WHERE group_id = ?2",
            params![state_blob, &group_id[..]],
        )?;
        
        // Send via mesh network (if router is available)
        if let Some(ref router_fn) = self.router {
            use crate::mesh_protocol::MeshMessage;
            
            let mesh_msg = MeshMessage::GroupMessage {
                group_id,
                sender_id: self.my_member_id,
                encrypted_content: bincode::serialize(&encrypted_msg)?,
                timestamp: Utc::now().timestamp(),
            };
            
            let message_bytes = mesh_msg.to_bytes()
                .map_err(|e| anyhow::anyhow!(e))?;
            
            router_fn.lock().unwrap()(message_bytes)?;
        }
        
        Ok(())
    }
    
    /// Receive and store a group message from mesh network
    pub fn receive_message(
        &self,
        group_id: GroupId,
        sender_id: MemberId,
        encrypted_content: &[u8],
        timestamp: i64,
    ) -> Result<()> {
        // Verify group exists
        {
            let groups = self.groups.lock().unwrap();
            if !groups.contains_key(&group_id) {
                return Err(anyhow::anyhow!("Group not found: {:?}", hex::encode(&group_id)));
            }
        }
        
        // Store message in database
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT INTO group_messages (group_id, sender_id, content, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![
                &group_id[..],
                &sender_id[..],
                encrypted_content,
                timestamp,
            ],
        )?;
        
        log::info!(
            "Stored group message from {} for group {}",
            hex::encode(&sender_id),
            hex::encode(&group_id)
        );
        
        Ok(())
    }
    
    /// Get messages for a group
    pub fn get_messages(&self, group_id_hex: &str) -> Result<Vec<GroupMessageDto>> {
        let group_id = hex::decode_group_id(group_id_hex)?;
        
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT sender_id, content, timestamp FROM group_messages 
             WHERE group_id = ?1 ORDER BY timestamp ASC"
        )?;
        
        let messages = stmt.query_map(params![&group_id[..]], |row| {
            let sender_id: Vec<u8> = row.get(0)?;
            let content: Vec<u8> = row.get(1)?;
            let timestamp: i64 = row.get(2)?;
            Ok((sender_id, content, timestamp))
        })?;
        
        let mut result = Vec::new();
        let mut groups = self.groups.lock().unwrap();
        let group = groups.get_mut(&group_id)
            .context("Group not found")?;
        
        for msg_res in messages {
            let (sender_id, content_blob, timestamp) = msg_res?;
            let encrypted_msg: messaging::group::sender_key::SenderKeyMessage = 
                bincode::deserialize(&content_blob)?;
            
            // Decrypt message
            let plaintext = group.decrypt(&encrypted_msg)
                .unwrap_or_else(|_| b"[Decryption failed]".to_vec());
            
            result.push(GroupMessageDto {
                group_id: group_id_hex.to_string(),
                sender_id: hex::encode(&sender_id),
                content: String::from_utf8_lossy(&plaintext).to_string(),
                timestamp,
            });
        }
        
        Ok(result)
    }
}

/// Helper functions for hex encoding/decoding
mod hex {
    use anyhow::{anyhow, Result};
    use messaging::types::{GroupId, MemberId};
    
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
    
    pub fn decode_group_id(hex_str: &str) -> Result<GroupId> {
        let bytes: Vec<u8> = hex_str
            .as_bytes()
            .chunks(2)
            .map(|chunk| {
                let hex = std::str::from_utf8(chunk)?;
                u8::from_str_radix(hex, 16).map_err(|e| anyhow!("Invalid hex: {}", e))
            })
            .collect::<Result<Vec<u8>>>()?;
        
        if bytes.len() != 32 {
            return Err(anyhow!("Invalid group ID length"));
        }
        
        let mut id = [0u8; 32];
        id.copy_from_slice(&bytes);
        Ok(id)
    }
    
    pub fn decode_member_id(hex_str: &str) -> Result<MemberId> {
        decode_group_id(hex_str) // Same format
    }
}
