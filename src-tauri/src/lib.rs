mod p2p_node;

use p2p_node::{ChatMessage, P2PNode, PeerInfo, sanitize_message as validate_message_content};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{mpsc, Mutex};
use futures::StreamExt;

type P2PState = Arc<Mutex<Option<P2PNodeHandle>>>;

struct P2PNodeHandle {
    #[allow(dead_code)]
    peer_id: String,
    command_tx: mpsc::UnboundedSender<P2PCommand>,
}

enum P2PCommand {
    JoinRoom(String),
    SendMessage { peer_id: String, content: String },
    ConnectToPeer(String),
    GetInfo(tokio::sync::oneshot::Sender<Result<NodeInfo, String>>),
    RequestConnection(String),
    AcceptConnection(String),
    RejectConnection(String),
    SetUsername(String),
}

#[derive(serde::Serialize, Clone)]
struct NodeInfo {
    peer_id: String,
    addresses: Vec<String>,
    connected_peers: Vec<PeerInfo>,
}

// ============================================================================
// TAURI COMMANDS - All with proper error handling
// ============================================================================

#[tauri::command]
async fn init_p2p(app: AppHandle, state: State<'_, P2PState>) -> Result<String, String> {
    let mut state_guard = state.lock().await;
    
    if state_guard.is_some() {
        return Err("P2P node already initialized".to_string());
    }

    let (message_tx, mut message_rx) = mpsc::unbounded_channel::<ChatMessage>();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<serde_json::Value>();
    let (command_tx, mut command_rx) = mpsc::unbounded_channel::<P2PCommand>();
    
    // Create P2P node with both message and event channels
    let (mut node, mut swarm) = P2PNode::create(message_tx.clone(), event_tx)
        .await
        .map_err(|e| format!("Failed to create P2P node: {}", e))?;

    let peer_id = node.get_peer_id();
    
    // Store node handle
    *state_guard = Some(P2PNodeHandle {
        peer_id: peer_id.clone(),
        command_tx,
    });
    drop(state_guard);

    // Send initial message
    let _ = node.message_tx.send(ChatMessage {
        from: "System".to_string(),
        content: "[>] Node initialized - connecting to network...".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        is_self: false,
        peer_id: None,
    });

    node.send_system_message("[O] Local network discovery (mDNS) enabled".to_string());
    node.bootstrap_dht(&mut swarm);
    
    // Listen on IPv4 — needed for local hotspot/WiFi (mDNS advertises IPv4 addresses)
    // Port 0 = OS picks a free port, avoids conflicts
    if let Err(e) = swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().map_err(|e| {
        format!("Failed to parse listen address: {}", e)
    })?) {
        eprintln!("Warning: Failed to listen on IPv4: {}", e);
    }

    // Listen on IPv6 — needed for carrier/NAT64 mobile networks
    if let Err(e) = swarm.listen_on("/ip6/::/tcp/0".parse().map_err(|e| {
        format!("Failed to parse listen address: {}", e)
    })?) {
        eprintln!("Warning: Failed to listen on IPv6: {}", e);
    }

    let app_message_relay = app.clone();
    let app_event_relay = app.clone();
    
    // Spawn message relay task
    tokio::spawn(async move {
        while let Some(msg) = message_rx.recv().await {
            let _ = app_message_relay.emit("chat-message", msg);
        }
    });

    // Spawn event relay task for peer discovery and connection events
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let event_type = event.get("type").and_then(|t| t.as_str()).unwrap_or("peer-discovered");
            if let Some(peer_id) = event.get("peer_id").and_then(|p| p.as_str()) {
                let _ = app_event_relay.emit(event_type, serde_json::json!({
                    "peerId": peer_id,
                    "address": event.get("address"),
                    "timestamp": event.get("timestamp")
                }));
            }
        }
    });

    // Spawn command handler and node runner
    tokio::spawn(async move {
        let mut peer_discovery_interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            tokio::select! {
                Some(cmd) = command_rx.recv() => {
                    match cmd {
                        P2PCommand::JoinRoom(room_name) => {
                            node.join_room(&mut swarm, room_name);
                        }
                        P2PCommand::SendMessage { peer_id, content } => {
                            tracing::debug!("Processing SendMessage command for peer: {}", peer_id);
                            if let Err(e) = node.send_message(&mut swarm, peer_id, content).await {
                                tracing::error!("SendMessage error: {}", e);
                                node.send_system_message(format!("[!] Failed to send message: {}", e));
                            }
                        }
                        P2PCommand::ConnectToPeer(addr) => {
                            if let Err(e) = node.connect_to_peer(&mut swarm, addr) {
                                node.send_system_message(format!("[!] Connection error: {}", e));
                            }
                        }
                        P2PCommand::GetInfo(tx) => {
                            let info = NodeInfo {
                                peer_id: node.get_peer_id(),
                                addresses: node.get_addresses(&swarm),
                                connected_peers: node.get_connected_peers(),
                            };
                            let _ = tx.send(Ok(info));
                        }
                        P2PCommand::RequestConnection(peer_id) => {
                            tracing::info!("RequestConnection command received for peer: {}", peer_id);
                            match node.request_connection(&mut swarm, &peer_id) {
                                Ok(_) => {
                                    tracing::info!("Dial initiated for peer: {}", peer_id);
                                }
                                Err(e) => {
                                    tracing::error!("Failed to request connection to {}: {}", peer_id, e);
                                    node.send_system_message(format!("[!] Failed to connect: {}", e));
                                }
                            }
                        }
                        P2PCommand::AcceptConnection(peer_id) => {
                            tracing::info!("AcceptConnection command received for peer: {}", peer_id);
                            match node.accept_connection(&peer_id) {
                                Ok(_) => {
                                    tracing::info!("Accepted connection from peer: {}", peer_id);
                                }
                                Err(e) => {
                                    tracing::error!("Failed to accept connection from {}: {}", peer_id, e);
                                    node.send_system_message(format!("[!] Failed to accept: {}", e));
                                }
                            }
                        }
                        P2PCommand::RejectConnection(peer_id) => {
                            tracing::info!("RejectConnection command received for peer: {}", peer_id);
                            match node.reject_connection(&mut swarm, &peer_id) {
                                Ok(_) => {
                                    tracing::info!("Rejected connection from peer: {}", peer_id);
                                }
                                Err(e) => {
                                    tracing::error!("Failed to reject connection from {}: {}", peer_id, e);
                                    node.send_system_message(format!("[!] Failed to reject: {}", e));
                                }
                            }
                        }
                        P2PCommand::SetUsername(name) => {
                            tracing::info!("SetUsername command received: {}", name);
                            node.set_username(name);
                        }
                    }
                    node.process_pending_dials(&mut swarm);
                    node.process_pending_gossipsub(&mut swarm);
                }
                event = swarm.select_next_some() => {
                    node.handle_event(event).await;
                    node.process_pending_dials(&mut swarm);
                    node.process_pending_gossipsub(&mut swarm);
                }
                _ = peer_discovery_interval.tick() => {
                    if let Some(room_name) = &node.current_room_name {
                        swarm
                            .behaviour_mut()
                            .kad
                            .get_providers(room_name.as_bytes().to_vec().into());
                    }
                }
            }
        }
    });

    Ok(peer_id)
}

#[tauri::command]
async fn get_node_info(state: State<'_, P2PState>) -> Result<NodeInfo, String> {
    tracing::debug!("get_node_info command received");
    let state_guard = state.lock().await;
    
    if let Some(handle) = state_guard.as_ref() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        handle.command_tx.send(P2PCommand::GetInfo(tx))
            .map_err(|e| {
                let err_msg = format!("Failed to send command: {}", e);
                tracing::error!("{}", err_msg);
                err_msg
            })?;
        
        // Timeout after 5 seconds
        match tokio::time::timeout(Duration::from_secs(5), rx).await {
            Ok(Ok(result)) => {
                tracing::debug!("get_node_info completed successfully");
                result
            },
            Ok(Err(e)) => {
                let err_msg = format!("Node command failed: {}", e);
                tracing::error!("{}", err_msg);
                Err(err_msg)
            },
            Err(_) => {
                let err_msg = "Node command timed out".to_string();
                tracing::error!("{}", err_msg);
                Err(err_msg)
            },
        }
    } else {
        let err_msg = "P2P node not initialized".to_string();
        tracing::error!("{}", err_msg);
        Err(err_msg)
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn join_room(room_name: String, state: State<'_, P2PState>) -> Result<(), String> {
    let room_name = validate_message_content(&room_name)
        .map_err(|e| format!("Invalid room name: {}", e))?;
    
    let state_guard = state.lock().await;
    
    if let Some(handle) = state_guard.as_ref() {
        handle.command_tx.send(P2PCommand::JoinRoom(room_name))
            .map_err(|e| format!("Failed to send command: {}", e))?;
        Ok(())
    } else {
        Err("P2P node not initialized".to_string())
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn send_message(
    peer_id: String,
    message: String,
    state: State<'_, P2PState>,
) -> Result<(), String> {
    eprintln!(">>> SEND_MESSAGE CALLED: peer_id={}, msg_len={}", peer_id, message.len());
    tracing::info!("send_message command received from peer: {}, message length: {}", peer_id, message.len());
    
    // Validate peer ID format
    peer_id.parse::<libp2p::PeerId>()
        .map_err(|e| {
            let err_msg = format!("Invalid peer ID format: {}", e);
            tracing::error!("{}", err_msg);
            err_msg
        })?;
    
    // Sanitize message
    let sanitized = validate_message_content(&message)
        .map_err(|e| {
            let err_msg = format!("Invalid message: {}", e);
            tracing::error!("{}", err_msg);
            err_msg
        })?;
    
    let state_guard = state.lock().await;
    
    if let Some(handle) = state_guard.as_ref() {
        tracing::debug!("Sending P2PCommand::SendMessage to backend");
        handle.command_tx.send(P2PCommand::SendMessage {
            peer_id: peer_id.clone(),
            content: sanitized,
        })
        .map_err(|e| {
            let err_msg = format!("Failed to send command: {}", e);
            tracing::error!("{}", err_msg);
            err_msg
        })?;
        tracing::debug!("P2PCommand::SendMessage sent successfully");
        Ok(())
    } else {
        let err_msg = "P2P node not initialized".to_string();
        tracing::error!("{}", err_msg);
        Err(err_msg)
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn connect_to_peer(addr: String, state: State<'_, P2PState>) -> Result<(), String> {
    // Basic validation
    if addr.is_empty() || addr.len() > 512 {
        return Err("Invalid address".to_string());
    }
    
    let state_guard = state.lock().await;
    
    if let Some(handle) = state_guard.as_ref() {
        handle.command_tx.send(P2PCommand::ConnectToPeer(addr))
            .map_err(|e| format!("Failed to send command: {}", e))?;
        Ok(())
    } else {
        Err("P2P node not initialized".to_string())
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn request_connection(peer_id: String, state: State<'_, P2PState>) -> Result<(), String> {
    eprintln!(">>> REQUEST_CONNECTION CALLED: peer_id={}", peer_id);
    tracing::info!("request_connection Tauri command received for: {}", peer_id);
    
    if peer_id.is_empty() || peer_id.len() > 512 {
        return Err("Invalid input".to_string());
    }
    
    let state_guard = state.lock().await;
    
    if let Some(handle) = state_guard.as_ref() {
        handle.command_tx.send(P2PCommand::RequestConnection(peer_id))
            .map_err(|e| format!("Failed to send command: {}", e))?;
        Ok(())
    } else {
        Err("P2P node not initialized".to_string())
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn accept_connection(peer_id: String, state: State<'_, P2PState>) -> Result<(), String> {
    eprintln!(">>> ACCEPT_CONNECTION CALLED: peer_id={}", peer_id);
    tracing::info!("accept_connection Tauri command received for: {}", peer_id);
    
    peer_id.parse::<libp2p::PeerId>()
        .map_err(|_| "Invalid peer ID format".to_string())?;
    
    let state_guard = state.lock().await;
    
    if let Some(handle) = state_guard.as_ref() {
        handle.command_tx.send(P2PCommand::AcceptConnection(peer_id))
            .map_err(|e| format!("Failed to send command: {}", e))?;
        Ok(())
    } else {
        Err("P2P node not initialized".to_string())
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn reject_connection(peer_id: String, state: State<'_, P2PState>) -> Result<(), String> {
    eprintln!(">>> REJECT_CONNECTION CALLED: peer_id={}", peer_id);
    tracing::info!("reject_connection Tauri command received for: {}", peer_id);
    
    // Validate peer ID format
    peer_id.parse::<libp2p::PeerId>()
        .map_err(|_| "Invalid peer ID format".to_string())?;
    
    let state_guard = state.lock().await;
    
    if let Some(handle) = state_guard.as_ref() {
        handle.command_tx.send(P2PCommand::RejectConnection(peer_id))
            .map_err(|e| format!("Failed to send command: {}", e))?;
        Ok(())
    } else {
        Err("P2P node not initialized".to_string())
    }
}

#[tauri::command]
async fn test_echo(message: String) -> Result<String, String> {
    eprintln!(">>> TEST_ECHO RECEIVED: {}", message);
    tracing::info!("test_echo command received: {}", message);
    Ok(format!("Echo: {}", message))
}

#[tauri::command]
async fn set_username(state: State<'_, P2PState>, name: String) -> Result<(), String> {
    let state_guard = state.lock().await;
    let handle = state_guard
        .as_ref()
        .ok_or("P2P node not initialized")?;
    
    handle.command_tx.send(P2PCommand::SetUsername(name))
        .map_err(|e| format!("Failed to send command: {}", e))
}

#[tauri::command]
async fn cleanup_p2p(state: State<'_, P2PState>) -> Result<(), String> {
    let mut state_guard = state.lock().await;
    
    if state_guard.is_some() {
        *state_guard = None;
        Ok(())
    } else {
        Err("P2P node not initialized".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(P2PState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            test_echo,
            init_p2p,
            get_node_info,
            join_room,
            send_message,
            connect_to_peer,
            request_connection,
            accept_connection,
            reject_connection,
            set_username,
            cleanup_p2p
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}