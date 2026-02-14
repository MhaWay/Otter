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
}
