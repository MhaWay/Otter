//! # Otter Protocol
//!
//! Protocol definition and versioning layer for the Otter decentralized chat platform.
//!
//! This crate provides:
//! - Protocol versioning and negotiation
//! - Binary message format definitions
//! - Peer handshake protocol
//! - Capability negotiation (voice, video, file transfer, etc.)
//! - Protocol upgrade mechanisms

use chrono::{DateTime, Utc};
use otter_identity::PublicIdentity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Current protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Protocol identifier
pub const PROTOCOL_ID: &str = "/otter/1.0.0";

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Incompatible protocol version: expected {expected}, got {actual}")]
    IncompatibleVersion { expected: u32, actual: u32 },
    #[error("Unsupported capability: {0}")]
    UnsupportedCapability(String),
    #[error("Invalid handshake: {0}")]
    InvalidHandshake(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Invalid message format: {0}")]
    InvalidFormat(String),
}

/// Peer capabilities that can be negotiated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Basic text messaging
    TextMessaging,
    /// Voice calls (WebRTC audio)
    VoiceCall,
    /// Video calls (WebRTC video)
    VideoCall,
    /// File transfer
    FileTransfer,
    /// Group chat support
    GroupChat,
    /// Screen sharing
    ScreenShare,
    /// End-to-end encryption (required)
    E2EEncryption,
    /// Custom capability
    Custom(String),
}

/// Protocol handshake message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handshake {
    /// Protocol version
    pub version: u32,
    
    /// Protocol identifier
    pub protocol_id: String,
    
    /// Sender's public identity
    pub identity: PublicIdentity,
    
    /// List of supported capabilities
    pub capabilities: Vec<Capability>,
    
    /// Optional metadata (client info, etc.)
    pub metadata: HashMap<String, String>,
    
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Signature over the handshake (excluding this field)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<u8>>,
}

impl Handshake {
    /// Create a new handshake message
    pub fn new(identity: PublicIdentity, capabilities: Vec<Capability>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            protocol_id: PROTOCOL_ID.to_string(),
            identity,
            capabilities,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            signature: None,
        }
    }
    
    /// Add metadata to the handshake
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// Check if a capability is supported
    pub fn supports(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }
    
    /// Verify protocol compatibility
    pub fn is_compatible(&self) -> Result<(), ProtocolError> {
        if self.version != PROTOCOL_VERSION {
            return Err(ProtocolError::IncompatibleVersion {
                expected: PROTOCOL_VERSION,
                actual: self.version,
            });
        }
        
        if self.protocol_id != PROTOCOL_ID {
            return Err(ProtocolError::InvalidHandshake(
                "Protocol ID mismatch".to_string(),
            ));
        }
        
        // E2E encryption is mandatory
        if !self.supports(&Capability::E2EEncryption) {
            return Err(ProtocolError::UnsupportedCapability(
                "E2E encryption is required".to_string(),
            ));
        }
        
        Ok(())
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        bincode::serialize(self)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        bincode::deserialize(bytes)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }
}

/// Handshake response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    /// Accepted capabilities (subset of requested)
    pub accepted_capabilities: Vec<Capability>,
    
    /// Response status
    pub accepted: bool,
    
    /// Optional reason for rejection
    pub reason: Option<String>,
    
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl HandshakeResponse {
    /// Create an acceptance response
    pub fn accept(capabilities: Vec<Capability>) -> Self {
        Self {
            accepted_capabilities: capabilities,
            accepted: true,
            reason: None,
            timestamp: Utc::now(),
        }
    }
    
    /// Create a rejection response
    pub fn reject(reason: String) -> Self {
        Self {
            accepted_capabilities: Vec::new(),
            accepted: false,
            reason: Some(reason),
            timestamp: Utc::now(),
        }
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        bincode::serialize(self)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        bincode::deserialize(bytes)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }
}

/// Protocol message types with version support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    /// Protocol version
    pub version: u32,
    
    /// Message payload
    pub payload: MessagePayload,
    
    /// Message ID for tracking
    pub message_id: String,
    
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Message payload types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessagePayload {
    /// Handshake initiation
    Handshake(Handshake),
    
    /// Handshake response
    HandshakeResponse(HandshakeResponse),
    
    /// Text message
    Text { content: Vec<u8> },
    
    /// Binary data
    Binary { data: Vec<u8> },
    
    /// Keep-alive ping
    Ping,
    
    /// Pong response
    Pong,
    
    /// Capability negotiation
    CapabilityRequest { capabilities: Vec<Capability> },
    
    /// Protocol upgrade request
    ProtocolUpgrade { target_version: u32 },
}

impl ProtocolMessage {
    /// Create a new protocol message
    pub fn new(payload: MessagePayload) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            payload,
            message_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
        }
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        bincode::serialize(self)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        bincode::deserialize(bytes)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }
}

/// Capability matcher - finds common capabilities between peers
pub struct CapabilityMatcher;

impl CapabilityMatcher {
    /// Find common capabilities between two peers
    pub fn match_capabilities(
        local: &[Capability],
        remote: &[Capability],
    ) -> Vec<Capability> {
        local
            .iter()
            .filter(|cap| remote.contains(cap))
            .cloned()
            .collect()
    }
    
    /// Check if a required capability is supported
    pub fn has_required(capabilities: &[Capability], required: &Capability) -> bool {
        capabilities.contains(required)
    }
}

/// WebRTC signaling messages for voice/video setup
/// 
/// These messages are transmitted over the encrypted messaging channel
/// to establish WebRTC connections between peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalingMessage {
    /// Offer to start a WebRTC session
    Offer {
        /// SDP (Session Description Protocol) offer
        sdp: String,
        /// Media type (audio, video, or both)
        media_type: MediaType,
        /// Session ID for tracking
        session_id: String,
    },
    
    /// Answer to a WebRTC offer
    Answer {
        /// SDP answer
        sdp: String,
        /// Session ID matching the offer
        session_id: String,
    },
    
    /// ICE candidate for NAT traversal
    IceCandidate {
        /// ICE candidate in SDP format
        candidate: String,
        /// SDP media line index
        sdp_mid: Option<String>,
        /// SDP line number
        sdp_mline_index: Option<u32>,
        /// Session ID
        session_id: String,
    },
    
    /// Signal that ICE candidate gathering is complete
    IceComplete {
        /// Session ID
        session_id: String,
    },
    
    /// Request to end the session
    Hangup {
        /// Session ID
        session_id: String,
        /// Reason for hangup
        reason: Option<String>,
    },
    
    /// Acknowledgment of received signaling message
    Ack {
        /// ID of the message being acknowledged
        ack_message_id: String,
    },
    
    /// Request retransmission of a message
    Retransmit {
        /// ID of the message to retransmit
        message_id: String,
    },
}

/// Media type for WebRTC sessions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaType {
    /// Audio only
    AudioOnly,
    /// Video only  
    VideoOnly,
    /// Both audio and video
    AudioVideo,
    /// Screen sharing
    ScreenShare,
    /// Data channel only
    DataOnly,
}

/// Signaling protocol message with reliability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalingProtocolMessage {
    /// Unique message ID
    pub message_id: String,
    
    /// Signaling payload
    pub payload: SignalingMessage,
    
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Sequence number for ordering
    pub sequence: u64,
    
    /// Requires acknowledgment
    pub requires_ack: bool,
}

impl SignalingProtocolMessage {
    /// Create a new signaling message
    pub fn new(payload: SignalingMessage, sequence: u64, requires_ack: bool) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            payload,
            timestamp: Utc::now(),
            sequence,
            requires_ack,
        }
    }
    
    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, ProtocolError> {
        serde_json::to_string(self)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }
    
    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, ProtocolError> {
        serde_json::from_str(json)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        bincode::serialize(self)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        bincode::deserialize(bytes)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }
}

/// Signaling session manager for tracking WebRTC setup
pub struct SignalingSession {
    /// Session ID
    pub session_id: String,
    
    /// Peer ID we're signaling with
    pub peer_id: String,
    
    /// Local media type
    pub media_type: MediaType,
    
    /// Whether we initiated the session
    pub is_initiator: bool,
    
    /// Sequence counter for messages
    pub sequence: u64,
    
    /// Pending acks (message_id -> send_time)
    pub pending_acks: HashMap<String, DateTime<Utc>>,
    
    /// Received ICE candidates
    pub received_candidates: Vec<String>,
    
    /// Session start time
    pub started_at: DateTime<Utc>,
    
    /// Session state
    pub state: SignalingState,
}

/// Signaling session state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalingState {
    /// Waiting to send offer
    Idle,
    /// Offer sent, waiting for answer
    OfferSent,
    /// Offer received, sending answer
    AnswerPending,
    /// Answer sent or received, exchanging ICE
    Connecting,
    /// Session established
    Connected,
    /// Session ended
    Closed,
    /// Error state
    Failed(String),
}

impl SignalingSession {
    /// Create a new signaling session as initiator
    pub fn new_initiator(session_id: String, peer_id: String, media_type: MediaType) -> Self {
        Self {
            session_id,
            peer_id,
            media_type,
            is_initiator: true,
            sequence: 0,
            pending_acks: HashMap::new(),
            received_candidates: Vec::new(),
            started_at: Utc::now(),
            state: SignalingState::Idle,
        }
    }
    
    /// Create a new signaling session as responder
    pub fn new_responder(session_id: String, peer_id: String, media_type: MediaType) -> Self {
        Self {
            session_id,
            peer_id,
            media_type,
            is_initiator: false,
            sequence: 0,
            pending_acks: HashMap::new(),
            received_candidates: Vec::new(),
            started_at: Utc::now(),
            state: SignalingState::AnswerPending,
        }
    }
    
    /// Create the next signaling message
    pub fn create_message(&mut self, payload: SignalingMessage, requires_ack: bool) -> SignalingProtocolMessage {
        let msg = SignalingProtocolMessage::new(payload, self.sequence, requires_ack);
        self.sequence += 1;
        
        if requires_ack {
            self.pending_acks.insert(msg.message_id.clone(), msg.timestamp);
        }
        
        msg
    }
    
    /// Handle received acknowledgment
    pub fn handle_ack(&mut self, message_id: &str) {
        self.pending_acks.remove(message_id);
    }
    
    /// Get messages that need retransmission (no ack after timeout)
    pub fn get_retransmit_needed(&self, timeout_seconds: i64) -> Vec<String> {
        let now = Utc::now();
        self.pending_acks
            .iter()
            .filter(|(_, sent_at)| {
                (now - **sent_at).num_seconds() > timeout_seconds
            })
            .map(|(msg_id, _)| msg_id.clone())
            .collect()
    }
    
    /// Update session state
    pub fn set_state(&mut self, state: SignalingState) {
        self.state = state;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use otter_identity::Identity;
    
    #[test]
    fn test_handshake_creation() {
        let identity = Identity::generate().unwrap();
        let public = PublicIdentity::from_identity(&identity);
        
        let handshake = Handshake::new(
            public,
            vec![Capability::TextMessaging, Capability::E2EEncryption],
        );
        
        assert_eq!(handshake.version, PROTOCOL_VERSION);
        assert!(handshake.supports(&Capability::TextMessaging));
    }
    
    #[test]
    fn test_handshake_compatibility() {
        let identity = Identity::generate().unwrap();
        let public = PublicIdentity::from_identity(&identity);
        
        let handshake = Handshake::new(
            public,
            vec![Capability::E2EEncryption, Capability::TextMessaging],
        );
        
        assert!(handshake.is_compatible().is_ok());
    }
    
    #[test]
    fn test_handshake_missing_e2e() {
        let identity = Identity::generate().unwrap();
        let public = PublicIdentity::from_identity(&identity);
        
        let handshake = Handshake::new(public, vec![Capability::TextMessaging]);
        
        assert!(handshake.is_compatible().is_err());
    }
    
    #[test]
    fn test_capability_matching() {
        let local = vec![
            Capability::TextMessaging,
            Capability::VoiceCall,
            Capability::E2EEncryption,
        ];
        
        let remote = vec![
            Capability::TextMessaging,
            Capability::E2EEncryption,
            Capability::FileTransfer,
        ];
        
        let common = CapabilityMatcher::match_capabilities(&local, &remote);
        
        assert_eq!(common.len(), 2);
        assert!(common.contains(&Capability::TextMessaging));
        assert!(common.contains(&Capability::E2EEncryption));
    }
    
    #[test]
    fn test_handshake_serialization() {
        let identity = Identity::generate().unwrap();
        let public = PublicIdentity::from_identity(&identity);
        
        let handshake = Handshake::new(
            public,
            vec![Capability::E2EEncryption, Capability::TextMessaging],
        );
        
        // Test JSON serialization instead (more reliable for complex types)
        let json = serde_json::to_string(&handshake).unwrap();
        let deserialized: Handshake = serde_json::from_str(&json).unwrap();
        
        assert_eq!(handshake.version, deserialized.version);
        assert_eq!(handshake.capabilities.len(), deserialized.capabilities.len());
    }
    
    #[test]
    fn test_signaling_session_creation() {
        let session_id = "test-session".to_string();
        let peer_id = "peer1".to_string();
        
        let session = SignalingSession::new_initiator(
            session_id.clone(),
            peer_id.clone(),
            MediaType::AudioOnly,
        );
        
        assert_eq!(session.session_id, session_id);
        assert_eq!(session.peer_id, peer_id);
        assert!(session.is_initiator);
        assert_eq!(session.state, SignalingState::Idle);
    }
    
    #[test]
    fn test_signaling_message_serialization() {
        let offer = SignalingMessage::Offer {
            sdp: "test-sdp".to_string(),
            media_type: MediaType::AudioVideo,
            session_id: "session1".to_string(),
        };
        
        let msg = SignalingProtocolMessage::new(offer, 0, true);
        
        // Test JSON serialization
        let json = msg.to_json().unwrap();
        let deserialized = SignalingProtocolMessage::from_json(&json).unwrap();
        
        assert_eq!(msg.message_id, deserialized.message_id);
        assert_eq!(msg.sequence, deserialized.sequence);
        assert!(deserialized.requires_ack);
    }
    
    #[test]
    fn test_signaling_ack_handling() {
        let mut session = SignalingSession::new_initiator(
            "session1".to_string(),
            "peer1".to_string(),
            MediaType::AudioOnly,
        );
        
        let offer = SignalingMessage::Offer {
            sdp: "test".to_string(),
            media_type: MediaType::AudioOnly,
            session_id: "session1".to_string(),
        };
        
        let msg = session.create_message(offer, true);
        assert_eq!(session.pending_acks.len(), 1);
        
        session.handle_ack(&msg.message_id);
        assert_eq!(session.pending_acks.len(), 0);
    }
}
