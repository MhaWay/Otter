//! # Otter Messaging
//!
//! High-level messaging layer for the Otter decentralized chat platform.
//!
//! This crate provides:
//! - Message types and protocols
//! - Message encryption/decryption integration
//! - Message routing and handling
//! - Conversation management

use chrono::{DateTime, Utc};
use libp2p::PeerId;
use otter_crypto::{CryptoSession, EncryptedMessage, MessageCrypto};
use otter_identity::{Identity, PublicIdentity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum MessagingError {
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    #[error("Decryption error: {0}")]
    DecryptionError(String),
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
    #[error("Invalid message format: {0}")]
    InvalidFormat(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Message types in the Otter protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Plain text message
    Text {
        content: String,
        timestamp: DateTime<Utc>,
    },
    
    /// Identity announcement (public key exchange)
    Identity {
        public_identity: PublicIdentity,
        timestamp: DateTime<Utc>,
    },
    
    /// Encrypted message envelope
    Encrypted {
        from_peer_id: String,
        encrypted: EncryptedMessage,
        timestamp: DateTime<Utc>,
    },
    
    /// Peer status update
    Status {
        status: String,
        timestamp: DateTime<Utc>,
    },
    
    /// Typing indicator
    Typing {
        is_typing: bool,
    },
}

impl Message {
    /// Create a new text message
    pub fn text(content: String) -> Self {
        Self::Text {
            content,
            timestamp: Utc::now(),
        }
    }
    
    /// Create an identity announcement
    pub fn identity(public_identity: PublicIdentity) -> Self {
        Self::Identity {
            public_identity,
            timestamp: Utc::now(),
        }
    }
    
    /// Create an encrypted message
    pub fn encrypted(from_peer_id: String, encrypted: EncryptedMessage) -> Self {
        Self::Encrypted {
            from_peer_id,
            encrypted,
            timestamp: Utc::now(),
        }
    }
    
    /// Serialize message to JSON
    pub fn to_json(&self) -> Result<String, MessagingError> {
        serde_json::to_string(self)
            .map_err(|e| MessagingError::SerializationError(e.to_string()))
    }
    
    /// Deserialize message from JSON
    pub fn from_json(json: &str) -> Result<Self, MessagingError> {
        serde_json::from_str(json)
            .map_err(|e| MessagingError::SerializationError(e.to_string()))
    }
    
    /// Serialize message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, MessagingError> {
        bincode::serialize(self)
            .map_err(|e| MessagingError::SerializationError(e.to_string()))
    }
    
    /// Deserialize message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, MessagingError> {
        bincode::deserialize(bytes)
            .map_err(|e| MessagingError::SerializationError(e.to_string()))
    }
}

/// Manages conversations and encryption sessions with peers
pub struct MessageHandler {
    local_identity: Identity,
    peers: HashMap<String, PublicIdentity>,
    sessions: HashMap<String, CryptoSession>,
}

impl MessageHandler {
    /// Create a new message handler
    pub fn new(local_identity: Identity) -> Self {
        Self {
            local_identity,
            peers: HashMap::new(),
            sessions: HashMap::new(),
        }
    }
    
    /// Register a peer's public identity
    pub fn register_peer(&mut self, public_identity: PublicIdentity) -> Result<(), MessagingError> {
        let peer_id = public_identity.peer_id().to_string();
        
        // Create crypto session with this peer
        let session = CryptoSession::new(&self.local_identity, &public_identity)
            .map_err(|e| MessagingError::EncryptionError(e.to_string()))?;
        
        info!("Registered peer {} with session fingerprint: {}", peer_id, session.fingerprint());
        
        self.peers.insert(peer_id.clone(), public_identity);
        self.sessions.insert(peer_id, session);
        
        Ok(())
    }
    
    /// Get local public identity for sharing
    pub fn public_identity(&self) -> PublicIdentity {
        PublicIdentity::from_identity(&self.local_identity)
    }
    
    /// Encrypt and prepare a text message for a specific peer
    pub fn prepare_encrypted_message(
        &mut self,
        peer_id: &str,
        text: &str,
    ) -> Result<Message, MessagingError> {
        let session = self
            .sessions
            .get_mut(peer_id)
            .ok_or_else(|| MessagingError::PeerNotFound(peer_id.to_string()))?;
        
        let encrypted = MessageCrypto::encrypt_text(session, text)
            .map_err(|e| MessagingError::EncryptionError(e.to_string()))?;
        
        Ok(Message::encrypted(
            self.local_identity.peer_id().to_string(),
            encrypted,
        ))
    }
    
    /// Decrypt a received encrypted message
    pub fn decrypt_message(&mut self, message: &Message) -> Result<String, MessagingError> {
        match message {
            Message::Encrypted {
                from_peer_id,
                encrypted,
                ..
            } => {
                let session = self
                    .sessions
                    .get_mut(from_peer_id)
                    .ok_or_else(|| MessagingError::PeerNotFound(from_peer_id.to_string()))?;
                
                MessageCrypto::decrypt_text(session, encrypted)
                    .map_err(|e| MessagingError::DecryptionError(e.to_string()))
            }
            Message::Text { content, .. } => Ok(content.clone()),
            _ => Err(MessagingError::InvalidFormat(
                "Not an encrypted or text message".to_string(),
            )),
        }
    }
    
    /// Get list of registered peers
    pub fn list_peers(&self) -> Vec<String> {
        self.peers.keys().cloned().collect()
    }
    
    /// Check if a peer is registered
    pub fn has_peer(&self, peer_id: &str) -> bool {
        self.peers.contains_key(peer_id)
    }
}

/// High-level messaging events
#[derive(Debug, Clone)]
pub enum MessagingEvent {
    /// Received a text message
    TextMessage {
        from: String,
        content: String,
        timestamp: DateTime<Utc>,
    },
    
    /// Peer identity received
    PeerIdentity {
        peer_id: String,
        public_identity: PublicIdentity,
    },
    
    /// Peer status update
    PeerStatus {
        peer_id: String,
        status: String,
    },
    
    /// Peer is typing
    PeerTyping {
        peer_id: String,
        is_typing: bool,
    },
}

/// Commands for the messaging layer
#[derive(Debug)]
pub enum MessagingCommand {
    /// Send a text message to a peer
    SendText {
        to: String,
        content: String,
    },
    
    /// Announce identity to network
    AnnounceIdentity,
    
    /// Update status
    UpdateStatus {
        status: String,
    },
    
    /// List registered peers
    ListPeers {
        response: mpsc::Sender<Vec<String>>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_serialization() {
        let msg = Message::text("Hello, Otter!".to_string());
        let json = msg.to_json().unwrap();
        let deserialized = Message::from_json(&json).unwrap();
        
        if let Message::Text { content, .. } = deserialized {
            assert_eq!(content, "Hello, Otter!");
        } else {
            panic!("Wrong message type");
        }
    }
    
    #[test]
    fn test_message_handler() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();
        
        let mut alice_handler = MessageHandler::new(alice);
        let bob_public = PublicIdentity::from_identity(&bob);
        
        alice_handler.register_peer(bob_public).unwrap();
        assert!(alice_handler.has_peer(&bob.peer_id().to_string()));
    }
    
    #[test]
    fn test_encrypted_messaging() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();
        
        let alice_public = PublicIdentity::from_identity(&alice);
        let bob_public = PublicIdentity::from_identity(&bob);
        
        let mut alice_handler = MessageHandler::new(alice);
        let mut bob_handler = MessageHandler::new(bob);
        
        alice_handler.register_peer(bob_public.clone()).unwrap();
        bob_handler.register_peer(alice_public).unwrap();
        
        // Alice sends encrypted message to Bob
        let text = "Secret message from Alice";
        let encrypted_msg = alice_handler
            .prepare_encrypted_message(bob_public.peer_id().as_str(), text)
            .unwrap();
        
        // Bob decrypts the message
        let decrypted = bob_handler.decrypt_message(&encrypted_msg).unwrap();
        assert_eq!(decrypted, text);
    }
}
