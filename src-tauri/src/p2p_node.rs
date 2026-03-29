use libp2p::{
    identify, kad, mdns, noise, gossipsub,
    swarm::{NetworkBehaviour, SwarmEvent}, tcp, yamux, 
    Multiaddr, PeerId, Swarm, StreamProtocol, SwarmBuilder,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::net::IpAddr;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn, debug};
use rand::seq::IndexedRandom;

const CHAT_PROTOCOL: StreamProtocol = StreamProtocol::new("/p2p-chat/1.0.0");
const MAX_MESSAGE_LENGTH: usize = 4096;

// Network behaviour combining all protocols
#[derive(NetworkBehaviour)]
pub struct ChatBehaviour {
    pub kad: kad::Behaviour<kad::store::MemoryStore>,
    pub mdns: mdns::tokio::Behaviour,
    pub identify: identify::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub from: String,
    pub content: String,
    pub timestamp: String,
    pub is_self: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub addresses: Vec<String>,
}



// Helper function to check if an IP is private/local
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_loopback()
                || ipv4.is_private()
                || ipv4.is_link_local()
                || ipv4.is_broadcast()
                || ipv4.is_documentation()
                || ipv4.is_unspecified()
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback()
                || ipv6.is_unspecified()
                || ipv6.is_multicast()
                || (ipv6.segments()[0] & 0xffc0) == 0xfe80
                || (ipv6.segments()[0] & 0xfe00) == 0xfc00
        }
    }
}

// Filter addresses to only IPv6 public addresses
fn filter_ipv6_public_addrs(addrs: &[Multiaddr]) -> Vec<Multiaddr> {
    let mut filtered = Vec::new();

    for addr in addrs {
        let mut is_ipv6_public = false;

        for component in addr.iter() {
            match component {
                libp2p::multiaddr::Protocol::Ip4(_) => {
                    is_ipv6_public = false;
                    break;
                }
                libp2p::multiaddr::Protocol::Ip6(ip) => {
                    let ip_addr = IpAddr::V6(ip);
                    if !is_private_ip(&ip_addr) {
                        is_ipv6_public = true;
                    } else {
                        is_ipv6_public = false;
                        break;
                    }
                }
                _ => {}
            }
        }

        if is_ipv6_public {
            filtered.push(addr.clone());
        }
    }

    filtered
}

fn generate_random_name() -> String {
    let adjectives = [
        "swift", "bright", "bold", "calm", "dark", "keen", "sharp",
        "warm", "cool", "wild", "fair", "pure", "vast", "deep",
        "rapid", "silent", "vivid", "noble", "fierce", "gentle",
    ];
    let animals = [
        "fox", "wolf", "hawk", "bear", "lynx", "owl", "deer",
        "crane", "raven", "heron", "otter", "eagle", "cobra",
        "puma", "bison", "falcon", "jaguar", "panther", "tiger", "whale",
    ];
    let mut rng = rand::rng();
    let adj = adjectives.choose(&mut rng).unwrap_or(&"anon");
    let animal = animals.choose(&mut rng).unwrap_or(&"user");
    format!("{}-{}", adj, animal)
}

/// Sanitizes message input to prevent injection attacks and enforce limits
pub fn sanitize_message(message: &str) -> Result<String, String> {
    // Check for valid UTF-8 (str already guarantees this, but explicit check for safety)
    if message.is_empty() {
        return Err("Message cannot be empty".to_string());
    }

    if message.len() > MAX_MESSAGE_LENGTH {
        return Err(format!(
            "Message exceeds maximum length of {} characters",
            MAX_MESSAGE_LENGTH
        ));
    }

    // Remove any null bytes which could be used for injection
    let sanitized = message.replace('\0', "");
    
    // Trim whitespace but preserve internal spacing
    let trimmed = sanitized.trim();

    if trimmed.is_empty() {
        return Err("Message cannot be empty or whitespace only".to_string());
    }

    // Verify the trimmed message is still valid UTF-8 (should always be)
    if !trimmed.is_ascii() && trimmed.chars().all(|c| !c.is_control()) {
        // Allow non-ASCII but reject control characters
        Ok(trimmed.to_string())
    } else if trimmed.chars().all(|c| c.is_ascii() || !c.is_control()) {
        Ok(trimmed.to_string())
    } else {
        Err("Message contains invalid control characters".to_string())
    }
}

pub struct P2PNode {
    pub peer_id: PeerId,
    pub connected_peers: HashMap<PeerId, Vec<String>>,
    pub message_tx: mpsc::UnboundedSender<ChatMessage>,
    pub event_tx: mpsc::UnboundedSender<serde_json::Value>,
    pub current_room: Option<gossipsub::IdentTopic>,
    pub current_room_name: Option<String>,
    pub bootstrap_peers: HashSet<PeerId>,
    pub peers_to_dial: Vec<PeerId>,
    pub discovered_peers: HashSet<PeerId>,
    pub outgoing_dials: HashSet<PeerId>,
    pub peers_to_add_gossipsub: Vec<PeerId>,
    pub username: String,
}

impl P2PNode {
    pub async fn create(
        message_tx: mpsc::UnboundedSender<ChatMessage>,
        event_tx: mpsc::UnboundedSender<serde_json::Value>,
    ) -> Result<(Self, Swarm<ChatBehaviour>), Box<dyn Error>> {
        let swarm = SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                let local_peer_id = key.public().to_peer_id();
                
                let store = kad::store::MemoryStore::new(local_peer_id);
                let mut kad_config = kad::Config::new(CHAT_PROTOCOL.clone());
                kad_config.set_query_timeout(Duration::from_secs(60));
                let kad = kad::Behaviour::with_config(local_peer_id, store, kad_config);
                
                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    local_peer_id,
                )?;
                
                let identify = identify::Behaviour::new(identify::Config::new(
                    CHAT_PROTOCOL.to_string(),
                    key.public(),
                ));
                
                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(1))
                    .validation_mode(gossipsub::ValidationMode::Strict)
                    // Lower mesh requirements for small P2P networks (2+ nodes)
                    .mesh_n(2)
                    .mesh_n_low(1)
                    .mesh_n_high(4)
                    .mesh_outbound_min(0)
                    // Flood publish ensures messages reach all peers, not just mesh peers
                    .flood_publish(true)
                    .message_id_fn(|message| {
                        // Include source + sequence + data so same content from
                        // different senders (or at different times) is unique
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        std::hash::Hash::hash(&message.data, &mut hasher);
                        if let Some(ref source) = message.source {
                            std::hash::Hash::hash(&source.to_bytes(), &mut hasher);
                        }
                        std::hash::Hash::hash(&message.sequence_number, &mut hasher);
                        gossipsub::MessageId::from(std::hash::Hasher::finish(&hasher).to_string())
                    })
                    .build()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                
                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                
                Ok(ChatBehaviour { kad, mdns, identify, gossipsub })
            })?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(60))
            })
            .build();
        
        let peer_id = *swarm.local_peer_id();
        
        let node = Self::new(peer_id, message_tx, event_tx);
        
        Ok((node, swarm))
    }

    fn new(
        peer_id: PeerId,
        message_tx: mpsc::UnboundedSender<ChatMessage>,
        event_tx: mpsc::UnboundedSender<serde_json::Value>,
    ) -> Self {
        let bootstrap_peers = HashSet::new();
        
        Self {
            peer_id,
            connected_peers: HashMap::new(),
            message_tx,
            event_tx,
            current_room: None,
            current_room_name: None,
            bootstrap_peers,
            peers_to_dial: Vec::new(),
            discovered_peers: HashSet::new(),
            outgoing_dials: HashSet::new(),
            peers_to_add_gossipsub: Vec::new(),
            username: generate_random_name(),
        }
    }

    pub fn get_peer_id(&self) -> String {
        self.peer_id.to_string()
    }

    pub fn get_connected_peers(&self) -> Vec<PeerInfo> {
        self.connected_peers
            .iter()
            .map(|(peer_id, addrs)| PeerInfo {
                peer_id: peer_id.to_string(),
                addresses: addrs.clone(),
            })
            .collect()
    }

    pub fn set_username(&mut self, name: String) {
        self.username = name;
    }

    pub fn send_system_message(&self, content: String) {
        let msg = ChatMessage {
            from: "System".to_string(),
            content,
            timestamp: chrono::Utc::now().to_rfc3339(),
            is_self: false,
            peer_id: None,
        };
        let _ = self.message_tx.send(msg);
    }

    /// User clicked Connect — accepts both a bare peer ID or a full multiaddr
    pub fn request_connection(&mut self, swarm: &mut Swarm<ChatBehaviour>, input: &str) -> Result<(), String> {
        let (peer_id_obj, dial_addr) = if input.starts_with('/') {
            // Full multiaddr like /ip6/2409:40f4:ab:4874::3e/tcp/41395/p2p/12D3KooW...
            let addr: Multiaddr = input.parse()
                .map_err(|e| format!("Invalid multiaddr: {}", e))?;
            
            // Extract peer ID from the /p2p/ component
            let peer_id = addr.iter()
                .find_map(|proto| {
                    if let libp2p::multiaddr::Protocol::P2p(peer_id) = proto {
                        Some(peer_id)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| "Multiaddr must contain /p2p/<peer_id>".to_string())?;
            
            (peer_id, Some(addr))
        } else {
            // Bare peer ID
            let peer_id = input.parse::<PeerId>()
                .map_err(|_| "Invalid peer ID format".to_string())?;
            (peer_id, None)
        };

        // Prevent self-connection
        if peer_id_obj == self.peer_id {
            return Err("Cannot connect to yourself".to_string());
        }

        // Track that WE initiated this connection
        self.outgoing_dials.insert(peer_id_obj);

        if !self.connected_peers.contains_key(&peer_id_obj) {
            info!("Dialing peer: {} (input: {})", peer_id_obj, input);
            let dial_result = if let Some(addr) = dial_addr {
                swarm.dial(addr)
            } else {
                swarm.dial(peer_id_obj)
            };
            if let Err(e) = dial_result {
                let err_msg = format!("Failed to dial peer: {}", e);
                warn!("{}", err_msg);
                self.outgoing_dials.remove(&peer_id_obj);
                return Err(err_msg);
            }
            self.send_system_message(format!("[..] Connecting to {}...", self.short_peer_id(&peer_id_obj.to_string())));
        } else {
            info!("Already connected to peer: {}", peer_id_obj);
            let event = serde_json::json!({
                "type": "connection-established",
                "peer_id": peer_id_obj.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            let _ = self.event_tx.send(event);
        }

        Ok(())
    }

    /// User clicked Accept on an incoming request — connection already exists, just confirm
    pub fn accept_connection(&mut self, peer_id: &str) -> Result<(), String> {
        let peer_id_obj = peer_id.parse::<PeerId>()
            .map_err(|_| "Invalid peer ID format".to_string())?;

        info!("Accepting incoming connection from peer: {}", peer_id);
        
        // The TCP connection already exists (they dialed us)
        // Just emit connection-established so the frontend knows we accepted
        let event = serde_json::json!({
            "type": "connection-established",
            "peer_id": peer_id.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let _ = self.event_tx.send(event);

        // Add as gossipsub peer
        self.peers_to_add_gossipsub.push(peer_id_obj);

        self.send_system_message(format!(
            "[O] Accepted connection from {}",
            self.short_peer_id(peer_id)
        ));

        Ok(())
    }

    /// User clicked Reject — disconnect from the peer
    pub fn reject_connection(&mut self, swarm: &mut Swarm<ChatBehaviour>, peer_id: &str) -> Result<(), String> {
        let peer_id_obj = peer_id.parse::<PeerId>()
            .map_err(|_| "Invalid peer ID format".to_string())?;

        self.discovered_peers.remove(&peer_id_obj);
        self.outgoing_dials.remove(&peer_id_obj);

        // Disconnect from the peer
        let _ = swarm.disconnect_peer_id(peer_id_obj);
        self.connected_peers.remove(&peer_id_obj);
        
        self.send_system_message(format!(
            "[X] Rejected connection from {}",
            self.short_peer_id(peer_id)
        ));

        Ok(())
    }

    pub fn bootstrap_dht(&self, swarm: &mut Swarm<ChatBehaviour>) {
        // LAN-only mode: no external bootstrap peers
        // mDNS handles local peer discovery
        if let Err(e) = swarm.behaviour_mut().kad.bootstrap() {
            // This is expected to fail with no bootstrap peers — just log
            debug!("DHT bootstrap skipped (no bootstrap peers): {}", e);
        }
        self.send_system_message("[O] Local-only mode -- peers discovered via mDNS".to_string());
    }

    pub fn join_room(&mut self, swarm: &mut Swarm<ChatBehaviour>, room_name: String) {
        info!("Joining room: {}", room_name);
        
        let topic = gossipsub::IdentTopic::new(room_name.clone());
        
        if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&topic) {
            warn!("Failed to subscribe to topic: {}", e);
            self.send_system_message(format!("[!] Failed to join room '{}': {}", room_name, e));
            return;
        }
        
        self.current_room = Some(topic);
        self.current_room_name = Some(room_name.clone());
        
        self.send_system_message(format!("📢 Announcing in room '{}'...", room_name));
        
        if let Err(e) = swarm
            .behaviour_mut()
            .kad
            .start_providing(room_name.as_bytes().to_vec().into())
        {
            warn!("Failed to start providing: {}", e);
            self.send_system_message(format!("[!] Failed to announce in room: {}", e));
            return;
        }

        self.send_system_message(format!("[O] Announced! Searching for peers in '{}'...", room_name));

        swarm
            .behaviour_mut()
            .kad
            .get_providers(room_name.as_bytes().to_vec().into());
    }

    pub async fn send_message(&self, swarm: &mut Swarm<ChatBehaviour>, peer_id: String, message: String) -> Result<(), String> {
        debug!("send_message called with peer_id: {}, message length: {}", peer_id, message.len());
        
        // Validate peer ID format
        let _peer = peer_id.parse::<PeerId>()
            .map_err(|e| {
                let err = format!("Invalid peer ID format: {}", e);
                warn!("{}", err);
                err
            })?;
        
        debug!("Peer ID validation passed");
        
        // Sanitize the message
        let sanitized_msg = sanitize_message(&message)
            .map_err(|e| {
                let err = format!("Message validation failed: {}", e);
                warn!("{}", err);
                err
            })?;

        debug!("Message sanitization passed, sanitized length: {}", sanitized_msg.len());

        let topic = match &self.current_room {
            Some(t) => {
                debug!("Current room found: {:?}", self.current_room_name);
                t
            },
            None => {
                let err = "Not in a room".to_string();
                warn!("Cannot send message: {}", err);
                self.send_system_message("[!] Join a room first".to_string());
                return Err(err);
            }
        };
        
        debug!("Attempting to publish message to gossipsub topic");
        
        // Build JSON payload with username
        let payload = serde_json::json!({
            "username": if self.username.is_empty() { self.short_peer_id(&self.peer_id.to_string()) } else { self.username.clone() },
            "content": sanitized_msg,
        });
        let payload_bytes = payload.to_string().into_bytes();
        
        match swarm.behaviour_mut().gossipsub.publish(topic.clone(), payload_bytes) {
            Ok(_) => {
                info!("Message published successfully to peer: {}", peer_id);
                let _ = self.message_tx.send(ChatMessage {
                    from: "You".to_string(),
                    content: sanitized_msg,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    is_self: true,
                    peer_id: Some(peer_id),
                });
                Ok(())
            }
            Err(e) => {
                let err = format!("Failed to publish message: {}", e);
                warn!("{}", err);
                self.send_system_message(format!("[!] {}", err));
                Err(err)
            }
        }
    }

    pub fn get_addresses(&self, swarm: &Swarm<ChatBehaviour>) -> Vec<String> {
        let addrs: Vec<Multiaddr> = swarm.listeners().cloned().collect();
        let filtered = filter_ipv6_public_addrs(&addrs);
        
        filtered
            .iter()
            .map(|addr| format!("{}/p2p/{}", addr, self.peer_id))
            .collect()
    }

    pub fn connect_to_peer(&mut self, swarm: &mut Swarm<ChatBehaviour>, addr: String) -> Result<(), String> {
        info!("Attempting to connect to peer at: {}", addr);
        
        match addr.parse::<Multiaddr>() {
            Ok(multiaddr) => {
                match swarm.dial(multiaddr.clone()) {
                    Ok(_) => {
                        self.send_system_message(format!("[>] Dialing peer at {}...", addr));
                        Ok(())
                    }
                    Err(e) => {
                        warn!("Failed to dial peer: {}", e);
                        self.send_system_message(format!("[!] Failed to dial peer: {}", e));
                        Err(format!("Dial failed: {}", e))
                    }
                }
            }
            Err(e) => {
                warn!("Invalid multiaddr: {}", e);
                self.send_system_message(format!("[!] Invalid address format: {}", e));
                Err(format!("Invalid address: {}", e))
            }
        }
    }

    pub async fn handle_event(&mut self, event: SwarmEvent<ChatBehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {}", address);
                self.send_system_message(format!("🎧 Listening on {}", address));
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source,
                message_id: _,
                message,
            })) => {
                let msg_str = String::from_utf8_lossy(&message.data);
                
                // Try to parse as JSON payload with username
                let (sender_name, content) = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&msg_str) {
                    let username = json.get("username")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let content = json.get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&msg_str)
                        .to_string();
                    (username, content)
                } else {
                    // Fallback: raw text (backward compat)
                    let fallback_name = self.short_peer_id(&message.source.map(|s| s.to_string()).unwrap_or_else(|| "Unknown".to_string()));
                    (fallback_name, msg_str.to_string())
                };
                
                // Skip own messages (already displayed locally)
                if message.source == Some(self.peer_id) {
                    return;
                }
                
                match sanitize_message(&content) {
                    Ok(sanitized) => {
                        info!("Received message from {}: {}", sender_name, sanitized);
                        
                        let _ = self.message_tx.send(ChatMessage {
                            from: sender_name,
                            content: sanitized,
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            is_self: false,
                            peer_id: message.source.map(|s| s.to_string()),
                        });
                    }
                    Err(e) => {
                        warn!("Rejected invalid message from {}: {}", propagation_source, e);
                        self.send_system_message(format!("[!] Received invalid message: {}", e));
                    }
                }
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, topic })) => {
                info!("Peer {} subscribed to topic: {}", peer_id, topic);
                self.send_system_message(format!("[O] Peer {} joined the room", self.short_peer_id(&peer_id.to_string())));
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(gossipsub::Event::Unsubscribed { peer_id, topic })) => {
                info!("Peer {} unsubscribed from topic: {}", peer_id, topic);
                self.send_system_message(format!("[X] Peer {} left the room", self.short_peer_id(&peer_id.to_string())));
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                for (peer_id, multiaddr) in peers {
                    if peer_id == self.peer_id {
                        continue;
                    }
                    info!("mDNS discovered peer: {} at {}", peer_id, multiaddr);
                    
                    // Skip if already connected or already discovered
                    if self.connected_peers.contains_key(&peer_id) {
                        continue;
                    }
                    
                    if self.bootstrap_peers.contains(&peer_id) {
                        // Auto-dial bootstrap peers without user confirmation
                        self.peers_to_dial.push(peer_id);
                        continue;
                    }
                    
                    // Emit peer-discovered event — show as available peer in sidebar
                    // Do NOT auto-dial or show connection dialog
                    if !self.discovered_peers.contains(&peer_id) {
                        self.discovered_peers.insert(peer_id);
                        
                        let event = serde_json::json!({
                            "type": "peer-discovered",
                            "peer_id": peer_id.to_string(),
                            "address": multiaddr.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });
                        let _ = self.event_tx.send(event);
                        info!("Emitted peer-discovered event for peer: {}", peer_id);
                        
                        self.send_system_message(format!("[>] Discovered peer: {}", self.short_peer_id(&peer_id.to_string())));
                    }
                }
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                for (peer_id, _) in peers {
                    debug!("mDNS peer expired: {}", peer_id);
                    self.discovered_peers.remove(&peer_id);
                    
                    let event = serde_json::json!({
                        "type": "peer-expired",
                        "peer_id": peer_id.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });
                    let _ = self.event_tx.send(event);
                }
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Identify(identify::Event::Received { peer_id, info, .. })) => {
                info!("Identified peer: {}", peer_id);
                let addrs: Vec<String> = info.listen_addrs.iter().map(|a| a.to_string()).collect();
                self.connected_peers.insert(peer_id, addrs);
                
                // Add as gossipsub explicit peer so messaging works with small networks
                if !self.bootstrap_peers.contains(&peer_id) {
                    self.peers_to_add_gossipsub.push(peer_id);
                }
            }
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                info!("Connection established with peer: {} (endpoint: {:?})", peer_id, endpoint);
                
                if self.bootstrap_peers.contains(&peer_id) {
                    self.send_system_message(format!("[O] Connected to bootstrap node {}", self.short_peer_id(&peer_id.to_string())));
                } else if self.outgoing_dials.remove(&peer_id) {
                    // WE initiated this connection — tell our frontend it's established
                    info!("Outgoing connection established with peer: {}", peer_id);
                    self.send_system_message(format!("[O] Connected to {}", self.short_peer_id(&peer_id.to_string())));
                    let event = serde_json::json!({
                        "type": "connection-established",
                        "peer_id": peer_id.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });
                    let _ = self.event_tx.send(event);
                } else if endpoint.is_listener() {
                    // Someone dialed US — show in Requests section on our frontend
                    info!("Incoming connection from peer: {} - emitting incoming-connection event", peer_id);
                    self.send_system_message(format!("[>] Connection request from {}", self.short_peer_id(&peer_id.to_string())));
                    let event = serde_json::json!({
                        "type": "incoming-connection",
                        "peer_id": peer_id.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });
                    let _ = self.event_tx.send(event);
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Disconnected from peer: {}", peer_id);
                self.connected_peers.remove(&peer_id);
                self.send_system_message(format!("[X] Disconnected from {}", self.short_peer_id(&peer_id.to_string())));
                
                // Emit peer-disconnected event to frontend
                let event = serde_json::json!({
                    "type": "peer-disconnected",
                    "peer_id": peer_id.to_string(),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                let _ = self.event_tx.send(event);
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Kad(kad::Event::RoutingUpdated { peer, .. })) => {
                info!("Routing updated for peer: {}", peer);
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Kad(kad::Event::OutboundQueryProgressed { result, .. })) => {
                match result {
                    kad::QueryResult::Bootstrap(Ok(kad::BootstrapOk { peer, num_remaining })) => {
                        info!("Bootstrap successful with peer: {} ({} remaining)", peer, num_remaining);
                        if num_remaining == 0 {
                            self.send_system_message("[O] DHT bootstrap complete - internet discovery enabled".to_string());
                        }
                    }
                    kad::QueryResult::Bootstrap(Err(e)) => {
                        warn!("Bootstrap error: {:?}", e);
                    }
                    kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders { providers, .. })) => {
                        for peer_id in providers {
                            if peer_id == self.peer_id {
                                continue;
                            }
                            
                            if self.connected_peers.contains_key(&peer_id) {
                                continue;
                            }
                            
                            info!("Found provider (peer) in room: {}", peer_id);
                            self.send_system_message(format!("[>] Found peer {} in room, connecting...", self.short_peer_id(&peer_id.to_string())));
                            
                            self.peers_to_dial.push(peer_id);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn process_pending_dials(&mut self, swarm: &mut Swarm<ChatBehaviour>) {
        while let Some(peer_id) = self.peers_to_dial.pop() {
            if self.connected_peers.contains_key(&peer_id) {
                continue;
            }
            
            info!("Dialing discovered peer: {}", peer_id);
            if let Err(e) = swarm.dial(peer_id) {
                warn!("Failed to dial peer {}: {}", peer_id, e);
                self.send_system_message(format!("[!] Failed to connect to {}: {}", self.short_peer_id(&peer_id.to_string()), e));
            }
        }
    }

    pub fn process_pending_gossipsub(&mut self, swarm: &mut Swarm<ChatBehaviour>) {
        while let Some(peer_id) = self.peers_to_add_gossipsub.pop() {
            info!("Adding {} as explicit gossipsub peer", peer_id);
            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
        }
    }

    fn short_peer_id(&self, peer_id: &str) -> String {
        if peer_id.len() > 16 {
            format!("{}...{}", &peer_id[..8], &peer_id[peer_id.len() - 6..])
        } else {
            peer_id.to_string()
        }
    }
}