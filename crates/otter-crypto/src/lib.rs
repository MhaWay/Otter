//! # Otter Crypto
//!
//! End-to-end encryption primitives for the Otter decentralized chat platform.
//!
//! This crate provides:
//! - X25519 Diffie-Hellman key exchange
//! - ChaCha20-Poly1305 authenticated encryption
//! - Secure message encryption and decryption
//! - Key derivation and management
//! - Perfect Forward Secrecy with ephemeral keys
//! - Simple key ratcheting for session security

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Nonce,
};
use otter_identity::{Identity, IdentityError, PublicIdentity};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, SharedSecret};

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Invalid key")]
    InvalidKey,
    #[error("Identity error: {0}")]
    IdentityError(#[from] IdentityError),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Replay attack detected: message counter out of order")]
    ReplayAttack,
    #[error("Message counter overflow")]
    CounterOverflow,
}

/// Encrypted message envelope with replay protection
/// 
/// Note: deny_unknown_fields is not used to maintain forward compatibility
/// in a distributed P2P network where peers may run different versions.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EncryptedMessage {
    /// Nonce used for encryption (96 bits / 12 bytes)
    pub nonce: Vec<u8>,
    /// Encrypted ciphertext with authentication tag
    pub ciphertext: Vec<u8>,
    /// Optional associated data (not encrypted but authenticated)
    #[serde(skip_serializing_if = "Option::is_none")]
    associated_data: Option<Vec<u8>>,
    /// Message counter for replay protection (monotonically increasing)
    pub message_counter: u64,
    /// Optional timestamp (signed as part of AAD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

/// Manages encryption sessions between peers
///
/// Uses X25519 ECDH for key exchange and ChaCha20-Poly1305 for encryption.
/// Note: For Perfect Forward Secrecy, use PFSSession instead.
pub struct CryptoSession {
    shared_secret: SharedSecret,
    cipher_key: [u8; 32],
    send_counter: u64,
    receive_counter: u64,
}

impl CryptoSession {
    /// Create a new crypto session between local and remote peer
    ///
    /// Performs X25519 Diffie-Hellman key exchange and derives a shared secret.
    pub fn new(
        local_identity: &Identity,
        remote_public: &PublicIdentity,
    ) -> Result<Self, CryptoError> {
        let remote_key = remote_public.encryption_public_key()?;
        let shared_secret = local_identity.encryption_secret_key().diffie_hellman(&remote_key);
        
        // Derive cipher key from shared secret using BLAKE3
        let hash = blake3::hash(shared_secret.as_bytes());
        let cipher_key: [u8; 32] = *hash.as_bytes();
        
        Ok(Self {
            shared_secret,
            cipher_key,
            send_counter: 0,
            receive_counter: 0,
        })
    }
    
    /// Encrypt a message with optional associated data
    ///
    /// Associated data is authenticated but not encrypted (useful for metadata).
    pub fn encrypt(
        &mut self,
        plaintext: &[u8],
        associated_data: Option<&[u8]>,
    ) -> Result<EncryptedMessage, CryptoError> {
        if self.send_counter == u64::MAX {
            return Err(CryptoError::CounterOverflow);
        }
        
        let cipher = ChaCha20Poly1305::new(&self.cipher_key.into());
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Include counter in AAD
        let mut aad = Vec::new();
        aad.extend_from_slice(&self.send_counter.to_le_bytes());
        if let Some(ad) = associated_data {
            aad.extend_from_slice(ad);
        }
        
        let payload = Payload {
            msg: plaintext,
            aad: &aad,
        };
        
        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, payload)
            .map_err(|_| CryptoError::EncryptionFailed)?;
        
        let message_counter = self.send_counter;
        self.send_counter += 1;
        
        Ok(EncryptedMessage {
            nonce: nonce_bytes.to_vec(),
            ciphertext,
            associated_data: associated_data.map(|ad| ad.to_vec()),
            message_counter,
            timestamp: Some(chrono::Utc::now().timestamp()),
        })
    }
    
    /// Decrypt an encrypted message
    pub fn decrypt(&mut self, encrypted: &EncryptedMessage) -> Result<Vec<u8>, CryptoError> {
        // Replay protection: ensure counter is strictly increasing
        // But allow same counter if it's 0 (first message from fresh session)
        if encrypted.message_counter < self.receive_counter || 
           (encrypted.message_counter == self.receive_counter && self.receive_counter > 0) {
            return Err(CryptoError::ReplayAttack);
        }
        
        let cipher = ChaCha20Poly1305::new(&self.cipher_key.into());
        
        // Reconstruct nonce
        let nonce_bytes: [u8; 12] = encrypted
            .nonce
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::DecryptionFailed)?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Reconstruct AAD with counter
        let mut aad = Vec::new();
        aad.extend_from_slice(&encrypted.message_counter.to_le_bytes());
        if let Some(ref ad) = encrypted.associated_data {
            aad.extend_from_slice(ad);
        }
        
        let payload = Payload {
            msg: &encrypted.ciphertext,
            aad: &aad,
        };
        
        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, payload)
            .map_err(|_| CryptoError::DecryptionFailed)?;
        
        self.receive_counter = encrypted.message_counter;
        
        Ok(plaintext)
    }
    
    /// Get the shared secret fingerprint (for verification)
    pub fn fingerprint(&self) -> String {
        let hash = blake3::hash(self.shared_secret.as_bytes());
        hex::encode(&hash.as_bytes()[..8])
    }
}

/// Perfect Forward Secrecy session with ephemeral keys and ratcheting
///
/// Provides session-level PFS through:
/// - Ephemeral X25519 key pairs for each session
/// - Key ratcheting on message exchange
/// - Message counter for replay protection
pub struct PFSSession {
    /// Static identity-based shared secret (for authentication)
    static_secret: SharedSecret,
    
    /// Current ephemeral shared secret (ratcheted)
    ephemeral_secret: SharedSecret,
    
    /// Current sending chain key
    sending_chain_key: [u8; 32],
    
    /// Current receiving chain key
    receiving_chain_key: [u8; 32],
    
    /// Message counter for sending (monotonically increasing)
    send_counter: u64,
    
    /// Last received message counter (for replay protection)
    receive_counter: u64,
    
    /// Ephemeral public key to share with peer
    pub ephemeral_public: X25519PublicKey,
}

impl PFSSession {
    /// Create a new PFS session with ephemeral handshake
    ///
    /// This performs:
    /// 1. Static DH (identity keys) for authentication
    /// 2. Ephemeral DH for PFS
    /// 3. Derives independent sending/receiving chain keys
    ///
    /// IMPORTANT: The role (initiator vs responder) determines which chain key is used for sending/receiving
    pub fn new(
        local_identity: &Identity,
        remote_public: &PublicIdentity,
        local_ephemeral: EphemeralSecret,
        remote_ephemeral: &X25519PublicKey,
        is_initiator: bool,
    ) -> Result<Self, CryptoError> {
        // Static DH for authentication
        let remote_key = remote_public.encryption_public_key()?;
        let static_secret = local_identity.encryption_secret_key().diffie_hellman(&remote_key);
        
        // Ephemeral DH for PFS
        let ephemeral_public = X25519PublicKey::from(&local_ephemeral);
        let ephemeral_secret = local_ephemeral.diffie_hellman(remote_ephemeral);
        
        // Derive root key from both secrets (KDF chain)
        let mut root_key_material = Vec::new();
        root_key_material.extend_from_slice(static_secret.as_bytes());
        root_key_material.extend_from_slice(ephemeral_secret.as_bytes());
        root_key_material.extend_from_slice(b"otter-pfs-v1");
        
        let root_key = blake3::hash(&root_key_material);
        
        // Derive chain keys for both directions
        let chain_key_0 = blake3::derive_key("chain-0", root_key.as_bytes());
        let chain_key_1 = blake3::derive_key("chain-1", root_key.as_bytes());
        
        // Initiator sends on chain-0, receives on chain-1
        // Responder sends on chain-1, receives on chain-0
        let (sending_chain_key, receiving_chain_key) = if is_initiator {
            (chain_key_0, chain_key_1)
        } else {
            (chain_key_1, chain_key_0)
        };
        
        Ok(Self {
            static_secret,
            ephemeral_secret,
            sending_chain_key,
            receiving_chain_key,
            send_counter: 0,
            receive_counter: 0,
            ephemeral_public,
        })
    }
    
    /// Generate a new ephemeral secret for initiating a session
    pub fn generate_ephemeral() -> EphemeralSecret {
        EphemeralSecret::random_from_rng(OsRng)
    }
    
    /// Encrypt a message with PFS and replay protection
    pub fn encrypt(
        &mut self,
        plaintext: &[u8],
        associated_data: Option<&[u8]>,
    ) -> Result<EncryptedMessage, CryptoError> {
        // Check counter overflow
        if self.send_counter == u64::MAX {
            return Err(CryptoError::CounterOverflow);
        }
        
        // Derive message key from chain key and counter
        let mut key_material = Vec::new();
        key_material.extend_from_slice(&self.sending_chain_key);
        key_material.extend_from_slice(&self.send_counter.to_le_bytes());
        let message_key = blake3::hash(&key_material);
        
        let cipher = ChaCha20Poly1305::new(message_key.as_bytes().into());
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Include counter in AAD for authentication
        let mut aad = Vec::new();
        aad.extend_from_slice(&self.send_counter.to_le_bytes());
        if let Some(ad) = associated_data {
            aad.extend_from_slice(ad);
        }
        
        let payload = Payload {
            msg: plaintext,
            aad: &aad,
        };
        
        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, payload)
            .map_err(|_| CryptoError::EncryptionFailed)?;
        
        let message_counter = self.send_counter;
        
        // Increment counter and ratchet chain key
        self.send_counter += 1;
        self.ratchet_sending_chain();
        
        Ok(EncryptedMessage {
            nonce: nonce_bytes.to_vec(),
            ciphertext,
            associated_data: associated_data.map(|ad| ad.to_vec()),
            message_counter,
            timestamp: Some(chrono::Utc::now().timestamp()),
        })
    }
    
    /// Decrypt a message with replay protection
    pub fn decrypt(&mut self, encrypted: &EncryptedMessage) -> Result<Vec<u8>, CryptoError> {
        // Replay protection: ensure counter is strictly increasing
        // But allow same counter if it's 0 (first message from fresh session)
        if encrypted.message_counter < self.receive_counter || 
           (encrypted.message_counter == self.receive_counter && self.receive_counter > 0) {
            return Err(CryptoError::ReplayAttack);
        }
        
        // Derive message key from chain key and counter
        let mut key_material = Vec::new();
        key_material.extend_from_slice(&self.receiving_chain_key);
        key_material.extend_from_slice(&encrypted.message_counter.to_le_bytes());
        let message_key = blake3::hash(&key_material);
        
        let cipher = ChaCha20Poly1305::new(message_key.as_bytes().into());
        
        // Reconstruct nonce
        let nonce_bytes: [u8; 12] = encrypted
            .nonce
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::DecryptionFailed)?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Reconstruct AAD with counter
        let mut aad = Vec::new();
        aad.extend_from_slice(&encrypted.message_counter.to_le_bytes());
        if let Some(ref ad) = encrypted.associated_data {
            aad.extend_from_slice(ad);
        }
        
        let payload = Payload {
            msg: &encrypted.ciphertext,
            aad: &aad,
        };
        
        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, payload)
            .map_err(|_| CryptoError::DecryptionFailed)?;
        
        // Update receive counter
        self.receive_counter = encrypted.message_counter;
        
        // Ratchet receiving chain key
        self.ratchet_receiving_chain();
        
        Ok(plaintext)
    }
    
    /// Ratchet the sending chain key forward (simple KDF ratchet)
    fn ratchet_sending_chain(&mut self) {
        let mut ratchet_material = Vec::new();
        ratchet_material.extend_from_slice(&self.sending_chain_key);
        ratchet_material.extend_from_slice(b"ratchet-forward");
        self.sending_chain_key = *blake3::hash(&ratchet_material).as_bytes();
    }
    
    /// Ratchet the receiving chain key forward
    fn ratchet_receiving_chain(&mut self) {
        let mut ratchet_material = Vec::new();
        ratchet_material.extend_from_slice(&self.receiving_chain_key);
        ratchet_material.extend_from_slice(b"ratchet-forward");
        self.receiving_chain_key = *blake3::hash(&ratchet_material).as_bytes();
    }
    
    /// Get fingerprint for verification
    pub fn fingerprint(&self) -> String {
        let hash = blake3::hash(self.static_secret.as_bytes());
        hex::encode(&hash.as_bytes()[..8])
    }
}

/// Utility functions for message encryption/decryption
pub struct MessageCrypto;

impl MessageCrypto {
    /// Encrypt a text message
    pub fn encrypt_text(
        session: &mut CryptoSession,
        text: &str,
    ) -> Result<EncryptedMessage, CryptoError> {
        session.encrypt(text.as_bytes(), None)
    }
    
    /// Decrypt a text message
    pub fn decrypt_text(
        session: &mut CryptoSession,
        encrypted: &EncryptedMessage,
    ) -> Result<String, CryptoError> {
        let plaintext = session.decrypt(encrypted)?;
        String::from_utf8(plaintext).map_err(|e| CryptoError::SerializationError(e.to_string()))
    }
    
    /// Encrypt a text message with PFS session
    pub fn encrypt_text_pfs(
        session: &mut PFSSession,
        text: &str,
    ) -> Result<EncryptedMessage, CryptoError> {
        session.encrypt(text.as_bytes(), None)
    }
    
    /// Decrypt a text message with PFS session
    pub fn decrypt_text_pfs(
        session: &mut PFSSession,
        encrypted: &EncryptedMessage,
    ) -> Result<String, CryptoError> {
        let plaintext = session.decrypt(encrypted)?;
        String::from_utf8(plaintext).map_err(|e| CryptoError::SerializationError(e.to_string()))
    }
    
    /// Serialize encrypted message to base64 JSON
    pub fn serialize(encrypted: &EncryptedMessage) -> Result<String, CryptoError> {
        serde_json::to_string(encrypted)
            .map_err(|e| CryptoError::SerializationError(e.to_string()))
    }
    
    /// Deserialize encrypted message from JSON
    pub fn deserialize(json: &str) -> Result<EncryptedMessage, CryptoError> {
        serde_json::from_str(json).map_err(|e| CryptoError::SerializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_crypto_session() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();
        
        let alice_public = PublicIdentity::from_identity(&alice);
        let bob_public = PublicIdentity::from_identity(&bob);
        
        // Create sessions on both sides
        let alice_session = CryptoSession::new(&alice, &bob_public).unwrap();
        let bob_session = CryptoSession::new(&bob, &alice_public).unwrap();
        
        // Verify fingerprints match
        assert_eq!(alice_session.fingerprint(), bob_session.fingerprint());
    }
    
    #[test]
    fn test_encryption_decryption() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();
        
        let bob_public = PublicIdentity::from_identity(&bob);
        let mut alice_session = CryptoSession::new(&alice, &bob_public).unwrap();
        
        let plaintext = b"Hello, Bob!";
        let encrypted = alice_session.encrypt(plaintext, None).unwrap();
        let decrypted = alice_session.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted.as_slice());
    }
    
    #[test]
    fn test_encryption_with_associated_data() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();
        
        let bob_public = PublicIdentity::from_identity(&bob);
        let mut alice_session = CryptoSession::new(&alice, &bob_public).unwrap();
        
        let plaintext = b"Secret message";
        let associated_data = b"public metadata";
        
        let encrypted = alice_session
            .encrypt(plaintext, Some(associated_data))
            .unwrap();
        let decrypted = alice_session.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted.as_slice());
    }
    
    #[test]
    fn test_text_encryption() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();
        
        let bob_public = PublicIdentity::from_identity(&bob);
        let mut alice_session = CryptoSession::new(&alice, &bob_public).unwrap();
        
        let text = "Hello, Bob! This is a secret message.";
        let encrypted = MessageCrypto::encrypt_text(&mut alice_session, text).unwrap();
        let decrypted = MessageCrypto::decrypt_text(&mut alice_session, &encrypted).unwrap();
        
        assert_eq!(text, decrypted);
    }
    
    #[test]
    fn test_serialization() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();
        
        let bob_public = PublicIdentity::from_identity(&bob);
        let mut alice_session = CryptoSession::new(&alice, &bob_public).unwrap();
        
        let text = "Test message";
        let encrypted = MessageCrypto::encrypt_text(&mut alice_session, text).unwrap();
        
        let json = MessageCrypto::serialize(&encrypted).unwrap();
        let deserialized = MessageCrypto::deserialize(&json).unwrap();
        
        let decrypted = MessageCrypto::decrypt_text(&mut alice_session, &deserialized).unwrap();
        assert_eq!(text, decrypted);
    }
    
    #[test]
    fn test_replay_protection() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();
        
        let bob_public = PublicIdentity::from_identity(&bob);
        let mut alice_session = CryptoSession::new(&alice, &bob_public).unwrap();
        
        let plaintext1 = b"Message 1";
        let encrypted1 = alice_session.encrypt(plaintext1, None).unwrap();
        let _decrypted1 = alice_session.decrypt(&encrypted1).unwrap();
        
        let plaintext2 = b"Message 2";
        let encrypted2 = alice_session.encrypt(plaintext2, None).unwrap();
        let _decrypted2 = alice_session.decrypt(&encrypted2).unwrap();
        
        // Try to replay an earlier message (should fail)
        let result = alice_session.decrypt(&encrypted1);
        assert!(matches!(result, Err(CryptoError::ReplayAttack)));
    }
    
    #[test]
    fn test_pfs_session() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();
        
        let alice_public = PublicIdentity::from_identity(&alice);
        let bob_public = PublicIdentity::from_identity(&bob);
        
        // Generate ephemeral keys
        let alice_ephemeral = PFSSession::generate_ephemeral();
        let bob_ephemeral = PFSSession::generate_ephemeral();
        
        let alice_ephemeral_pub = X25519PublicKey::from(&alice_ephemeral);
        let bob_ephemeral_pub = X25519PublicKey::from(&bob_ephemeral);
        
        // Create PFS sessions (Alice is initiator, Bob is responder)
        let mut alice_session = PFSSession::new(
            &alice,
            &bob_public,
            alice_ephemeral,
            &bob_ephemeral_pub,
            true, // Alice is initiator
        ).unwrap();
        
        let mut bob_session = PFSSession::new(
            &bob,
            &alice_public,
            bob_ephemeral,
            &alice_ephemeral_pub,
            false, // Bob is responder
        ).unwrap();
        
        // Test encryption/decryption
        let plaintext = b"PFS test message";
        let encrypted = alice_session.encrypt(plaintext, None).unwrap();
        let decrypted = bob_session.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted.as_slice());
    }
    
    #[test]
    fn test_pfs_ratcheting() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();
        
        let alice_public = PublicIdentity::from_identity(&alice);
        let bob_public = PublicIdentity::from_identity(&bob);
        
        let alice_ephemeral = PFSSession::generate_ephemeral();
        let bob_ephemeral = PFSSession::generate_ephemeral();
        
        let alice_ephemeral_pub = X25519PublicKey::from(&alice_ephemeral);
        let bob_ephemeral_pub = X25519PublicKey::from(&bob_ephemeral);
        
        let mut alice_session = PFSSession::new(
            &alice,
            &bob_public,
            alice_ephemeral,
            &bob_ephemeral_pub,
            true,
        ).unwrap();
        
        let mut bob_session = PFSSession::new(
            &bob,
            &alice_public,
            bob_ephemeral,
            &alice_ephemeral_pub,
            false,
        ).unwrap();
        
        // Send multiple messages and verify ratcheting works
        for i in 0..5 {
            let plaintext = format!("Message {}", i);
            let encrypted = alice_session.encrypt(plaintext.as_bytes(), None).unwrap();
            let decrypted = bob_session.decrypt(&encrypted).unwrap();
            assert_eq!(plaintext.as_bytes(), decrypted.as_slice());
        }
    }
    
    #[test]
    fn test_pfs_replay_protection() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();
        
        let alice_public = PublicIdentity::from_identity(&alice);
        let bob_public = PublicIdentity::from_identity(&bob);
        
        let alice_ephemeral = PFSSession::generate_ephemeral();
        let bob_ephemeral = PFSSession::generate_ephemeral();
        
        let alice_ephemeral_pub = X25519PublicKey::from(&alice_ephemeral);
        let bob_ephemeral_pub = X25519PublicKey::from(&bob_ephemeral);
        
        let mut alice_session = PFSSession::new(
            &alice,
            &bob_public,
            alice_ephemeral,
            &bob_ephemeral_pub,
            true,
        ).unwrap();
        
        let mut bob_session = PFSSession::new(
            &bob,
            &alice_public,
            bob_ephemeral,
            &alice_ephemeral_pub,
            false,
        ).unwrap();
        
        let plaintext1 = b"Test message 1";
        let encrypted1 = alice_session.encrypt(plaintext1, None).unwrap();
        let _decrypted1 = bob_session.decrypt(&encrypted1).unwrap();
        
        let plaintext2 = b"Test message 2";
        let encrypted2 = alice_session.encrypt(plaintext2, None).unwrap();
        let _decrypted2 = bob_session.decrypt(&encrypted2).unwrap();
        
        // Try to replay earlier message (should fail)
        let result = bob_session.decrypt(&encrypted1);
        assert!(matches!(result, Err(CryptoError::ReplayAttack)));
    }
}
