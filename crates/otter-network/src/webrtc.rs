//! # WebRTC Transport
//!
//! WebRTC transport layer with ICE negotiation for NAT traversal.
//!
//! This module provides:
//! - ICE candidate negotiation
//! - STUN/TURN support for NAT traversal
//! - WebRTC data channel support
//! - Fallback relay mechanisms

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WebRTCError {
    #[error("ICE negotiation failed: {0}")]
    IceNegotiationFailed(String),
    #[error("Connection timeout")]
    ConnectionTimeout,
    #[error("Invalid candidate: {0}")]
    InvalidCandidate(String),
    #[error("STUN server error: {0}")]
    StunError(String),
    #[error("TURN server error: {0}")]
    TurnError(String),
}

/// ICE candidate types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CandidateType {
    /// Host candidate (local address)
    Host,
    /// Server reflexive (STUN-discovered address)
    ServerReflexive,
    /// Peer reflexive (learned from peer)
    PeerReflexive,
    /// Relay candidate (via TURN server)
    Relay,
}

/// ICE candidate for NAT traversal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    /// Candidate type
    pub candidate_type: CandidateType,
    
    /// Transport protocol (UDP or TCP)
    pub protocol: TransportProtocol,
    
    /// IP address and port
    pub address: String,
    pub port: u16,
    
    /// Priority for this candidate
    pub priority: u32,
    
    /// Foundation (identifier for related candidates)
    pub foundation: String,
    
    /// Component ID (1 for RTP, 2 for RTCP)
    pub component: u16,
    
    /// Related address (for reflexive/relay candidates)
    pub related_address: Option<String>,
    pub related_port: Option<u16>,
}

impl IceCandidate {
    /// Create a host candidate
    pub fn host(address: String, port: u16, protocol: TransportProtocol) -> Self {
        Self {
            candidate_type: CandidateType::Host,
            protocol,
            address,
            port,
            priority: Self::calculate_priority(CandidateType::Host, 1),
            foundation: format!("host-{}", port),
            component: 1,
            related_address: None,
            related_port: None,
        }
    }
    
    /// Create a server reflexive candidate (from STUN)
    pub fn server_reflexive(
        address: String,
        port: u16,
        protocol: TransportProtocol,
        local_address: String,
        local_port: u16,
    ) -> Self {
        Self {
            candidate_type: CandidateType::ServerReflexive,
            protocol,
            address,
            port,
            priority: Self::calculate_priority(CandidateType::ServerReflexive, 1),
            foundation: format!("srflx-{}", port),
            component: 1,
            related_address: Some(local_address),
            related_port: Some(local_port),
        }
    }
    
    /// Create a relay candidate (from TURN)
    pub fn relay(
        address: String,
        port: u16,
        protocol: TransportProtocol,
        relay_address: String,
        relay_port: u16,
    ) -> Self {
        Self {
            candidate_type: CandidateType::Relay,
            protocol,
            address,
            port,
            priority: Self::calculate_priority(CandidateType::Relay, 1),
            foundation: format!("relay-{}", port),
            component: 1,
            related_address: Some(relay_address),
            related_port: Some(relay_port),
        }
    }
    
    /// Calculate ICE priority (RFC 5245)
    fn calculate_priority(candidate_type: CandidateType, component: u16) -> u32 {
        let type_preference = match candidate_type {
            CandidateType::Host => 126,
            CandidateType::PeerReflexive => 110,
            CandidateType::ServerReflexive => 100,
            CandidateType::Relay => 0,
        };
        
        // Priority = (2^24)*(type preference) + (2^8)*(local preference) + (2^0)*(256 - component)
        (type_preference << 24) | (65535 << 8) | ((256 - component as u32) & 0xFF)
    }
    
    /// Serialize to SDP format
    pub fn to_sdp(&self) -> String {
        let mut sdp = format!(
            "candidate:{} {} {} {} {} {}",
            self.foundation,
            self.component,
            match self.protocol {
                TransportProtocol::Udp => "udp",
                TransportProtocol::Tcp => "tcp",
            },
            self.priority,
            self.address,
            self.port
        );
        
        sdp.push_str(&format!(
            " typ {}",
            match self.candidate_type {
                CandidateType::Host => "host",
                CandidateType::ServerReflexive => "srflx",
                CandidateType::PeerReflexive => "prflx",
                CandidateType::Relay => "relay",
            }
        ));
        
        if let (Some(addr), Some(port)) = (&self.related_address, self.related_port) {
            sdp.push_str(&format!(" raddr {} rport {}", addr, port));
        }
        
        sdp
    }
}

/// Transport protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransportProtocol {
    Udp,
    Tcp,
}

/// ICE configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceConfig {
    /// STUN servers for discovering public IP
    pub stun_servers: Vec<String>,
    
    /// TURN servers for relay (with credentials)
    pub turn_servers: Vec<TurnServer>,
    
    /// Gather candidates locally
    pub gather_host_candidates: bool,
    
    /// Use IPv6 candidates
    pub enable_ipv6: bool,
    
    /// ICE timeout in seconds
    pub timeout_seconds: u32,
}

impl Default for IceConfig {
    fn default() -> Self {
        Self {
            stun_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            turn_servers: Vec::new(),
            gather_host_candidates: true,
            enable_ipv6: false,
            timeout_seconds: 30,
        }
    }
}

/// TURN server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnServer {
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
}

/// ICE session state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IceState {
    /// Gathering candidates
    Gathering,
    /// Candidates gathered, ready to exchange
    GatheringComplete,
    /// Checking candidate pairs
    Checking,
    /// Connection established
    Connected,
    /// Connection completed (all components connected)
    Completed,
    /// Connection failed
    Failed,
    /// Connection closed
    Closed,
}

/// ICE negotiation manager
pub struct IceNegotiator {
    config: IceConfig,
    local_candidates: Vec<IceCandidate>,
    remote_candidates: Vec<IceCandidate>,
    state: IceState,
}

impl IceNegotiator {
    /// Create a new ICE negotiator
    pub fn new(config: IceConfig) -> Self {
        Self {
            config,
            local_candidates: Vec::new(),
            remote_candidates: Vec::new(),
            state: IceState::Gathering,
        }
    }
    
    /// Gather local candidates
    pub fn gather_candidates(&mut self) -> Result<Vec<IceCandidate>, WebRTCError> {
        let mut candidates = Vec::new();
        
        // Gather host candidates (local interfaces)
        if self.config.gather_host_candidates {
            // In a real implementation, this would enumerate network interfaces
            // For now, add localhost as an example
            candidates.push(IceCandidate::host(
                "127.0.0.1".to_string(),
                0, // OS assigns port
                TransportProtocol::Udp,
            ));
        }
        
        // TODO: Query STUN servers for server reflexive candidates
        // TODO: Request TURN allocation for relay candidates
        
        self.local_candidates = candidates.clone();
        self.state = IceState::GatheringComplete;
        
        Ok(candidates)
    }
    
    /// Add remote candidate
    pub fn add_remote_candidate(&mut self, candidate: IceCandidate) {
        self.remote_candidates.push(candidate);
    }
    
    /// Get current ICE state
    pub fn state(&self) -> &IceState {
        &self.state
    }
    
    /// Get local candidates
    pub fn local_candidates(&self) -> &[IceCandidate] {
        &self.local_candidates
    }
    
    /// Get remote candidates
    pub fn remote_candidates(&self) -> &[IceCandidate] {
        &self.remote_candidates
    }
}

/// WebRTC transport configuration
#[derive(Debug, Clone)]
pub struct WebRTCTransportConfig {
    pub ice_config: IceConfig,
    pub use_data_channels: bool,
    pub enable_audio: bool,
    pub enable_video: bool,
}

impl Default for WebRTCTransportConfig {
    fn default() -> Self {
        Self {
            ice_config: IceConfig::default(),
            use_data_channels: true,
            enable_audio: false,
            enable_video: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ice_candidate_creation() {
        let candidate = IceCandidate::host(
            "192.168.1.100".to_string(),
            8080,
            TransportProtocol::Udp,
        );
        
        assert_eq!(candidate.candidate_type, CandidateType::Host);
        assert_eq!(candidate.address, "192.168.1.100");
        assert_eq!(candidate.port, 8080);
    }
    
    #[test]
    fn test_ice_priority_calculation() {
        let host = IceCandidate::host("127.0.0.1".to_string(), 8080, TransportProtocol::Udp);
        let relay = IceCandidate::relay(
            "relay.example.com".to_string(),
            3478,
            TransportProtocol::Udp,
            "127.0.0.1".to_string(),
            8080,
        );
        
        // Host candidates should have higher priority than relay
        assert!(host.priority > relay.priority);
    }
    
    #[test]
    fn test_ice_negotiator() {
        let config = IceConfig::default();
        let mut negotiator = IceNegotiator::new(config);
        
        assert_eq!(negotiator.state(), &IceState::Gathering);
        
        let candidates = negotiator.gather_candidates().unwrap();
        assert!(!candidates.is_empty());
        assert_eq!(negotiator.state(), &IceState::GatheringComplete);
    }
    
    #[test]
    fn test_candidate_sdp_format() {
        let candidate = IceCandidate::host(
            "192.168.1.100".to_string(),
            8080,
            TransportProtocol::Udp,
        );
        
        let sdp = candidate.to_sdp();
        assert!(sdp.contains("192.168.1.100"));
        assert!(sdp.contains("8080"));
        assert!(sdp.contains("typ host"));
    }
}
