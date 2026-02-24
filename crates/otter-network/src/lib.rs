//! # Otter Network
//!
//! Peer-to-peer networking layer for the Otter decentralized chat platform.
//!
//! This crate provides:
//! - libp2p-based peer discovery (mDNS and Kademlia DHT)
//! - Connection management
//! - Custom chat protocol
//! - Peer information and routing
//! - WebRTC transport with ICE negotiation for NAT traversal
//! - Bootstrap peer discovery with DNS fallback
//! - NAT traversal with AutoNAT and relay support

pub mod webrtc;
pub mod bootstrap;
pub mod heartbeat;

use futures::{prelude::*, select};
use libp2p::{
    core::transport::upgrade,
    gossipsub, identify, kad,
    mdns,
    noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, PeerId, Swarm, Multiaddr, Transport,
};
use std::{
    collections::{HashSet, HashMap},
    time::{Duration, Instant},
};
use thiserror::Error as ThisError;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

#[derive(ThisError, Debug)]
pub enum NetworkError {
    #[error("Failed to create network: {0}")]
    InitializationError(String),
    #[error("Failed to listen on address: {0}")]
    ListenError(String),
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
    #[error("Send error: {0}")]
    SendError(String),
    #[error("Transport error: {0}")]
    TransportError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkErrorCategory {
    Transient,
    Permanent,
    Configuration,
}

impl NetworkError {
    pub fn category(&self) -> NetworkErrorCategory {
        match self {
            NetworkError::InitializationError(_) => NetworkErrorCategory::Configuration,
            NetworkError::ListenError(_) => NetworkErrorCategory::Configuration,
            NetworkError::PeerNotFound(_) => NetworkErrorCategory::Permanent,
            NetworkError::SendError(_) => NetworkErrorCategory::Transient,
            NetworkError::TransportError(_) => NetworkErrorCategory::Transient,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OtterNetworkConfig {
    pub min_peers: usize,
    pub max_peers: usize,
    pub target_peers: usize,
    pub discovery_throttle_secs: u64,
    pub heartbeat_interval_secs: u64,
    pub heartbeat_freshness_secs: u64,
}

impl Default for OtterNetworkConfig {
    fn default() -> Self {
        Self {
            min_peers: 3,
            max_peers: 20,
            target_peers: 12,
            discovery_throttle_secs: 30,
            heartbeat_interval_secs: 15,
            heartbeat_freshness_secs: 60,
        }
    }
}

impl OtterNetworkConfig {
    pub fn load_default() -> Self {
        let config_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".otter")
            .join("network_config.toml");

        match std::fs::read_to_string(&config_path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_else(|e| {
                warn!("Failed to parse config {}, using defaults: {}", config_path.display(), e);
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }
}

/// Events from the network layer
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// A new peer was discovered
    PeerDiscovered { peer_id: PeerId, addresses: Vec<String> },
    /// Network is ready for use (GUI-friendly)
    NetworkReady { mesh_peer_count: usize },
    /// A peer is online (GUI-friendly)
    PeerOnline { peer_id: PeerId, nickname: Option<String>, avatar: Option<String> },
    /// A peer went offline (GUI-friendly)
    PeerOffline { peer_id: PeerId },
    /// Network is degraded (too few peers)
    NetworkDegraded { connected_count: usize },
    /// Peer discovery in progress (connection pool)
    DiscoveringPeers { connected_count: usize },
    /// Connection quality update for a peer
    PeerQualityUpdate { peer_id: PeerId, score: f64 },
    /// Network health report
    HealthReport {
        peer_count: usize,
        error_rate: f64,
        avg_latency_ms: Option<u32>,
        dht_size: usize,
    },
    /// A peer subscribed to gossipsub and is ready for messages
    PeerReadyForMessages { peer_id: PeerId },
    /// Received a message from a peer
    MessageReceived { from: PeerId, data: Vec<u8> },
    /// Network listening started
    ListeningOn { address: String },
    /// Cached peers loaded from disk at startup (Phase 3)
    CachedPeersLoaded { count: usize },
}

/// Commands to the network layer
#[derive(Debug)]
pub enum NetworkCommand {
    /// Send a message to a specific peer
    SendMessage { to: PeerId, data: Vec<u8> },
    /// Request list of connected peers
    ListPeers { response: mpsc::Sender<Vec<PeerId>> },
    /// Dial a specific peer
    DialPeer { peer_id: PeerId, address: String },
    /// Query DHT for more peers (used after bootstrap)
    QueryBootstrap { bootstrap_peer_id: PeerId },
}

/// Network behavior combining multiple protocols
#[derive(NetworkBehaviour)]
pub struct OtterBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
    kad: kad::Behaviour<kad::store::MemoryStore>,
    identify: identify::Behaviour,
}

#[derive(Debug, Clone)]
struct PeerRetryState {
    consecutive_failures: u32,
    retry_attempts: u8,
    next_retry: Option<Instant>,
    last_address: Option<Multiaddr>,
    circuit_open_until: Option<Instant>,
    blacklist_until: Option<Instant>,
}

/// The main network manager
pub struct Network {
    swarm: Swarm<OtterBehaviour>,
    event_tx: mpsc::Sender<NetworkEvent>,
    command_rx: mpsc::Receiver<NetworkCommand>,
    connected_peers: HashSet<PeerId>,
    gossipsub_topic: gossipsub::IdentTopic,
    bootstrap_peers: HashSet<PeerId>,  // Track bootstrap peer connections
    bootstrap_query_started: bool,  // Track if we've queried
    local_peer_id: PeerId,  // Our own peer ID for provider registration
    discovered_mesh_peers: HashSet<PeerId>,  // Track peers discovered via DHT to avoid duplicate dials
    provider_registered: bool,  // Track if we've registered as Otter provider
    heartbeat_topic: gossipsub::IdentTopic,  // Dedicated topic for presence heartbeats
    peer_heartbeats: HashMap<PeerId, Instant>,  // Track last heartbeat from each peer
    last_heartbeat_sent: Instant,  // When we last sent our heartbeat
    bootstrap_sources: Option<bootstrap::BootstrapSources>,  // Phase 3: Persistent cache
    network_ready_sent: bool,
    min_peers: usize,
    max_peers: usize,
    target_peers: usize,
    last_dht_query: Instant,
    peer_retry_state: HashMap<PeerId, PeerRetryState>,
    error_count_window: u32,
    last_error_reset: Instant,
    last_health_report: Instant,
    config: OtterNetworkConfig,
}

impl Network {
    /// Create a new network instance
    pub fn new(
        event_tx: mpsc::Sender<NetworkEvent>,
        command_rx: mpsc::Receiver<NetworkCommand>,
    ) -> Result<Self, NetworkError> {
        // Generate a new keypair for this peer
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        let config = OtterNetworkConfig::load_default();
        
        info!("Local peer ID: {}", local_peer_id);
        
        // Create a transport
        let transport = tcp::tokio::Transport::default()
            .upgrade(upgrade::Version::V1Lazy)
            .authenticate(noise::Config::new(&local_key).unwrap())
            .multiplex(yamux::Config::default())
            .timeout(Duration::from_secs(20))
            .boxed();
        
        // Configure Gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .map_err(|e| NetworkError::InitializationError(e.to_string()))?;
        
        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )
        .map_err(|e| NetworkError::InitializationError(e.to_string()))?;
        
        // Create mDNS for local peer discovery
        let mdns = mdns::tokio::Behaviour::new(
            mdns::Config::default(),
            local_peer_id,
        )
        .map_err(|e| NetworkError::InitializationError(e.to_string()))?;
        
        // Create Kademlia DHT in server mode to answer queries from other peers
        let store = kad::store::MemoryStore::new(local_peer_id);
        let mut kad = kad::Behaviour::new(local_peer_id, store);
        kad.set_mode(Some(kad::Mode::Server));
        
        // Create identify protocol
        let identify = identify::Behaviour::new(identify::Config::new(
            "/otter/1.0.0".to_string(),
            local_key.public(),
        ));
        
        // Combine behaviors
        let behaviour = OtterBehaviour {
            gossipsub,
            mdns,
            kad,
            identify,
        };
        
        // Create swarm with custom config to prevent idle disconnections
        // Default idle_connection_timeout is 120 seconds (2 minutes) which causes unwanted disconnections
        // Set to 10 minutes - balances resource usage with connection stability
        let swarm_config = libp2p::swarm::Config::with_tokio_executor()
            .with_idle_connection_timeout(Duration::from_secs(600)); // 10 minutes
        
        let swarm = Swarm::new(transport, behaviour, local_peer_id, swarm_config);
        
        // Create gossipsub topics
        let gossipsub_topic = gossipsub::IdentTopic::new("otter-chat");
        let heartbeat_topic = gossipsub::IdentTopic::new("otter:presence:v1");
        
        Ok(Self {
            swarm,
            event_tx,
            command_rx,
            connected_peers: HashSet::new(),
            gossipsub_topic,
            bootstrap_peers: HashSet::new(),
            bootstrap_query_started: false,
            local_peer_id,
            discovered_mesh_peers: HashSet::new(),
            provider_registered: false,
            heartbeat_topic,
            peer_heartbeats: HashMap::new(),
            last_heartbeat_sent: Instant::now(),
            bootstrap_sources: None,
            network_ready_sent: false,
            min_peers: config.min_peers,
            max_peers: config.max_peers,
            target_peers: config.target_peers,
            last_dht_query: Instant::now() - Duration::from_secs(60),
            peer_retry_state: HashMap::new(),
            error_count_window: 0,
            last_error_reset: Instant::now(),
            last_health_report: Instant::now(),
            config,
        })
    }

    /// Mark a peer as a bootstrap peer
    pub fn mark_as_bootstrap(&mut self, peer_id: PeerId) {
        self.bootstrap_peers.insert(peer_id);
        info!("📍 Marked as bootstrap peer: {}", peer_id);
    }

    /// Register this peer as a provider for Otter network discovery
    /// This allows other peers to find us via DHT queries
    pub fn register_as_otter_provider(&mut self) {
        if self.provider_registered {
            debug!("Already registered as Otter provider, skipping");
            return;
        }

        let key = kad::RecordKey::from(b"otter:discovery:v1".to_vec());
        
        match self.swarm.behaviour_mut().kad.start_providing(key.clone()) {
            Ok(_) => {
                self.provider_registered = true;
                info!("📡 Registered as Otter provider (TTL: 24h)");
            }
            Err(e) => {
                warn!("Failed to register as Otter provider: {}", e);
            }
        }
    }
    
    /// Start listening on the given address
    pub fn listen(&mut self, addr: &str) -> Result<(), NetworkError> {
        let addr: Multiaddr = addr
            .parse()
            .map_err(|e| NetworkError::ListenError(format!("Invalid address: {}", e)))?;
        
        self.swarm
            .listen_on(addr)
            .map_err(|e| NetworkError::ListenError(e.to_string()))?;
        
        // Subscribe to gossipsub topics (chat + heartbeat)
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.gossipsub_topic)
            .map_err(|e| NetworkError::InitializationError(format!("Subscribe error: {}", e)))?;
        
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.heartbeat_topic)
            .map_err(|e| NetworkError::InitializationError(format!("Heartbeat subscribe error: {}", e)))?;
        
        info!("📡 Subscribed to chat and heartbeat topics");
        
        Ok(())
    }
    
    /// Publish heartbeat to indicate we're online
    pub fn publish_heartbeat(&mut self) {
        use crate::heartbeat::HeartbeatMessage;
        
        let heartbeat = HeartbeatMessage::new(self.local_peer_id.to_string());
        
        match heartbeat.to_bytes() {
            Ok(data) => {
                match self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(self.heartbeat_topic.clone(), data)
                {
                    Ok(_) => {
                        self.last_heartbeat_sent = Instant::now();
                        debug!("💓 Heartbeat published");
                    }
                    Err(e) => {
                        warn!("Failed to publish heartbeat: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to serialize heartbeat: {}", e);
            }
        }
    }
    
    /// Check if heartbeat should be sent (every 15 seconds)
    pub fn should_send_heartbeat(&self) -> bool {
        self.last_heartbeat_sent.elapsed() >= Duration::from_secs(self.config.heartbeat_interval_secs)
    }
    
    /// Get list of peers with fresh heartbeats (< 60s old)
    pub fn get_alive_peers(&self) -> Vec<PeerId> {
        let now = Instant::now();
        self.peer_heartbeats
            .iter()
            .filter(|(_, last_seen)| {
                now.duration_since(**last_seen) < Duration::from_secs(self.config.heartbeat_freshness_secs)
            })
            .map(|(peer_id, _)| *peer_id)
            .collect()
    }

    /// Calculate quality score for a peer (Phase 5)
    fn peer_quality(&self, peer_id: &PeerId) -> f64 {
        if let Some(sources) = &self.bootstrap_sources {
            if let Some(peer) = sources.cache().peers.iter().find(|p| p.peer_id == peer_id.to_string()) {
                return peer.reputation_score();
            }
        }

        0.5
    }

    /// Trigger DHT discovery if below minimum peers (Phase 5)
    async fn maybe_trigger_discovery(&mut self) {
        if self.connected_peers.len() < self.min_peers
            && self.last_dht_query.elapsed() >= Duration::from_secs(self.config.discovery_throttle_secs)
        {
            self.last_dht_query = Instant::now();
            let query_id = self.swarm.behaviour_mut().kad.get_closest_peers(PeerId::random());
            info!("🔍 Discovery triggered (connected: {}) query: {:?}", self.connected_peers.len(), query_id);
            let _ = self.event_tx.send(NetworkEvent::DiscoveringPeers {
                connected_count: self.connected_peers.len(),
            }).await;
        }
    }

    /// Prune excess peers based on quality score (Phase 5)
    async fn prune_excess_peers(&mut self) {
        if self.connected_peers.len() <= self.max_peers {
            return;
        }

        let prune_count = self.connected_peers.len().saturating_sub(self.max_peers);
        if prune_count == 0 {
            return;
        }

        let mut candidates: Vec<(PeerId, f64)> = self.connected_peers
            .iter()
            .filter(|peer_id| !self.bootstrap_peers.contains(peer_id))
            .map(|peer_id| (*peer_id, self.peer_quality(peer_id)))
            .collect();

        if candidates.is_empty() {
            return;
        }

        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        for (peer_id, score) in candidates.into_iter().take(prune_count) {
            info!("🧹 Pruning peer {} (quality: {:.2})", peer_id, score);
            if let Err(e) = self.swarm.disconnect_peer_id(peer_id) {
                warn!("Failed to disconnect peer {}: {:?}", peer_id, e);
            }
        }
    }

    fn record_dial_success(&mut self, peer_id: PeerId) {
        if let Some(state) = self.peer_retry_state.get_mut(&peer_id) {
            state.consecutive_failures = 0;
            state.retry_attempts = 0;
            state.next_retry = None;
            state.circuit_open_until = None;
            state.blacklist_until = None;
        }
    }

    fn record_dial_failure(&mut self, peer_id: PeerId, address: Option<Multiaddr>, category: NetworkErrorCategory) {
        self.error_count_window += 1;

        let state = self.peer_retry_state.entry(peer_id).or_insert(PeerRetryState {
            consecutive_failures: 0,
            retry_attempts: 0,
            next_retry: None,
            last_address: None,
            circuit_open_until: None,
            blacklist_until: None,
        });

        state.consecutive_failures += 1;
        if let Some(addr) = address {
            state.last_address = Some(addr);
        }

        if category == NetworkErrorCategory::Permanent {
            state.blacklist_until = Some(Instant::now() + Duration::from_secs(3600));
            return;
        }

        if state.consecutive_failures >= 5 {
            state.circuit_open_until = Some(Instant::now() + Duration::from_secs(300));
            return;
        }

        if state.retry_attempts < 3 {
            let backoff_secs = 2u64.pow(state.retry_attempts as u32);
            state.retry_attempts += 1;
            state.next_retry = Some(Instant::now() + Duration::from_secs(backoff_secs));
        }
    }

    async fn process_retries(&mut self) {
        let now = Instant::now();
        let mut to_retry: Vec<(PeerId, Multiaddr)> = Vec::new();

        for (peer_id, state) in self.peer_retry_state.iter_mut() {
            if let Some(until) = state.blacklist_until {
                if until > now {
                    continue;
                }
                state.blacklist_until = None;
            }

            if let Some(until) = state.circuit_open_until {
                if until > now {
                    continue;
                }
                state.circuit_open_until = None;
            }

            if let Some(next_retry) = state.next_retry {
                if next_retry <= now {
                    if let Some(addr) = state.last_address.clone() {
                        to_retry.push((*peer_id, addr));
                    }
                    state.next_retry = None;
                }
            }
        }

        for (peer_id, addr) in to_retry {
            if let Err(e) = self.swarm.dial(addr.clone()) {
                warn!("Retry dial failed for {}: {}", peer_id, e);
                self.record_dial_failure(peer_id, Some(addr), NetworkErrorCategory::Transient);
            }
        }
    }

    async fn maybe_emit_health_report(&mut self) {
        if self.last_health_report.elapsed() < Duration::from_secs(60) {
            return;
        }

        let error_rate = self.error_count_window as f64 / 60.0;
        self.error_count_window = 0;
        self.last_error_reset = Instant::now();
        self.last_health_report = Instant::now();

        let mut latency_sum = 0u64;
        let mut latency_count = 0u64;
        if let Some(sources) = &self.bootstrap_sources {
            for peer_id in &self.connected_peers {
                if let Some(peer) = sources.cache().peers.iter().find(|p| p.peer_id == peer_id.to_string()) {
                    if let Some(lat) = peer.latency_ms {
                        latency_sum += lat as u64;
                        latency_count += 1;
                    }
                }
            }
        }
        let avg_latency_ms = if latency_count > 0 {
            Some((latency_sum / latency_count) as u32)
        } else {
            None
        };

        let dht_size: usize = self.swarm.behaviour_mut().kad.kbuckets()
            .map(|bucket| bucket.iter().count())
            .sum();

        let _ = self.event_tx.send(NetworkEvent::HealthReport {
            peer_count: self.connected_peers.len(),
            error_rate,
            avg_latency_ms,
            dht_size,
        }).await;
    }
    
    /// Handle received heartbeat message
    fn handle_heartbeat_message(&mut self, from: PeerId, data: &[u8]) {
        use crate::heartbeat::HeartbeatMessage;
        
        match HeartbeatMessage::from_bytes(data) {
            Ok(heartbeat) => {
                // Verify the heartbeat is fresh
                if heartbeat.is_fresh(self.config.heartbeat_freshness_secs as i64) {
                    // Update last seen time
                    self.peer_heartbeats.insert(from, Instant::now());
                    
                    let age = heartbeat.age_secs();
                    debug!("💓 Heartbeat from {} (age: {}s)", from, age);
                } else {
                    warn!("Received stale heartbeat from {} (age: {}s)", from, heartbeat.age_secs());
                }
            }
            Err(e) => {
                warn!("Failed to parse heartbeat from {}: {}", from, e);
            }
        }
    }
    
    /// Dial a peer by multiaddr (used by bootstrap)
    pub fn dial(&mut self, addr: &Multiaddr) -> Result<(), NetworkError> {
        self.swarm
            .dial(addr.clone())
            .map_err(|e| NetworkError::TransportError(e.to_string()))
    }
    
    /// Initialize bootstrap sources and load cached peers (Phase 3)
    pub async fn load_cached_peers(&mut self, cache_path: std::path::PathBuf) -> Result<(), NetworkError> {
        let mut sources = bootstrap::BootstrapSources::new(cache_path);
        sources.initialize().await
            .map_err(|e| NetworkError::InitializationError(format!("Bootstrap init failed: {}", e)))?;
        
        // Get peers suitable for auto-dial
        let auto_dial_peers = sources.get_peers_for_auto_dial();
        let count = auto_dial_peers.len();
        
        if count > 0 {
            info!("💾 Loaded {} cached peers for auto-dial", count);
            
            // Auto-dial cached peers (limit to avoid overwhelming the network)
            let max_auto_dials = 5;
            for (peer_id_str, addrs) in auto_dial_peers.iter().take(max_auto_dials) {
                for addr in addrs {
                    if let Ok(peer_id) = peer_id_str.parse::<PeerId>() {
                        let full_addr = addr.clone().with(libp2p::multiaddr::Protocol::P2p(peer_id.into()));
                        match self.dial(&full_addr) {
                            Ok(_) => {
                                debug!("📞 Attempting to reconnect to cached peer {}",  peer_id);
                            }
                            Err(e) => {
                                debug!("Could not dial cached peer {}: {}", peer_id, e);
                            }
                        }
                    }
                }
            }
            
            // Emit event for cached peers loaded
            let _ = self.event_tx.send(NetworkEvent::CachedPeersLoaded { count }).await;
        } else {
            info!("📝 No cached peers to load");
        }

        self.bootstrap_sources = Some(sources);
        Ok(())
    }
    
    /// Add peer address to DHT routing table (used by bootstrap gossip)
    pub fn add_dht_peer(&mut self, peer_id: &PeerId, addr: &Multiaddr) {
        self.swarm
            .behaviour_mut()
            .kad
            .add_address(peer_id, addr.clone());
    }

    /// Get local peer id
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Override connection pool parameters (primarily for tests)
    pub fn set_pool_params(&mut self, min_peers: usize, max_peers: usize, target_peers: usize) {
        self.min_peers = min_peers;
        self.max_peers = max_peers;
        self.target_peers = target_peers;
        self.config.min_peers = min_peers;
        self.config.max_peers = max_peers;
        self.config.target_peers = target_peers;
    }
    
    /// Run the network event loop
    pub async fn run(mut self) ->Result<(), NetworkError> {
        loop {
            // Check if we should send heartbeat
            if self.should_send_heartbeat() {
                self.publish_heartbeat();
            }

            // Connection pool: trigger discovery when below minimum peers
            self.maybe_trigger_discovery().await;

            // Error handling: process scheduled retries
            self.process_retries().await;

            // Health monitoring
            self.maybe_emit_health_report().await;
            
            select! {
                event = self.swarm.select_next_some() => {
                    if let Err(e) = self.handle_swarm_event(event).await {
                        warn!("Error handling swarm event: {}", e);
                    }
                }
                command = self.command_rx.recv().fuse() => {
                    match command {
                        Some(cmd) => {
                            if let Err(e) = self.handle_command(cmd).await {
                                warn!("Error handling command: {}", e);
                            }
                        }
                        None => {
                            info!("Command channel closed, shutting down network");
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn handle_swarm_event<THandlerErr>(
        &mut self,
        event: SwarmEvent<OtterBehaviourEvent, THandlerErr>,
    ) -> Result<(), NetworkError>
    where
        THandlerErr: std::error::Error,
    {
        match event {
            SwarmEvent::Behaviour(OtterBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (peer_id, multiaddr) in list {
                    debug!("Discovered peer: {} at {}", peer_id, multiaddr);
                    
                    // Add to Kademlia DHT
                    self.swarm
                        .behaviour_mut()
                        .kad
                        .add_address(&peer_id, multiaddr.clone());
                    
                    let _ = self.event_tx.send(NetworkEvent::PeerDiscovered {
                        peer_id,
                        addresses: vec![multiaddr.to_string()],
                    }).await;
                }
            }
            
            SwarmEvent::Behaviour(OtterBehaviourEvent::Gossipsub(
                gossipsub::Event::Message {
                    propagation_source,
                    message,
                    ..
                },
            )) => {
                // Check if this is a heartbeat message
                if message.topic == self.heartbeat_topic.hash() {
                    self.handle_heartbeat_message(propagation_source, &message.data);
                } else {
                    // Regular chat message
                    debug!("Received chat message from {}", propagation_source);
                    
                    let _ = self.event_tx.send(NetworkEvent::MessageReceived {
                        from: propagation_source,
                        data: message.data,
                    }).await;
                }
            }
            
            SwarmEvent::Behaviour(OtterBehaviourEvent::Gossipsub(
                gossipsub::Event::Subscribed { peer_id, .. }
            )) => {
                info!("Peer {} subscribed to gossipsub topic", peer_id);
                
                // Notify that peer is ready for messages via gossipsub
                let _ = self.event_tx.send(NetworkEvent::PeerReadyForMessages { peer_id }).await;
            }
            
            SwarmEvent::Behaviour(OtterBehaviourEvent::Gossipsub(
                gossipsub::Event::Unsubscribed { peer_id, .. }
            )) => {
                info!("Peer {} unsubscribed from gossipsub topic", peer_id);
            }
            
            SwarmEvent::Behaviour(OtterBehaviourEvent::Gossipsub(event)) => {
                // Log any other gossipsub events (validation failures, etc.) at debug level
                debug!("Unhandled gossipsub event: {:?}", event);
            }

            SwarmEvent::Behaviour(OtterBehaviourEvent::Kad(kad::Event::OutboundQueryProgressed {
                result: kad::QueryResult::GetClosestPeers(Ok(ok)),
                ..
            })) => {
                info!("🔍 DHT query found {} closest peers", ok.peers.len());
                
                // Extract peer IDs and try dialing them (DHT query doesn't always return addresses)
                for peer_id in ok.peers {
                    if peer_id == self.local_peer_id {
                        continue; // Don't dial ourselves
                    }
                    
                    if self.connected_peers.contains(&peer_id) {
                        debug!("Peer {} already connected, skipping", peer_id);
                        continue;
                    }
                    
                    if self.discovered_mesh_peers.contains(&peer_id) {
                        debug!("Peer {} already discovered, skipping duplicate dial", peer_id);
                        continue;
                    }
                    
                    // Mark as discovered to prevent duplicate dials
                    self.discovered_mesh_peers.insert(peer_id);
                    
                    info!("📞 Attempting to dial mesh peer {} discovered via DHT", peer_id);
                    
                    // Dial by peer ID - libp2p will use routing table to find addresses
                    if let Err(e) = self.swarm.dial(peer_id) {
                        warn!("Failed to dial mesh peer {}: {}", peer_id, e);
                        self.record_dial_failure(peer_id, None, NetworkErrorCategory::Transient);
                    }
                }
            }
            
            SwarmEvent::Behaviour(OtterBehaviourEvent::Kad(kad::Event::OutboundQueryProgressed {
                result: kad::QueryResult::GetClosestPeers(Err(e)),
                ..
            })) => {
                warn!("DHT query failed: {:?}", e);
            }
            
            SwarmEvent::Behaviour(OtterBehaviourEvent::Kad(event)) => {
                debug!("Kademlia event: {:?}", event);
            }
            
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                info!("Connected to peer: {}", peer_id);
                self.connected_peers.insert(peer_id);
                self.record_dial_success(peer_id);
                
                // Record successful dial in cache (Phase 3.2)
                if let Some(sources) = &mut self.bootstrap_sources {
                    // Use the connection endpoint multiaddr if available
                    let addr = endpoint.get_remote_address();
                    sources.record_successful_dial(&peer_id, addr, 0).await;
                }
                
                // If this is a bootstrap peer, register as provider and start DHT query
                if self.bootstrap_peers.contains(&peer_id) {
                    // Register ourselves as an Otter provider so others can find us
                    self.register_as_otter_provider();
                    
                    // Start DHT query to discover mesh peers
                    if !self.bootstrap_query_started {
                        self.bootstrap_query_started = true;
                        let query_id = self.swarm.behaviour_mut().kad.get_closest_peers(PeerId::random());
                        info!("🔍 Started DHT query through bootstrap peer {}: {:?}", peer_id, query_id);
                    }
                }
                
                let nickname = self.bootstrap_sources.as_ref().and_then(|sources| {
                    sources.cache().peers.iter()
                        .find(|p| p.peer_id == peer_id.to_string())
                        .and_then(|p| p.nickname.clone())
                });

                let quality = self.peer_quality(&peer_id);
                let _ = self.event_tx.send(NetworkEvent::PeerQualityUpdate {
                    peer_id,
                    score: quality,
                }).await;

                let _ = self.event_tx.send(NetworkEvent::PeerOnline {
                    peer_id,
                    nickname,
                    avatar: None,
                }).await;

                self.prune_excess_peers().await;

                if !self.network_ready_sent {
                    let mesh_peer_count = self.connected_peers.len();
                    let _ = self.event_tx.send(NetworkEvent::NetworkReady { mesh_peer_count }).await;
                    self.network_ready_sent = true;
                }
            }
            
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Disconnected from peer: {}", peer_id);
                self.connected_peers.remove(&peer_id);

                let _ = self.event_tx.send(NetworkEvent::PeerOffline { peer_id }).await;

                if self.connected_peers.len() < 2 {
                    let _ = self.event_tx.send(NetworkEvent::NetworkDegraded {
                        connected_count: self.connected_peers.len(),
                    }).await;
                }
            }
            
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on: {}", address);
                
                let _ = self.event_tx.send(NetworkEvent::ListeningOn {
                    address: address.to_string(),
                }).await;
            }
            
            _ => {}
        }
        
        Ok(())
    }
    
    async fn handle_command(&mut self, command: NetworkCommand) -> Result<(), NetworkError> {
        match command {
            NetworkCommand::SendMessage { to, data } => {
                // NOTE: 'to' parameter is currently ignored - gossipsub broadcasts to all subscribers.
                // E2E encryption ensures only the intended recipient can decrypt the message.
                debug!("Broadcasting message (intended for: {}, size: {} bytes)", to, data.len());
                
                // Publish to gossipsub topic
                match self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(self.gossipsub_topic.clone(), data)
                {
                    Ok(message_id) => {
                        debug!("Published message to gossipsub, message_id: {:?}", message_id);
                    }
                    Err(e) => {
                        error!("Failed to publish to gossipsub: {}", e);
                        return Err(NetworkError::SendError(format!("Publish error: {}", e)));
                    }
                }
            }
            
            NetworkCommand::ListPeers { response } => {
                let peers: Vec<PeerId> = self.connected_peers.iter().copied().collect();
                let _ = response.send(peers).await;
            }
            
            NetworkCommand::DialPeer { peer_id, address } => {
                let addr: Multiaddr = match address.parse() {
                    Ok(addr) => addr,
                    Err(e) => {
                        let err = NetworkError::TransportError(format!("Invalid address: {}", e));
                        self.record_dial_failure(peer_id, None, err.category());
                        return Err(err);
                    }
                };

                if let Some(state) = self.peer_retry_state.get(&peer_id) {
                    if let Some(until) = state.blacklist_until {
                        if until > Instant::now() {
                            warn!("Dial blocked (blacklist) for {}", peer_id);
                            return Ok(());
                        }
                    }
                    if let Some(until) = state.circuit_open_until {
                        if until > Instant::now() {
                            warn!("Dial blocked (circuit open) for {}", peer_id);
                            return Ok(());
                        }
                    }
                }

                if let Err(e) = self.swarm.dial(addr.clone()) {
                    let err = NetworkError::TransportError(e.to_string());
                    self.record_dial_failure(peer_id, Some(addr), err.category());
                    return Err(err);
                }
            }

            NetworkCommand::QueryBootstrap { bootstrap_peer_id } => {
                // Mark as bootstrap peer and trigger DHT query
                self.mark_as_bootstrap(bootstrap_peer_id);
                
                // Start DHT query to discover mesh peers
                // Only start if not already started, but QueryBootstrap is explicit request
                if !self.bootstrap_query_started {
                    self.bootstrap_query_started = true;
                    let query_id = self.swarm.behaviour_mut().kad.get_closest_peers(PeerId::random());
                    info!("🔍 Explicit DHT query started through {}: {:?}", bootstrap_peer_id, query_id);
                }
            }
        }
        
        Ok(())
    }
}

/// Create network channels
pub fn create_network_channels() -> (
    mpsc::Sender<NetworkEvent>,
    mpsc::Receiver<NetworkEvent>,
    mpsc::Sender<NetworkCommand>,
    mpsc::Receiver<NetworkCommand>,
) {
    let (event_tx, event_rx) = mpsc::channel(100);
    let (command_tx, command_rx) = mpsc::channel(100);
    (event_tx, event_rx, command_tx, command_rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};
    
    #[tokio::test]
    async fn test_network_creation() {
        let (event_tx, _event_rx, _command_tx, command_rx) = create_network_channels();
        let network = Network::new(event_tx, command_rx);
        assert!(network.is_ok());
    }

    #[test]
    fn test_error_category() {
        let err = NetworkError::ListenError("fail".to_string());
        assert_eq!(err.category(), NetworkErrorCategory::Configuration);

        let err = NetworkError::TransportError("dial".to_string());
        assert_eq!(err.category(), NetworkErrorCategory::Transient);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_circuit_breaker_opens() {
        let (event_tx, _event_rx, _command_tx, command_rx) = create_network_channels();
        let mut network = Network::new(event_tx, command_rx).unwrap();
        let peer_id = PeerId::random();

        for _ in 0..5 {
            network.record_dial_failure(peer_id, None, NetworkErrorCategory::Transient);
        }

        let state = network.peer_retry_state.get(&peer_id).expect("retry state missing");
        assert!(state.circuit_open_until.is_some(), "Circuit should be open after failures");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_peer_online_event_emitted() {
        let (event_tx1, mut event_rx1, _command_tx1, command_rx1) = create_network_channels();
        let mut network1 = Network::new(event_tx1, command_rx1).unwrap();
        network1.listen("/ip4/127.0.0.1/tcp/0").unwrap();
        let peer_id1 = network1.local_peer_id;

        let handle1 = tokio::spawn(async move {
            let _ = network1.run().await;
        });

        let addr1 = timeout(Duration::from_secs(5), async {
            loop {
                if let Some(event) = event_rx1.recv().await {
                    if let NetworkEvent::ListeningOn { address } = event {
                        break address;
                    }
                }
            }
        }).await.expect("Timeout waiting for network1 listen");

        let (event_tx2, mut event_rx2, command_tx2, command_rx2) = create_network_channels();
        let mut network2 = Network::new(event_tx2, command_rx2).unwrap();
        network2.listen("/ip4/127.0.0.1/tcp/0").unwrap();
        let _peer_id2 = network2.local_peer_id;

        let handle2 = tokio::spawn(async move {
            let _ = network2.run().await;
        });

        let addr_with_peer = format!("{}/p2p/{}", addr1, peer_id1);
        command_tx2.send(NetworkCommand::DialPeer {
            peer_id: peer_id1,
            address: addr_with_peer,
        }).await.unwrap();

        let (mut saw_online, mut saw_ready) = (false, false);
        let result = timeout(Duration::from_secs(8), async {
            while let Some(event) = event_rx2.recv().await {
                match event {
                    NetworkEvent::PeerOnline { peer_id, .. } => {
                        if peer_id == peer_id1 {
                            saw_online = true;
                        }
                    }
                    NetworkEvent::NetworkReady { .. } => {
                        saw_ready = true;
                    }
                    _ => {}
                }

                if saw_online && saw_ready {
                    break;
                }
            }
        }).await;

        handle1.abort();
        handle2.abort();

        assert!(result.is_ok(), "Timeout waiting for PeerOnline/NetworkReady events");
        assert!(saw_online, "PeerOnline event not emitted");
        assert!(saw_ready, "NetworkReady event not emitted");
    }
}
