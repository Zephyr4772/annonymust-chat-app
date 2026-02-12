// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod group_manager;
mod file_transfer;
mod circuit_manager;
mod mesh_protocol;
mod message_router;
mod simple_mesh;
mod mesh_init;
mod mesh_commands;

use group_manager::{GroupManager, GroupDto, GroupMessageDto};
use file_transfer::{FileTransferManager, FileTransferDto};
use circuit_manager::{CircuitManager, CircuitDto};
use message_router::MessageRouter;
use simple_mesh::SimpleMeshNetwork;
use mesh_commands::{init_mesh, add_peer, get_mesh_status};
use messaging::types::PeerId;
use std::sync::Arc;

// Global state for managers
struct AppState {
    group_manager: Arc<tokio::sync::Mutex<Option<GroupManager>>>,
    file_transfer_manager: Arc<tokio::sync::Mutex<Option<FileTransferManager>>>,
    circuit_manager: Arc<tokio::sync::Mutex<Option<CircuitManager>>>,
    message_router: Arc<tokio::sync::Mutex<Option<MessageRouter>>>,
    mesh_network: Arc<tokio::sync::Mutex<Option<SimpleMeshNetwork>>>,
}

#[tauri::command]
async fn greet(name: &str) -> Result<String, String> {
    Ok(format!("Hello, {}! You've been greeted from Rust!", name))
}

// Group management commands

#[tauri::command]
async fn init_groups(my_member_id_hex: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let member_id = decode_hex_to_32(&my_member_id_hex)
        .map_err(|e| format!("Invalid member ID: {}", e))?;
    
    let manager = GroupManager::new("groups.db", member_id)
        .map_err(|e| format!("Failed to initialize group manager: {}", e))?;
    
    *state.group_manager.lock().await = Some(manager);
    Ok(())
}

#[tauri::command]
async fn create_group(name: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let manager_lock = state.group_manager.lock().await;
    let manager = manager_lock.as_ref()
        .ok_or_else(|| "Group manager not initialized".to_string())?;
    
    manager.create_group(&name)
        .map_err(|e| format!("Failed to create group: {}", e))
}

#[tauri::command]
async fn get_groups(state: tauri::State<'_, AppState>) -> Result<Vec<GroupDto>, String> {
    let manager_lock = state.group_manager.lock().await;
    let manager = manager_lock.as_ref()
        .ok_or_else(|| "Group manager not initialized".to_string())?;
    
    manager.get_groups()
        .map_err(|e| format!("Failed to get groups: {}", e))
}

#[tauri::command]
async fn send_group_message(
    group_id: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let manager_lock = state.group_manager.lock().await;
    let manager = manager_lock.as_ref()
        .ok_or_else(|| "Group manager not initialized".to_string())?;
    
    manager.send_message(&group_id, &content)
        .map_err(|e| format!("Failed to send message: {}", e))
}

#[tauri::command]
async fn get_group_messages(
    group_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<GroupMessageDto>, String> {
    let manager_lock = state.group_manager.lock().await;
    let manager = manager_lock.as_ref()
        .ok_or_else(|| "Group manager not initialized".to_string())?;
    
    manager.get_messages(&group_id)
        .map_err(|e| format!("Failed to get messages: {}", e))
}

// File transfer commands

#[tauri::command]
async fn init_file_transfer(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let manager = FileTransferManager::new("file_transfers.db")
        .map_err(|e| format!("Failed to initialize file transfer manager: {}", e))?;
    
    *state.file_transfer_manager.lock().await = Some(manager);
    Ok(())
}

#[tauri::command]
async fn send_file(
    file_path: String,
    peer_id_hex: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let peer_id = decode_hex_to_peer_id(&peer_id_hex)?;
    
    // Use tokio::sync::Mutex so we can hold the lock across await
    let manager_arc = state.file_transfer_manager.clone();
    let manager_lock = manager_arc.lock().await;
    
    if let Some(manager) = manager_lock.as_ref() {
        let transfer_id = manager.send_file(std::path::Path::new(&file_path), peer_id)
            .await
            .map_err(|e| format!("Failed to send file: {}", e))?;
        
        Ok(transfer_id.to_string())
    } else {
        Err("File transfer manager not initialized".to_string())
    }
}

#[tauri::command]
async fn get_file_transfers(state: tauri::State<'_, AppState>) -> Result<Vec<FileTransferDto>, String> {
    let manager_lock = state.file_transfer_manager.lock().await;
    let manager = manager_lock.as_ref()
        .ok_or_else(|| "File transfer manager not initialized".to_string())?;
    
    manager.get_transfers()
        .map_err(|e| format!("Failed to get transfers: {}", e))
}

#[tauri::command]
async fn get_file_transfer_progress(
    transfer_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<f64, String> {
    let transfer_id = uuid::Uuid::parse_str(&transfer_id)
        .map_err(|e| format!("Invalid transfer ID: {}", e))?;
    
    let manager_lock = state.file_transfer_manager.lock().await;
    let manager = manager_lock.as_ref()
        .ok_or_else(|| "File transfer manager not initialized".to_string())?;
    
    manager.get_progress(transfer_id)
        .map_err(|e| format!("Failed to get progress: {}", e))
}

#[tauri::command]
async fn cancel_file_transfer(
    transfer_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let transfer_id = uuid::Uuid::parse_str(&transfer_id)
        .map_err(|e| format!("Invalid transfer ID: {}", e))?;
    
    let manager_lock = state.file_transfer_manager.lock().await;
    let manager = manager_lock.as_ref()
        .ok_or_else(|| "File transfer manager not initialized".to_string())?;
    
    manager.cancel_transfer(transfer_id)
        .map_err(|e| format!("Failed to cancel transfer: {}", e))
}

// Onion routing circuit commands

#[tauri::command]
async fn init_circuit_manager(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let manager = CircuitManager::new("circuits.db")
        .map_err(|e| format!("Failed to initialize circuit manager: {}", e))?;
    
    // Add bootstrap relays
    manager.add_bootstrap_relays()
        .map_err(|e| format!("Failed to add bootstrap relays: {}", e))?;
    
    *state.circuit_manager.lock().await = Some(manager);
    Ok(())
}

#[tauri::command]
async fn build_circuit(state: tauri::State<'_, AppState>) -> Result<u64, String> {
    let manager_lock = state.circuit_manager.lock().await;
    let manager = manager_lock.as_ref()
        .ok_or_else(|| "Circuit manager not initialized".to_string())?;
    
    manager.build_circuit()
        .map_err(|e| format!("Failed to build circuit: {}", e))
}

#[tauri::command]
async fn get_circuits(state: tauri::State<'_, AppState>) -> Result<Vec<CircuitDto>, String> {
    let manager_lock = state.circuit_manager.lock().await;
    let manager = manager_lock.as_ref()
        .ok_or_else(|| "Circuit manager not initialized".to_string())?;
    
    manager.get_circuits()
        .map_err(|e| format!("Failed to get circuits: {}", e))
}

#[tauri::command]
async fn close_circuit(circuit_id: u64, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let manager_lock = state.circuit_manager.lock().await;
    let manager = manager_lock.as_ref()
        .ok_or_else(|| "Circuit manager not initialized".to_string())?;
    
    manager.close_circuit(circuit_id)
        .map_err(|e| format!("Failed to close circuit: {}", e))
}

// Mesh integration commands

#[tauri::command]
async fn init_message_router(state: tauri::State<'_, AppState>) -> Result<(), String> {
    // Just clone the Arc wrappers directly from the state
    let group_mgr = state.group_manager.clone();
    let file_mgr = state.file_transfer_manager.clone();
    let circuit_mgr = state.circuit_manager.clone();
    
    let router = MessageRouter::new(group_mgr, file_mgr, circuit_mgr);
    
    *state.message_router.lock().await = Some(router);
    Ok(())
}

#[tauri::command]
async fn test_mesh_routing(message_type: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    use crate::mesh_protocol::MeshMessage;
    
    let router_lock = state.message_router.lock().await;
    let router = router_lock.as_ref()
        .ok_or_else(|| "Message router not initialized".to_string())?;
    
    // Create test message based on type
    let test_msg = match message_type.as_str() {
        "group" => MeshMessage::GroupMessage {
            group_id: [1u8; 32],
            sender_id: [2u8; 32],
            encrypted_content: b"test message".to_vec(),
            timestamp: chrono::Utc::now().timestamp(),
        },
        "file_chunk" => MeshMessage::FileChunk {
            transfer_id: uuid::Uuid::new_v4(),
            chunk_index: 0,
            data: b"test data".to_vec(),
            checksum: [0u8; 32],
        },
        _ => return Err(format!("Unknown message type: {}", message_type)),
    };
    
    // Serialize and route
    let bytes = router.send_outgoing(test_msg)
        .map_err(|e| format!("Failed to send: {}", e))?;
    
    Ok(format!("Sent {} bytes", bytes.len()))
}

// Helper function to decode hex string to [u8; 32]
fn decode_hex_to_32(hex_str: &str) -> Result<[u8; 32], String> {
    let bytes: Vec<u8> = hex_str
        .as_bytes()
        .chunks(2)
        .map(|chunk| {
            let hex = std::str::from_utf8(chunk)
                .map_err(|_| "Invalid UTF-8".to_string())?;
            u8::from_str_radix(hex, 16)
                .map_err(|_| format!("Invalid hex: {}", hex))
        })
        .collect::<Result<Vec<u8>, String>>()?;
    
    if bytes.len() != 32 {
        return Err(format!("Invalid length: expected 32, got {}", bytes.len()));
    }
    
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn decode_hex_to_peer_id(hex_str: &str) -> Result<PeerId, String> {
    let bytes = decode_hex_to_32(hex_str)?;
    Ok(PeerId(bytes))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    env_logger::init();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            group_manager: Arc::new(tokio::sync::Mutex::new(None)),
            file_transfer_manager: Arc::new(tokio::sync::Mutex::new(None)),
            circuit_manager: Arc::new(tokio::sync::Mutex::new(None)),
            message_router: Arc::new(tokio::sync::Mutex::new(None)),
            mesh_network: Arc::new(tokio::sync::Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            init_groups,
            create_group,
            get_groups,
            send_group_message,
            get_group_messages,
            init_file_transfer,
            send_file,
            get_file_transfers,
            get_file_transfer_progress,
            cancel_file_transfer,
            init_circuit_manager,
            build_circuit,
            get_circuits,
            close_circuit,
            init_message_router,
            test_mesh_routing,
            init_mesh,
            add_peer,
            get_mesh_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
