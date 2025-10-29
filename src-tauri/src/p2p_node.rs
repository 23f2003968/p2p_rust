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
use tracing::{info, warn};

const CHAT_PROTOCOL: StreamProtocol = StreamProtocol::new("/p2p-chat/1.0.0");

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
                // Check for link-local (fe80::/10)
                || (ipv6.segments()[0] & 0xffc0) == 0xfe80
                // Check for unique local (fc00::/7)
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
                    // Skip all IPv4 addresses due to symmetric NAT
                    is_ipv6_public = false;
                    break;
                }
                libp2p::multiaddr::Protocol::Ip6(ip) => {
                    let ip_addr = IpAddr::V6(ip);
                    // Only accept public IPv6 addresses
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

pub struct P2PNode {
    pub peer_id: PeerId,
    pub connected_peers: HashMap<PeerId, Vec<String>>,
    pub message_tx: mpsc::UnboundedSender<ChatMessage>,
    pub discovered_peers: HashSet<PeerId>,
    pub current_room: Option<gossipsub::IdentTopic>,
    pub current_room_name: Option<String>,
    pub bootstrap_peers: HashSet<PeerId>,
    pub peers_to_dial: Vec<PeerId>,
}

impl P2PNode {
    pub async fn create(
        message_tx: mpsc::UnboundedSender<ChatMessage>,
    ) -> Result<(Self, Swarm<ChatBehaviour>), Box<dyn Error>> {
        // Create swarm following the tutorial pattern
        let swarm = SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                let local_peer_id = key.public().to_peer_id();
                
                // Create Kademlia DHT
                let store = kad::store::MemoryStore::new(local_peer_id);
                let mut kad_config = kad::Config::new(CHAT_PROTOCOL.clone());
                kad_config.set_query_timeout(Duration::from_secs(60));
                let mut kad = kad::Behaviour::with_config(local_peer_id, store, kad_config);
                
                // Add bootstrap peers
                let bootstrap_peers = vec![
                    ("QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN", "/dnsaddr/bootstrap.libp2p.io"),
                    ("QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa", "/dnsaddr/bootstrap.libp2p.io"),
                    ("QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb", "/dnsaddr/bootstrap.libp2p.io"),
                    ("QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt", "/dnsaddr/bootstrap.libp2p.io"),
                ];
                
                for (peer_id_str, _) in bootstrap_peers {
                    if let Ok(peer_id) = peer_id_str.parse::<PeerId>() {
                        let addr: Multiaddr = format!("/dnsaddr/bootstrap.libp2p.io/p2p/{}", peer_id_str)
                            .parse()
                            .unwrap();
                        kad.add_address(&peer_id, addr);
                    }
                }
                
                // Enable server mode for DHT
                kad.set_mode(Some(kad::Mode::Server));
                
                // Create mDNS behaviour
                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    local_peer_id,
                )?;
                
                // Create identify behaviour
                let identify = identify::Behaviour::new(identify::Config::new(
                    CHAT_PROTOCOL.to_string(),
                    key.public(),
                ));
                
                // Create Gossipsub behaviour
                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(1))
                    .validation_mode(gossipsub::ValidationMode::Strict)
                    .message_id_fn(|message| {
                        // Use content hash as message ID to deduplicate
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        std::hash::Hash::hash(&message.data, &mut hasher);
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
        
        let node = Self::new(peer_id, message_tx);
        
        Ok((node, swarm))
    }

    fn new(
        peer_id: PeerId,
        message_tx: mpsc::UnboundedSender<ChatMessage>,
    ) -> Self {
        // Parse bootstrap peer IDs
        let bootstrap_peer_ids = vec![
            "QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
            "QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
            "QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
            "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
            "QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
        ];
        
        let mut bootstrap_peers = HashSet::new();
        for peer_id_str in bootstrap_peer_ids {
            if let Ok(pid) = peer_id_str.parse::<PeerId>() {
                bootstrap_peers.insert(pid);
            }
        }
        
        Self {
            peer_id,
            connected_peers: HashMap::new(),
            message_tx,
            discovered_peers: HashSet::new(),
            current_room: None,
            current_room_name: None,
            bootstrap_peers,
            peers_to_dial: Vec::new(),
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

    pub fn send_system_message(&self, content: String) {
        let _ = self.message_tx.send(ChatMessage {
            from: "System".to_string(),
            content,
            timestamp: chrono::Utc::now().to_rfc3339(),
            is_self: false,
        });
    }

    pub fn bootstrap_dht(&self, swarm: &mut Swarm<ChatBehaviour>) {
        // Bootstrap the DHT
        if let Err(e) = swarm.behaviour_mut().kad.bootstrap() {
            warn!("DHT bootstrap failed: {}", e);
            self.send_system_message("⚠ DHT bootstrap failed - only local discovery available".to_string());
            return;
        }
        
        self.send_system_message("✓ DHT bootstrap initiated".to_string());
        
        // Connect to bootstrap peers
        let bootstrap_peers = vec![
            "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
            "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
            "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
            "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
            "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
        ];
        
        let mut connected = 0;
        for addr_str in bootstrap_peers {
            if let Ok(addr) = addr_str.parse::<Multiaddr>() {
                info!("Attempting to dial bootstrap peer: {}", addr);
                if swarm.dial(addr.clone()).is_ok() {
                    connected += 1;
                }
            }
        }
        
        if connected > 0 {
            self.send_system_message(format!("🔗 Connecting to {} bootstrap nodes...", connected));
        } else {
            self.send_system_message("⚠ Failed to dial bootstrap peers".to_string());
        }
    }

    pub fn join_room(&mut self, swarm: &mut Swarm<ChatBehaviour>, room_name: String) {
        info!("Joining room: {}", room_name);
        
        // Create gossipsub topic from room name
        let topic = gossipsub::IdentTopic::new(room_name.clone());
        
        // Subscribe to the topic
        if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&topic) {
            warn!("Failed to subscribe to topic: {}", e);
            self.send_system_message(format!("⚠ Failed to join room '{}': {}", room_name, e));
            return;
        }
        
        self.current_room = Some(topic);
        self.current_room_name = Some(room_name.clone());
        
        self.send_system_message(format!("📢 Announcing in room '{}'...", room_name));
        
        // Announce ourselves in Kademlia for peer discovery
        if let Err(e) = swarm
            .behaviour_mut()
            .kad
            .start_providing(room_name.as_bytes().to_vec().into())
        {
            warn!("Failed to start providing: {}", e);
            self.send_system_message(format!("⚠ Failed to announce in room: {}", e));
            return;
        }

        self.send_system_message(format!("✓ Announced! Searching for peers in '{}'...", room_name));

        // Search for peers in the room via DHT
        swarm
            .behaviour_mut()
            .kad
            .get_providers(room_name.as_bytes().to_vec().into());
    }

    pub async fn send_message(&self, swarm: &mut Swarm<ChatBehaviour>, message: String) {
        // Check if we're in a room
        let topic = match &self.current_room {
            Some(t) => t,
            None => {
                self.send_system_message("⚠ Join a room first (Ctrl+J)".to_string());
                return;
            }
        };
        
        // Publish message to gossipsub topic
        match swarm.behaviour_mut().gossipsub.publish(topic.clone(), message.as_bytes()) {
            Ok(_) => {
                // Echo message back to UI as sent
                let _ = self.message_tx.send(ChatMessage {
                    from: "You".to_string(),
                    content: message,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    is_self: true,
                });
            }
            Err(e) => {
                warn!("Failed to publish message: {}", e);
                self.send_system_message(format!("⚠ Failed to send message: {}", e));
            }
        }
    }

    pub fn get_addresses(&self, swarm: &Swarm<ChatBehaviour>) -> Vec<String> {
        swarm
            .listeners()
            .map(|addr| format!("{}/p2p/{}", addr, self.peer_id))
            .collect()
    }

    pub fn connect_to_peer(&mut self, swarm: &mut Swarm<ChatBehaviour>, addr: String) {
        info!("Attempting to connect to peer at: {}", addr);
        
        // Parse the multiaddr
        match addr.parse::<Multiaddr>() {
            Ok(multiaddr) => {
                // Try to dial the address
                match swarm.dial(multiaddr.clone()) {
                    Ok(_) => {
                        self.send_system_message(format!("🔗 Dialing peer at {}...", addr));
                    }
                    Err(e) => {
                        warn!("Failed to dial peer: {}", e);
                        self.send_system_message(format!("⚠ Failed to dial peer: {}", e));
                    }
                }
            }
            Err(e) => {
                warn!("Invalid multiaddr: {}", e);
                self.send_system_message(format!("⚠ Invalid address format: {}", e));
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
                // Received a message from gossipsub
                let msg_str = String::from_utf8_lossy(&message.data);
                info!("Received message from {}: {}", propagation_source, msg_str);
                
                // Send to frontend
                let _ = self.message_tx.send(ChatMessage {
                    from: self.short_peer_id(&message.source.map(|s| s.to_string()).unwrap_or_else(|| "Unknown".to_string())),
                    content: msg_str.to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    is_self: false,
                });
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, topic })) => {
                info!("Peer {} subscribed to topic: {}", peer_id, topic);
                self.send_system_message(format!("✓ Peer {} joined the room", self.short_peer_id(&peer_id.to_string())));
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(gossipsub::Event::Unsubscribed { peer_id, topic })) => {
                info!("Peer {} unsubscribed from topic: {}", peer_id, topic);
                self.send_system_message(format!("✗ Peer {} left the room", self.short_peer_id(&peer_id.to_string())));
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                for (peer_id, multiaddr) in peers {
                    if peer_id == self.peer_id {
                        continue;
                    }
                    info!("mDNS discovered peer: {} at {}", peer_id, multiaddr);
                    
                    if !self.discovered_peers.contains(&peer_id) {
                        self.discovered_peers.insert(peer_id);
                        self.send_system_message(format!("🔍 mDNS discovered peer: {}", self.short_peer_id(&peer_id.to_string())));
                        
                        // Filter to IPv6 public addresses only
                        let filtered_addrs = filter_ipv6_public_addrs(&[multiaddr.clone()]);
                        if !filtered_addrs.is_empty() {
                            self.send_system_message(format!("Attempting to dial {} via IPv6", self.short_peer_id(&peer_id.to_string())));
                        } else {
                            self.send_system_message(format!("⚠ Peer {} has no reachable IPv6 addresses", self.short_peer_id(&peer_id.to_string())));
                        }
                    }
                }
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                for (peer_id, _) in peers {
                    self.discovered_peers.remove(&peer_id);
                    info!("mDNS peer expired: {}", peer_id);
                }
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Identify(identify::Event::Received { peer_id, info, .. })) => {
                info!("Identified peer: {}", peer_id);
                let addrs: Vec<String> = info.listen_addrs.iter().map(|a| a.to_string()).collect();
                self.connected_peers.insert(peer_id, addrs);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected to peer: {}", peer_id);
                
                // Check if this is a bootstrap peer
                if self.bootstrap_peers.contains(&peer_id) {
                    self.send_system_message(format!("✓ Connected to bootstrap node {}", self.short_peer_id(&peer_id.to_string())));
                } else {
                    self.send_system_message(format!("✓ Connected to {}", self.short_peer_id(&peer_id.to_string())));
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Disconnected from peer: {}", peer_id);
                self.connected_peers.remove(&peer_id);
                self.send_system_message(format!("✗ Disconnected from {}", self.short_peer_id(&peer_id.to_string())));
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Kad(kad::Event::RoutingUpdated { peer, .. })) => {
                info!("Routing updated for peer: {}", peer);
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Kad(kad::Event::OutboundQueryProgressed { result, .. })) => {
                match result {
                    kad::QueryResult::Bootstrap(Ok(kad::BootstrapOk { peer, num_remaining })) => {
                        info!("Bootstrap successful with peer: {} ({} remaining)", peer, num_remaining);
                        if num_remaining == 0 {
                            self.send_system_message("✓ DHT bootstrap complete - internet discovery enabled".to_string());
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
                            
                            // Skip if already connected
                            if self.connected_peers.contains_key(&peer_id) {
                                continue;
                            }
                            
                            info!("Found provider (peer) in room: {}", peer_id);
                            self.send_system_message(format!("🔍 Found peer {} in room, connecting...", self.short_peer_id(&peer_id.to_string())));
                            
                            // Queue this peer for dialing
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
        // Dial any pending peers (skip duplicates and already connected)
        while let Some(peer_id) = self.peers_to_dial.pop() {
            // Skip if already connected
            if self.connected_peers.contains_key(&peer_id) {
                continue;
            }
            
            info!("Dialing discovered peer: {}", peer_id);
            if let Err(e) = swarm.dial(peer_id) {
                warn!("Failed to dial peer {}: {}", peer_id, e);
                self.send_system_message(format!("⚠ Failed to connect to {}: {}", self.short_peer_id(&peer_id.to_string()), e));
            }
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
