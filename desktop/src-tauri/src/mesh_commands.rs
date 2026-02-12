use crate::AppState;
use crate::mesh_init;

#[tauri::command]
pub async fn init_mesh(
    port: u16,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    log::info!("Initializing mesh network on port {}", port);
    
    // Initialize mesh network
    let mesh = mesh_init::init_mesh_network(port, state.message_router.clone())
        .await
        .map_err(|e| format!("Failed to initialize mesh: {}", e))?;
    
    *state.mesh_network.lock().await = Some(mesh);
    
    Ok(format!("Mesh network initialized on port {}", port))
}

#[tauri::command]
pub async fn add_peer(
    peer_address: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let peer_addr: std::net::SocketAddr = peer_address.parse()
        .map_err(|e| format!("Invalid peer address: {}", e))?;
    
    let mesh_lock = state.mesh_network.lock().await;
    let mesh = mesh_lock.as_ref()
        .ok_or("Mesh network not initialized")?;
    
    mesh.add_peer(peer_addr).await;
    
    Ok(())
}

#[tauri::command]
pub async fn get_mesh_status(
    state: tauri::State<'_, AppState>,
) -> Result<MeshStatus, String> {
    let mesh_lock = state.mesh_network.lock().await;
    let mesh = mesh_lock.as_ref()
        .ok_or("Mesh network not initialized")?;
    
    Ok(MeshStatus {
        peer_count: mesh.peer_count().await,
        is_active: true,
    })
}

#[derive(serde::Serialize)]
pub struct MeshStatus {
    peer_count: usize,
    is_active: bool,
}
