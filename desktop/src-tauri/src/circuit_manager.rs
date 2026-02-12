//! Circuit manager for onion routing
//!
//! Manages 4-hop onion circuits using the existing `onion_routing` crate.

use anyhow::{Context, Result};
use messaging::types::PeerId;
use onion_routing::circuit::Circuit;
use onion_routing::relay::{RelayNode, RelayRole};
use onion_routing::selection::GuardSelector;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Circuit DTO for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitDto {
    pub circuit_id: u64,
    pub hops: Vec<String>, // IP addresses
    pub is_ready: bool,
    pub created_at: i64,
    pub last_used: i64,
}

/// Manages onion routing circuits
pub struct CircuitManager {
    db: Arc<Mutex<Connection>>,
    circuits: Arc<Mutex<HashMap<u64, Circuit>>>,
    relay_pool: Arc<Mutex<Vec<RelayNode>>>,
    next_circuit_id: AtomicU64,
}

impl CircuitManager {
    /// Initialize circuit manager
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db = Connection::open(db_path)?;
        
        // Create tables
        db.execute(
            "CREATE TABLE IF NOT EXISTS circuits (
                circuit_id INTEGER PRIMARY KEY,
                hops BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                last_used INTEGER NOT NULL,
                is_active INTEGER NOT NULL
            )",
            [],
        )?;
        
        db.execute(
            "CREATE TABLE IF NOT EXISTS relay_pool (
                address TEXT PRIMARY KEY,
                public_key BLOB NOT NULL,
                role TEXT NOT NULL,
                uptime_hours REAL,
                bandwidth_mbps INTEGER,
                asn INTEGER,
                country_code TEXT,
                last_seen INTEGER NOT NULL
            )",
            [],
        )?;
        
        let manager = Self {
            db: Arc::new(Mutex::new(db)),
            circuits: Arc::new(Mutex::new(HashMap::new())),
            relay_pool: Arc::new(Mutex::new(Vec::new())),
            next_circuit_id: AtomicU64::new(1),
        };
        
        // Load relay pool
        manager.load_relay_pool()?;
        
        // Load active circuits
        manager.load_circuits()?;
        
        Ok(manager)
    }
    
    /// Load relay pool from database
    fn load_relay_pool(&self) -> Result<()> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT address, public_key, role, uptime_hours, bandwidth_mbps, asn, country_code 
             FROM relay_pool WHERE last_seen > ?"
        )?;
        
        let one_day_ago = chrono::Utc::now().timestamp() - 86400;
        
        let relays = stmt.query_map(params![one_day_ago], |row| {
            let address_str: String = row.get(0)?;
            let public_key_bytes: Vec<u8> = row.get(1)?;
            let role_str: String = row.get(2)?;
            let uptime_hours: f64 = row.get(3)?;
            let bandwidth_mbps: i64 = row.get(4)?;
            let asn: i64 = row.get(5)?;
            let country_code: String = row.get(6)?;
            
            Ok((
                address_str,
                public_key_bytes,
                role_str,
                uptime_hours,
                bandwidth_mbps,
                asn,
                country_code,
            ))
        })?;
        
        let mut pool = self.relay_pool.lock().unwrap();
        for relay_res in relays {
            let (address_str, pk_bytes, role_str, uptime_h, bw, asn_val, cc) = relay_res?;
            
            // Parse address
            let address: SocketAddr = address_str.parse()
                .context("Invalid relay address")?;
            
            // Reconstruct public identity
            let public_key: core_crypto::identity::PublicIdentity = bincode::deserialize(&pk_bytes)
                .context("Invalid public key")?;
            
            // Parse role
            let role = match role_str.as_str() {
                "Guard" => RelayRole::Guard,
                "Middle" => RelayRole::Middle,
                "Exit" => RelayRole::Exit,
                _ => RelayRole::Middle,
            };
            
            let mut node = RelayNode::new(address, public_key, role);
            node.uptime_percentage = (uptime_h / 24.0 * 100.0).min(100.0) as f32;
            node.bandwidth_mbps = bw as u32;
            node.asn = asn_val as u32;
            node.country_code = cc;
            
            pool.push(node);
        }
        
        Ok(())
    }
    
    /// Load active circuits from database
    fn load_circuits(&self) -> Result<()> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT circuit_id, hops FROM circuits WHERE is_active = 1"
        )?;
        
        let circuits = stmt.query_map([], |row| {
            let circuit_id: i64 = row.get(0)?;
            let hops_blob: Vec<u8> = row.get(1)?;
            Ok((circuit_id as u64, hops_blob))
        })?;
        
        let mut circuits_map = self.circuits.lock().unwrap();
        for circuit_res in circuits {
            let (cid, hops_blob) = circuit_res?;
            
            // Deserialize hops
            let hops: Vec<RelayNode> = bincode::deserialize(&hops_blob)
                .context("Failed to deserialize circuit hops")?;
            
            let circuit = Circuit::new(cid, hops)
                .map_err(|e| anyhow::anyhow!(e))?;
            circuits_map.insert(cid, circuit);
            
            // Update next_circuit_id
            let current = self.next_circuit_id.load(Ordering::SeqCst);
            if cid >= current {
                self.next_circuit_id.store(cid + 1, Ordering::SeqCst);
            }
        }
        
        Ok(())
    }
    
    /// Build a new 4-hop circuit
    pub fn build_circuit(&self) -> Result<u64> {
        let pool = self.relay_pool.lock().unwrap();
        
        if pool.len() < 4 {
            return Err(anyhow::anyhow!("Not enough relays in pool (need 4, have {})", pool.len()));
        }
        
        // Select guards
        let guards: Vec<RelayNode> = pool.iter()
            .filter(|n| n.role == RelayRole::Guard)
            .cloned()
            .collect();
        
        let primary = GuardSelector::select_primary_guard(&guards)
            .ok_or_else(|| anyhow::anyhow!("No suitable primary guard"))?;
        
        let secondary = GuardSelector::select_secondary_guard(&guards, &primary)
            .ok_or_else(|| anyhow::anyhow!("No suitable secondary guard"))?;
        
        // Select middle and exit
        let middles: Vec<RelayNode> = pool.iter()
            .filter(|n| n.role == RelayRole::Middle)
            .cloned()
            .collect();
        
        let exits: Vec<RelayNode> = pool.iter()
            .filter(|n| n.role == RelayRole::Exit)
            .cloned()
            .collect();
        
        let hops = GuardSelector::build_circuit(primary, secondary, &middles, &exits)
            .ok_or_else(|| anyhow::anyhow!("Failed to build circuit with diversity requirements"))?;
        
        drop(pool);
        
        // Create circuit
        let circuit_id = self.next_circuit_id.fetch_add(1, Ordering::SeqCst);
        let mut circuit = Circuit::new(circuit_id, hops.clone())
            .map_err(|e| anyhow::anyhow!(e))?;
        
        // Perform simplified handshakes (MVP: derive from relay keys)
        // Production TODO: Real X25519 ECDH network handshakes  
        let shared_secrets = hops.iter()
            .map(|relay| {
                // Derive shared secret from relay's identity key
                // In production, perform CREATE/CREATED cell exchange
                let key_bytes = bincode::serialize(&relay.identity.signing_public_key)
                    .unwrap_or_default();
                let hash = blake3::hash(&key_bytes);
                *hash.as_bytes()
            })
            .collect();
        
        circuit.establish(shared_secrets)
            .map_err(|e| anyhow::anyhow!(e))?;
        
        // Save to database
        self.save_circuit(&circuit)?;
        
        // Add to active circuits
        self.circuits.lock().unwrap().insert(circuit_id, circuit);
        
        Ok(circuit_id)
    }
    
    /// Send message through circuit
    pub fn send_through_circuit(&self, circuit_id: u64, payload: &[u8]) -> Result<Vec<u8>> {
        let circuits = self.circuits.lock().unwrap();
        let circuit = circuits.get(&circuit_id)
            .ok_or_else(|| anyhow::anyhow!("Circuit not found"))?;
        
        let encrypted = circuit.send(payload)
            .map_err(|e| anyhow::anyhow!(e))?;
        
        // Update last_used
        drop(circuits);
        self.update_last_used(circuit_id)?;
        
        Ok(encrypted)
    }
    
    /// Decrypt received circuit packet (unwrap onion layers)
    pub fn decrypt_from_circuit(&self, circuit_id: u64, onion_packet: &[u8]) -> Result<Vec<u8>> {
        let circuits = self.circuits.lock().unwrap();
        let circuit = circuits.get(&circuit_id)
            .ok_or_else(|| anyhow::anyhow!("Circuit not found"))?;
        
        // Circuit doesn't have receive() method - this would need to be implemented
        // For MVP, we'll just return the packet as-is (would need proper decryption)
        // TODO: Implement proper onion decryption when Circuit API supports it
        Ok(onion_packet.to_vec())
    }
    
    /// Get circuit for routing decisions
    pub fn get_circuit(&self, circuit_id: u64) -> Result<Circuit> {
        let circuits = self.circuits.lock().unwrap();
        circuits.get(&circuit_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Circuit {} not found", circuit_id))
    }
    
    /// Get all active circuits
    pub fn get_circuits(&self) -> Result<Vec<CircuitDto>> {
        let circuits = self.circuits.lock().unwrap();
        let result: Vec<CircuitDto> = circuits.values()
            .map(|c| CircuitDto {
                circuit_id: c.id,
                hops: c.hops.iter().map(|h| h.address.to_string()).collect(),
                is_ready: c.is_ready,
                created_at: c.created_at,
                last_used: c.created_at, // TODO: Track actual last_used
            })
            .collect();
        
        Ok(result)
    }
    
    /// Close a circuit
    pub fn close_circuit(&self, circuit_id: u64) -> Result<()> {
        self.circuits.lock().unwrap().remove(&circuit_id);
        
        let db = self.db.lock().unwrap();
        db.execute(
            "UPDATE circuits SET is_active = 0 WHERE circuit_id = ?1",
            params![circuit_id as i64],
        )?;
        
        Ok(())
    }
    
    /// Add bootstrap relays to pool
    pub fn add_bootstrap_relays(&self) -> Result<()> {
        // Bootstrap relays (hardcoded for now)
        // TODO: Load from config or directory
        
        let bootstrap_addresses = vec![
            "127.0.0.1:9001",
            "127.0.0.1:9002",
            "127.0.0.1:9003",
            "127.0.0.1:9004",
        ];
        
        let mut pool = self.relay_pool.lock().unwrap();
        
        for (idx, addr_str) in bootstrap_addresses.iter().enumerate() {
            let address: SocketAddr = addr_str.parse()?;
            
            // Generate dummy identities for bootstrap
            let identity = core_crypto::identity::Identity::generate();
            let public_key = identity.to_public();
            
            let role = match idx {
                0 | 1 => RelayRole::Guard,
                2 => RelayRole::Middle,
                3 => RelayRole::Exit,
                _ => RelayRole::Middle,
            };
            
            let mut node = RelayNode::new(address, public_key, role);
            node.uptime_percentage = 99.5;
            node.bandwidth_mbps = 100;
            node.asn = 1000 + idx as u32;
            node.country_code = format!("C{}", idx);
            
            pool.push(node.clone());
            
            // Save to database
            self.save_relay(&node)?;
        }
        
        Ok(())
    }
    
    // Helper methods
    
    fn save_circuit(&self, circuit: &Circuit) -> Result<()> {
        let hops_blob = bincode::serialize(&circuit.hops)?;
        
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT OR REPLACE INTO circuits (circuit_id, hops, created_at, last_used, is_active) 
             VALUES (?1, ?2, ?3, ?4, 1)",
            params![
                circuit.id as i64,
                hops_blob,
                circuit.created_at,
                circuit.created_at,
            ],
        )?;
        
        Ok(())
    }
    
    fn save_relay(&self, relay: &RelayNode) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT OR REPLACE INTO relay_pool 
             (address, public_key, role, uptime_hours, bandwidth_mbps, asn, country_code, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                relay.address.to_string(),
                bincode::serialize(&relay.identity.signing_public_key).unwrap_or_default(),
                format!("{:?}", relay.role),
                (relay.uptime_percentage / 100.0 * 24.0),
                relay.bandwidth_mbps as i64,
                relay.asn as i64,
                relay.country_code,
                chrono::Utc::now().timestamp(),
            ],
        )?;
        
        Ok(())
    }
    
    fn update_last_used(&self, circuit_id: u64) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.execute(
            "UPDATE circuits SET last_used = ?1 WHERE circuit_id = ?2",
            params![chrono::Utc::now().timestamp(), circuit_id as i64],
        )?;
        
        Ok(())
    }
}
