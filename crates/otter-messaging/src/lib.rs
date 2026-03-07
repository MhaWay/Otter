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
use otter_crypto::{CryptoSession, EncryptedMessage, MessageCrypto, PFSSession};
use otter_identity::{Identity, PublicIdentity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};

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
    /// Optionally carries an ephemeral X25519 public key for PFS handshake
    Identity {
        public_identity: PublicIdentity,
        ephemeral_public: Option<Vec<u8>>,
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
    Typing { is_typing: bool },
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
    /// Create an identity announcement, optionally carrying an ephemeral public key
    pub fn identity(public_identity: PublicIdentity, ephemeral_public: Option<Vec<u8>>) -> Self {
        Self::Identity {
            public_identity,
            ephemeral_public,
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
        serde_json::to_string(self).map_err(|e| MessagingError::SerializationError(e.to_string()))
    }

    /// Deserialize message from JSON
    pub fn from_json(json: &str) -> Result<Self, MessagingError> {
        serde_json::from_str(json).map_err(|e| MessagingError::SerializationError(e.to_string()))
    }

    /// Serialize message to MessagePack bytes
    ///
    /// Uses MessagePack with named struct serialization (maps, not arrays)
    /// to ensure compatibility with complex nested structures.
    pub fn to_bytes(&self) -> Result<Vec<u8>, MessagingError> {
        let mut buf = Vec::new();
        let mut serializer = rmp_serde::Serializer::new(&mut buf).with_struct_map();
        serde::Serialize::serialize(self, &mut serializer).map_err(|e| {
            MessagingError::SerializationError(format!("MessagePack encode error: {}", e))
        })?;
        Ok(buf)
    }

    /// Deserialize message from MessagePack bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, MessagingError> {
        rmp_serde::from_slice(bytes).map_err(|e| {
            MessagingError::SerializationError(format!("MessagePack decode error: {}", e))
        })
    }
}

/// Manages conversations and encryption sessions with peers
pub struct MessageHandler {
    local_identity: Identity,
    peers: HashMap<String, PublicIdentity>,
    /// Static sessions derived from identity keys
    sessions: HashMap<String, CryptoSession>,
    /// PFS-enabled sessions established when both peers exchange ephemeral keys
    pfs_sessions: HashMap<String, PFSSession>,
    /// Local ephemeral secret for PFS handshakes (per run)
    local_ephemeral: Option<EphemeralSecret>,
    /// Local ephemeral public key
    local_ephemeral_public: X25519PublicKey,
    /// Map from otter identity peer_id (string) to libp2p PeerId (network)
    peer_network_map: HashMap<String, PeerId>,
}

impl MessageHandler {
    /// Create a new message handler
    pub fn new(local_identity: Identity) -> Self {
        let local_ephemeral = PFSSession::generate_ephemeral();
        let local_ephemeral_public = X25519PublicKey::from(&local_ephemeral);

        Self {
            local_identity,
            peers: HashMap::new(),
            sessions: HashMap::new(),
            pfs_sessions: HashMap::new(),
            local_ephemeral: Some(local_ephemeral),
            local_ephemeral_public,
            peer_network_map: HashMap::new(),
        }
    }

    /// Register a peer's public identity
    /// Register a peer's public identity. If the peer included an ephemeral public key
    /// in their identity announcement, complete a PFS handshake and prefer PFS for messaging.
    pub fn register_peer(
        &mut self,
        public_identity: PublicIdentity,
        remote_ephemeral: Option<Vec<u8>>,
        network_peer: Option<PeerId>,
    ) -> Result<(), MessagingError> {
        let peer_id = public_identity.peer_id().to_string();

        // Create static fallback session
        let static_session = CryptoSession::new(&self.local_identity, &public_identity)
            .map_err(|e| MessagingError::EncryptionError(e.to_string()))?;

        info!(
            "Registered peer {} (static fingerprint: {})",
            peer_id,
            static_session.fingerprint()
        );

        self.peers.insert(peer_id.clone(), public_identity.clone());
        self.sessions.insert(peer_id.clone(), static_session);

        // Record mapping between identity peer_id and libp2p PeerId if provided
        if let Some(net) = network_peer {
            self.peer_network_map.insert(peer_id.clone(), net);
        }

        // If remote provided ephemeral, attempt to build PFS session
        if let Some(epub) = remote_ephemeral {
            if epub.len() == 32 {
                let mut epub_arr = [0u8; 32];
                epub_arr.copy_from_slice(&epub);
                let remote_ephemeral_pub = X25519PublicKey::from(epub_arr);

                // Determine initiator based on lexicographic ordering of peer IDs to avoid role ambiguity
                let local_id = self.local_identity.peer_id().to_string();
                let is_initiator = local_id < peer_id;

                if let Some(local_ephemeral) = self.local_ephemeral.take() {
                    let local_ephemeral_public = self.local_ephemeral_public;
                    let ephemeral_secret = local_ephemeral.diffie_hellman(&remote_ephemeral_pub);

                    match PFSSession::new_precomputed(
                        &self.local_identity,
                        &public_identity,
                        local_ephemeral_public,
                        ephemeral_secret,
                        is_initiator,
                    ) {
                        Ok(pfs) => {
                            info!(
                                "PFS established with {} (fingerprint: {})",
                                peer_id,
                                pfs.fingerprint()
                            );
                            self.pfs_sessions.insert(peer_id, pfs);
                        }
                        Err(e) => {
                            warn!("Failed to establish PFS with {}: {}", peer_id, e);
                        }
                    }
                } else {
                    warn!(
                        "Local ephemeral already consumed; falling back to static session for {}",
                        peer_id
                    );
                }
            } else {
                warn!("Invalid ephemeral public key length from {}", peer_id);
            }
        }

        Ok(())
    }

    /// Get network PeerId mapped to an identity peer id (if known)
    pub fn get_network_peer(&self, peer_id: &str) -> Option<PeerId> {
        self.peer_network_map.get(peer_id).cloned()
    }

    /// Return local ephemeral public key bytes for inclusion in identity announcements
    pub fn local_ephemeral_public_bytes(&self) -> Vec<u8> {
        self.local_ephemeral_public.to_bytes().to_vec()
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
        // Prefer PFS session if available
        let encrypted = if let Some(pfs) = self.pfs_sessions.get_mut(peer_id) {
            MessageCrypto::encrypt_text_pfs(pfs, text)
                .map_err(|e| MessagingError::EncryptionError(e.to_string()))?
        } else {
            let session = self
                .sessions
                .get_mut(peer_id)
                .ok_or_else(|| MessagingError::PeerNotFound(peer_id.to_string()))?;

            MessageCrypto::encrypt_text(session, text)
                .map_err(|e| MessagingError::EncryptionError(e.to_string()))?
        };

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
                // Prefer PFS session if present
                if let Some(pfs) = self.pfs_sessions.get_mut(from_peer_id) {
                    MessageCrypto::decrypt_text_pfs(pfs, encrypted)
                        .map_err(|e| MessagingError::DecryptionError(e.to_string()))
                } else {
                    let session = self
                        .sessions
                        .get_mut(from_peer_id)
                        .ok_or_else(|| MessagingError::PeerNotFound(from_peer_id.to_string()))?;

                    MessageCrypto::decrypt_text(session, encrypted)
                        .map_err(|e| MessagingError::DecryptionError(e.to_string()))
                }
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
    PeerStatus { peer_id: String, status: String },

    /// Peer is typing
    PeerTyping { peer_id: String, is_typing: bool },
}

/// Commands for the messaging layer
#[derive(Debug)]
pub enum MessagingCommand {
    /// Send a text message to a peer
    SendText { to: String, content: String },

    /// Announce identity to network
    AnnounceIdentity,

    /// Update status
    UpdateStatus { status: String },

    /// List registered peers
    ListPeers { response: mpsc::Sender<Vec<String>> },
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
    fn test_encrypted_message_bincode_roundtrip() {
        // This test verifies that encrypted messages can be serialized and deserialized correctly
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();

        let alice_public = PublicIdentity::from_identity(&alice);
        let bob_public = PublicIdentity::from_identity(&bob);

        let mut alice_handler = MessageHandler::new(alice);
        let mut bob_handler = MessageHandler::new(bob);

        alice_handler
            .register_peer(bob_public.clone(), None, None)
            .unwrap();
        bob_handler.register_peer(alice_public, None, None).unwrap();

        // Alice creates encrypted message
        let text = "Ciao";
        let encrypted_msg = alice_handler
            .prepare_encrypted_message(bob_public.peer_id().as_str(), text)
            .unwrap();

        // Serialize to bytes
        let bytes = encrypted_msg.to_bytes().unwrap();
        println!("Serialized {} bytes", bytes.len());
        println!(
            "First 64 bytes hex: {}",
            hex::encode(&bytes[..bytes.len().min(64)])
        );

        // Deserialize from bytes
        let deserialized_msg =
            Message::from_bytes(&bytes).expect("Failed to deserialize encrypted message");

        // Verify it's still an encrypted message
        match deserialized_msg {
            Message::Encrypted { .. } => {
                println!("✓ Deserialization successful!");
            }
            _ => panic!("Deserialized to wrong message type!"),
        }

        // Bob should be able to decrypt it
        let decrypted = bob_handler.decrypt_message(&deserialized_msg).unwrap();
        assert_eq!(decrypted, text);
    }

    #[test]
    fn test_message_handler() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();

        let mut alice_handler = MessageHandler::new(alice);
        let bob_public = PublicIdentity::from_identity(&bob);

        alice_handler.register_peer(bob_public, None, None).unwrap();
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

        alice_handler
            .register_peer(bob_public.clone(), None, None)
            .unwrap();
        bob_handler.register_peer(alice_public, None, None).unwrap();

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
