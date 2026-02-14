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

pub mod webrtc;

use futures::{prelude::*, select};
use libp2p::{
    core::transport::upgrade,
    core::muxing::StreamMuxerBox,
    gossipsub, identify, kad,
    mdns,
    noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, PeerId, Swarm, Multiaddr, Transport,
};
use std::{
    collections::HashSet,
    time::Duration,
};
use thiserror::Error as ThisError;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use void::Void;

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

/// Events from the network layer
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// A new peer was discovered
    PeerDiscovered { peer_id: PeerId, addresses: Vec<String> },
    /// A peer connected
    PeerConnected { peer_id: PeerId },
    /// A peer disconnected
    PeerDisconnected { peer_id: PeerId },
    /// Received a message from a peer
    MessageReceived { from: PeerId, data: Vec<u8> },
    /// Network listening started
    ListeningOn { address: String },
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
}

/// Network behavior combining multiple protocols
#[derive(NetworkBehaviour)]
pub struct OtterBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
    kad: kad::Behaviour<kad::store::MemoryStore>,
    identify: identify::Behaviour,
}

/// The main network manager
pub struct Network {
    swarm: Swarm<OtterBehaviour>,
    event_tx: mpsc::Sender<NetworkEvent>,
    command_rx: mpsc::Receiver<NetworkCommand>,
    connected_peers: HashSet<PeerId>,
    gossipsub_topic: gossipsub::IdentTopic,
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
        
        // Create Kademlia DHT
        let store = kad::store::MemoryStore::new(local_peer_id);
        let kad = kad::Behaviour::new(local_peer_id, store);
        
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
        
        // Create swarm
        let swarm = Swarm::new(transport, behaviour, local_peer_id, libp2p::swarm::Config::with_tokio_executor());
        
        // Create gossipsub topic
        let gossipsub_topic = gossipsub::IdentTopic::new("otter-chat");
        
        Ok(Self {
            swarm,
            event_tx,
            command_rx,
            connected_peers: HashSet::new(),
            gossipsub_topic,
        })
    }
    
    /// Start listening on the given address
    pub fn listen(&mut self, addr: &str) -> Result<(), NetworkError> {
        let addr: Multiaddr = addr
            .parse()
            .map_err(|e| NetworkError::ListenError(format!("Invalid address: {}", e)))?;
        
        self.swarm
            .listen_on(addr)
            .map_err(|e| NetworkError::ListenError(e.to_string()))?;
        
        // Subscribe to gossipsub topic
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.gossipsub_topic)
            .map_err(|e| NetworkError::InitializationError(format!("Subscribe error: {}", e)))?;
        
        Ok(())
    }
    
    /// Run the network event loop
    pub async fn run(mut self) -> Result<(), NetworkError> {
        loop {
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
                debug!("Received message from {}", propagation_source);
                
                let _ = self.event_tx.send(NetworkEvent::MessageReceived {
                    from: propagation_source,
                    data: message.data,
                }).await;
            }
            
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected to peer: {}", peer_id);
                self.connected_peers.insert(peer_id);
                
                let _ = self.event_tx.send(NetworkEvent::PeerConnected { peer_id }).await;
            }
            
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Disconnected from peer: {}", peer_id);
                self.connected_peers.remove(&peer_id);
                
                let _ = self.event_tx.send(NetworkEvent::PeerDisconnected { peer_id }).await;
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
                debug!("Sending message to peer: {}", to);
                
                // Publish to gossipsub topic
                self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(self.gossipsub_topic.clone(), data)
                    .map_err(|e| NetworkError::SendError(format!("Publish error: {}", e)))?;
            }
            
            NetworkCommand::ListPeers { response } => {
                let peers: Vec<PeerId> = self.connected_peers.iter().copied().collect();
                let _ = response.send(peers).await;
            }
            
            NetworkCommand::DialPeer { peer_id: _, address } => {
                let addr: Multiaddr = address
                    .parse()
                    .map_err(|e| NetworkError::TransportError(format!("Invalid address: {}", e)))?;
                
                self.swarm
                    .dial(addr)
                    .map_err(|e| NetworkError::TransportError(e.to_string()))?;
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
    
    #[tokio::test]
    async fn test_network_creation() {
        let (event_tx, _event_rx, _command_tx, command_rx) = create_network_channels();
        let network = Network::new(event_tx, command_rx);
        assert!(network.is_ok());
    }
}
