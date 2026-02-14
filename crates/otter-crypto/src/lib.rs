//! # Otter Crypto
//!
//! End-to-end encryption primitives for the Otter decentralized chat platform.
//!
//! This crate provides:
//! - X25519 Diffie-Hellman key exchange
//! - ChaCha20-Poly1305 authenticated encryption
//! - Secure message encryption and decryption
//! - Key derivation and management

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Nonce,
};
use otter_identity::{Identity, IdentityError, PublicIdentity};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use x25519_dalek::SharedSecret;

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
}

/// Encrypted message envelope
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EncryptedMessage {
    /// Nonce used for encryption (96 bits / 12 bytes)
    nonce: Vec<u8>,
    /// Encrypted ciphertext with authentication tag
    ciphertext: Vec<u8>,
    /// Optional associated data (not encrypted but authenticated)
    #[serde(skip_serializing_if = "Option::is_none")]
    associated_data: Option<Vec<u8>>,
}

/// Manages encryption sessions between peers
///
/// Uses X25519 ECDH for key exchange and ChaCha20-Poly1305 for encryption.
pub struct CryptoSession {
    shared_secret: SharedSecret,
    cipher_key: [u8; 32],
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
        })
    }
    
    /// Encrypt a message with optional associated data
    ///
    /// Associated data is authenticated but not encrypted (useful for metadata).
    pub fn encrypt(
        &self,
        plaintext: &[u8],
        associated_data: Option<&[u8]>,
    ) -> Result<EncryptedMessage, CryptoError> {
        let cipher = ChaCha20Poly1305::new(&self.cipher_key.into());
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Prepare payload with optional associated data
        let payload = if let Some(ad) = associated_data {
            Payload {
                msg: plaintext,
                aad: ad,
            }
        } else {
            Payload {
                msg: plaintext,
                aad: b"",
            }
        };
        
        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, payload)
            .map_err(|_| CryptoError::EncryptionFailed)?;
        
        Ok(EncryptedMessage {
            nonce: nonce_bytes.to_vec(),
            ciphertext,
            associated_data: associated_data.map(|ad| ad.to_vec()),
        })
    }
    
    /// Decrypt an encrypted message
    pub fn decrypt(&self, encrypted: &EncryptedMessage) -> Result<Vec<u8>, CryptoError> {
        let cipher = ChaCha20Poly1305::new(&self.cipher_key.into());
        
        // Reconstruct nonce
        let nonce_bytes: [u8; 12] = encrypted
            .nonce
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::DecryptionFailed)?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Prepare payload with optional associated data
        let payload = if let Some(ref ad) = encrypted.associated_data {
            Payload {
                msg: &encrypted.ciphertext,
                aad: ad.as_slice(),
            }
        } else {
            Payload {
                msg: &encrypted.ciphertext,
                aad: b"",
            }
        };
        
        // Decrypt
        cipher
            .decrypt(nonce, payload)
            .map_err(|_| CryptoError::DecryptionFailed)
    }
    
    /// Get the shared secret fingerprint (for verification)
    pub fn fingerprint(&self) -> String {
        let hash = blake3::hash(self.shared_secret.as_bytes());
        hex::encode(&hash.as_bytes()[..8])
    }
}

/// Utility functions for message encryption/decryption
pub struct MessageCrypto;

impl MessageCrypto {
    /// Encrypt a text message
    pub fn encrypt_text(
        session: &CryptoSession,
        text: &str,
    ) -> Result<EncryptedMessage, CryptoError> {
        session.encrypt(text.as_bytes(), None)
    }
    
    /// Decrypt a text message
    pub fn decrypt_text(
        session: &CryptoSession,
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
        let alice_session = CryptoSession::new(&alice, &bob_public).unwrap();
        
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
        let alice_session = CryptoSession::new(&alice, &bob_public).unwrap();
        
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
        let alice_session = CryptoSession::new(&alice, &bob_public).unwrap();
        
        let text = "Hello, Bob! This is a secret message.";
        let encrypted = MessageCrypto::encrypt_text(&alice_session, text).unwrap();
        let decrypted = MessageCrypto::decrypt_text(&alice_session, &encrypted).unwrap();
        
        assert_eq!(text, decrypted);
    }
    
    #[test]
    fn test_serialization() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();
        
        let bob_public = PublicIdentity::from_identity(&bob);
        let alice_session = CryptoSession::new(&alice, &bob_public).unwrap();
        
        let text = "Test message";
        let encrypted = MessageCrypto::encrypt_text(&alice_session, text).unwrap();
        
        let json = MessageCrypto::serialize(&encrypted).unwrap();
        let deserialized = MessageCrypto::deserialize(&json).unwrap();
        
        let decrypted = MessageCrypto::decrypt_text(&alice_session, &deserialized).unwrap();
        assert_eq!(text, decrypted);
    }
}
