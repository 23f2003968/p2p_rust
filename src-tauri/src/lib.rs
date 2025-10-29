mod p2p_node;

use p2p_node::{ChatMessage, P2PNode, PeerInfo};
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
    SendMessage(String),
    ConnectToPeer(String),
    GetInfo(tokio::sync::oneshot::Sender<NodeInfo>),
}

#[derive(serde::Serialize, Clone)]
struct NodeInfo {
    peer_id: String,
    addresses: Vec<String>,
    connected_peers: Vec<PeerInfo>,
}

#[tauri::command]
async fn init_p2p(app: AppHandle, state: State<'_, P2PState>) -> Result<String, String> {
    let mut state_guard = state.lock().await;
    
    if state_guard.is_some() {
        return Err("P2P node already initialized".to_string());
    }

    let (message_tx, mut message_rx) = mpsc::unbounded_channel::<ChatMessage>();
    let (command_tx, mut command_rx) = mpsc::unbounded_channel::<P2PCommand>();
    
    // Create P2P node
    let (mut node, mut swarm) = P2PNode::create(message_tx)
        .await
        .map_err(|e| e.to_string())?;

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
        content: "🚀 Node initialized - connecting to network...".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        is_self: false,
    });

    // Send mDNS enabled message
    node.send_system_message("✓ Local network discovery (mDNS) enabled".to_string());

    // Bootstrap DHT
    node.bootstrap_dht(&mut swarm);

    // Listen on IPv6
    swarm.listen_on("/ip6/::/tcp/8080".parse().unwrap()).unwrap();

    // Clone for tasks
    let app_message_relay = app.clone();
    
    // Spawn message relay task
    tokio::spawn(async move {
        while let Some(msg) = message_rx.recv().await {
            let _ = app_message_relay.emit("chat-message", msg);
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
                        P2PCommand::SendMessage(message) => {
                            node.send_message(&mut swarm, message).await;
                        }
                        P2PCommand::ConnectToPeer(addr) => {
                            node.connect_to_peer(&mut swarm, addr);
                        }
                        P2PCommand::GetInfo(tx) => {
                            let info = NodeInfo {
                                peer_id: node.get_peer_id(),
                                addresses: node.get_addresses(&swarm),
                                connected_peers: node.get_connected_peers(),
                            };
                            let _ = tx.send(info);
                        }
                    }
                    // Process any pending peer dials after handling commands
                    node.process_pending_dials(&mut swarm);
                }
                event = swarm.select_next_some() => {
                    node.handle_event(event).await;
                    // Process any pending peer dials after handling events
                    node.process_pending_dials(&mut swarm);
                }
                _ = peer_discovery_interval.tick() => {
                    // Periodically search for more peers in the current room
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
    let state_guard = state.lock().await;
    
    if let Some(handle) = state_guard.as_ref() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        handle.command_tx.send(P2PCommand::GetInfo(tx))
            .map_err(|e| e.to_string())?;
        
        rx.await.map_err(|e| e.to_string())
    } else {
        Err("P2P node not initialized".to_string())
    }
}

#[tauri::command]
async fn join_room(room_name: String, state: State<'_, P2PState>) -> Result<(), String> {
    let state_guard = state.lock().await;
    
    if let Some(handle) = state_guard.as_ref() {
        handle.command_tx.send(P2PCommand::JoinRoom(room_name))
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("P2P node not initialized".to_string())
    }
}

#[tauri::command]
async fn send_message(message: String, state: State<'_, P2PState>) -> Result<(), String> {
    let state_guard = state.lock().await;
    
    if let Some(handle) = state_guard.as_ref() {
        handle.command_tx.send(P2PCommand::SendMessage(message))
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("P2P node not initialized".to_string())
    }
}

#[tauri::command]
async fn connect_to_peer(addr: String, state: State<'_, P2PState>) -> Result<(), String> {
    let state_guard = state.lock().await;
    
    if let Some(handle) = state_guard.as_ref() {
        handle.command_tx.send(P2PCommand::ConnectToPeer(addr))
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("P2P node not initialized".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(P2PState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            init_p2p,
            get_node_info,
            join_room,
            send_message,
            connect_to_peer
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
