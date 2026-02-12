//! Mesh network initialization and setup

use crate::message_router::MessageRouter;
use crate::simple_mesh::SimpleMeshNetwork;
use crate::group_manager::GroupManager;
use crate::file_transfer::FileTransferManager;
use crate::circuit_manager::CircuitManager;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;
use std::net::SocketAddr;

/// Initialize mesh network and connect to message router
pub async fn init_mesh_network(
    port: u16,
    router: Arc<TokioMutex<Option<MessageRouter>>>,
) -> Result<SimpleMeshNetwork> {
    // Listen address
    let listen_addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    
    // Create mesh network
    let mesh = SimpleMeshNetwork::new(listen_addr, router.clone());
    
    // Set mesh network in router
    {
        let mesh_arc = Arc::new(TokioMutex::new(Some(mesh.clone())));
        let mut router_lock = router.lock().await;
        if let Some(router_ref) = router_lock.as_mut() {
            router_ref.set_mesh_network(mesh_arc);
        }
    }
    
    // Start listening
    mesh.start().await?;
    
    log::info!("Mesh network initialized on {}", listen_addr);
    
    Ok(mesh)
}
