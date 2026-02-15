//! # Otter Identity
//!
//! Identity management for the Otter decentralized chat platform.
//!
//! This crate provides:
//! - Ed25519 keypair generation for signing and identity
//! - X25519 keypair generation for encryption key exchange
//! - Peer identity representation and verification
//! - Key serialization and deserialization
//! - Multi-device support with device subkeys
//! - Trust chain and device revocation
//! - Trust management and fingerprint verification (TOFU model)

pub mod trust;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};
use chrono::{DateTime, Utc};

#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid public key format")]
    InvalidPublicKey,
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Key generation error: {0}")]
    KeyGenerationError(String),
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    #[error("Device revoked: {0}")]
    DeviceRevoked(String),
    #[error("Invalid device signature")]
    InvalidDeviceSignature,
}

/// A peer's identity in the network
///
/// Contains both Ed25519 signing keys and X25519 encryption keys.
/// The PeerId is derived from the Ed25519 public key.
#[derive(Clone)]
pub struct Identity {
    /// Ed25519 signing key pair
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    
    /// X25519 encryption key pair for key exchange
    encryption_secret: X25519StaticSecret,
    encryption_public: X25519PublicKey,
    
    /// Unique peer identifier derived from public key
    peer_id: PeerId,
}

impl Identity {
    /// Generate a new random identity
    pub fn generate() -> Result<Self, IdentityError> {
        let mut rng = OsRng;
        
        // Generate Ed25519 signing keypair
        let signing_key = SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();
        
        // Generate X25519 encryption keypair
        let encryption_secret = X25519StaticSecret::random_from_rng(&mut rng);
        let encryption_public = X25519PublicKey::from(&encryption_secret);
        
        // Derive peer ID from Ed25519 public key
        let peer_id = PeerId::from_public_key(&verifying_key);
        
        Ok(Self {
            signing_key,
            verifying_key,
            encryption_secret,
            encryption_public,
            peer_id,
        })
    }
    
    /// Get the peer ID for this identity
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }
    
    /// Get the Ed25519 verifying (public) key
    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }
    
    /// Get the X25519 public encryption key
    pub fn encryption_public_key(&self) -> &X25519PublicKey {
        &self.encryption_public
    }
    
    /// Get the X25519 secret key (for key exchange)
    pub fn encryption_secret_key(&self) -> &X25519StaticSecret {
        &self.encryption_secret
    }
    
    /// Sign a message with this identity
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }
    
    /// Export identity to JSON format
    pub fn to_json(&self) -> Result<String, IdentityError> {
        let export = IdentityExport {
            signing_key: hex::encode(self.signing_key.to_bytes()),
            encryption_secret: hex::encode(self.encryption_secret.to_bytes()),
        };
        
        serde_json::to_string_pretty(&export)
            .map_err(|e| IdentityError::SerializationError(e.to_string()))
    }
    
    /// Import identity from JSON format
    pub fn from_json(json: &str) -> Result<Self, IdentityError> {
        let export: IdentityExport = serde_json::from_str(json)
            .map_err(|e| IdentityError::SerializationError(e.to_string()))?;
        
        let signing_bytes = hex::decode(&export.signing_key)
            .map_err(|e| IdentityError::SerializationError(e.to_string()))?;
        
        let signing_key = SigningKey::from_bytes(
            &signing_bytes
                .try_into()
                .map_err(|_| IdentityError::InvalidPublicKey)?,
        );
        
        let verifying_key = signing_key.verifying_key();
        
        let encryption_bytes = hex::decode(&export.encryption_secret)
            .map_err(|e| IdentityError::SerializationError(e.to_string()))?;
        
        let encryption_secret = X25519StaticSecret::from(
            TryInto::<[u8; 32]>::try_into(encryption_bytes)
                .map_err(|_| IdentityError::InvalidPublicKey)?,
        );
        
        let encryption_public = X25519PublicKey::from(&encryption_secret);
        let peer_id = PeerId::from_public_key(&verifying_key);
        
        Ok(Self {
            signing_key,
            verifying_key,
            encryption_secret,
            encryption_public,
            peer_id,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct IdentityExport {
    signing_key: String,
    encryption_secret: String,
}

/// A unique identifier for a peer in the network
///
/// Derived from the peer's Ed25519 public key using BLAKE3 hashing
/// and encoded as base58 for human-readable representation.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PeerId(String);

impl PeerId {
    /// Create a PeerId from an Ed25519 public key
    pub fn from_public_key(public_key: &VerifyingKey) -> Self {
        let hash = blake3::hash(public_key.as_bytes());
        let encoded = bs58::encode(hash.as_bytes()).into_string();
        Self(encoded)
    }
    
    /// Create a PeerId from a base58 string
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
    
    /// Get the string representation
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PeerId({})", self.0)
    }
}

/// Public identity information that can be shared with other peers
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PublicIdentity {
    peer_id: PeerId,
    verifying_key: Vec<u8>,
    encryption_public: Vec<u8>,
}

impl PublicIdentity {
    /// Create a public identity from a full identity
    pub fn from_identity(identity: &Identity) -> Self {
        Self {
            peer_id: identity.peer_id.clone(),
            verifying_key: identity.verifying_key.to_bytes().to_vec(),
            encryption_public: identity.encryption_public.to_bytes().to_vec(),
        }
    }
    
    /// Get the peer ID
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }
    
    /// Get the Ed25519 verifying key
    pub fn verifying_key(&self) -> Result<VerifyingKey, IdentityError> {
        let bytes: [u8; 32] = self
            .verifying_key
            .as_slice()
            .try_into()
            .map_err(|_| IdentityError::InvalidPublicKey)?;
        
        VerifyingKey::from_bytes(&bytes).map_err(|_| IdentityError::InvalidPublicKey)
    }
    
    /// Get the X25519 encryption public key
    pub fn encryption_public_key(&self) -> Result<X25519PublicKey, IdentityError> {
        let bytes: [u8; 32] = self
            .encryption_public
            .as_slice()
            .try_into()
            .map_err(|_| IdentityError::InvalidPublicKey)?;
        
        Ok(X25519PublicKey::from(bytes))
    }
    
    /// Verify a signature on a message
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), IdentityError> {
        let key = self.verifying_key()?;
        key.verify(message, signature)
            .map_err(|_| IdentityError::InvalidSignature)
    }
}

/// Device identifier for multi-device support
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct DeviceId(String);

impl DeviceId {
    /// Generate a new random device ID
    pub fn generate() -> Self {
        use rand::Rng;
        let mut rng = OsRng;
        let random_bytes: [u8; 16] = rng.gen();
        Self(bs58::encode(random_bytes).into_string())
    }
    
    /// Create from string
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
    
    /// Get string representation
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Device key signed by root identity
///
/// Enables multi-device support where one user identity can have multiple devices
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DeviceKey {
    /// Unique device identifier
    pub device_id: DeviceId,
    
    /// Device-specific signing key (public part)
    pub device_verifying_key: Vec<u8>,
    
    /// Device-specific encryption key (public part)
    pub device_encryption_key: Vec<u8>,
    
    /// Device name/label
    pub device_name: String,
    
    /// When this device was added
    pub created_at: DateTime<Utc>,
    
    /// Whether this device is revoked
    pub revoked: bool,
    
    /// Revocation timestamp
    pub revoked_at: Option<DateTime<Utc>>,
    
    /// Signature by root identity over device key
    pub root_signature: Vec<u8>,
}

impl DeviceKey {
    /// Create a new device key
    pub fn new(
        device_id: DeviceId,
        device_verifying_key: VerifyingKey,
        device_encryption_key: X25519PublicKey,
        device_name: String,
        root_identity: &Identity,
    ) -> Result<Self, IdentityError> {
        let created_at = Utc::now();
        
        // Create message to sign (device_id + keys + timestamp)
        let mut message = Vec::new();
        message.extend_from_slice(device_id.as_str().as_bytes());
        message.extend_from_slice(device_verifying_key.as_bytes());
        message.extend_from_slice(device_encryption_key.as_bytes());
        message.extend_from_slice(created_at.to_rfc3339().as_bytes());
        
        // Sign with root identity
        let signature = root_identity.sign(&message);
        
        Ok(Self {
            device_id,
            device_verifying_key: device_verifying_key.to_bytes().to_vec(),
            device_encryption_key: device_encryption_key.to_bytes().to_vec(),
            device_name,
            created_at,
            revoked: false,
            revoked_at: None,
            root_signature: signature.to_bytes().to_vec(),
        })
    }
    
    /// Verify device key signature against root identity
    pub fn verify_signature(&self, root_public: &PublicIdentity) -> Result<(), IdentityError> {
        // Reconstruct message
        let mut message = Vec::new();
        message.extend_from_slice(self.device_id.as_str().as_bytes());
        message.extend_from_slice(&self.device_verifying_key);
        message.extend_from_slice(&self.device_encryption_key);
        message.extend_from_slice(self.created_at.to_rfc3339().as_bytes());
        
        // Parse signature
        let sig_bytes: [u8; 64] = self
            .root_signature
            .as_slice()
            .try_into()
            .map_err(|_| IdentityError::InvalidDeviceSignature)?;
        let signature = Signature::from_bytes(&sig_bytes);
        
        // Verify
        root_public.verify(&message, &signature)?;
        
        Ok(())
    }
    
    /// Mark device as revoked
    pub fn revoke(&mut self) {
        self.revoked = true;
        self.revoked_at = Some(Utc::now());
    }
    
    /// Check if device is revoked
    pub fn is_revoked(&self) -> bool {
        self.revoked
    }
}

/// Root identity with multi-device support
///
/// Represents a user that can have multiple devices, each with their own keys
#[derive(Clone)]
pub struct RootIdentity {
    /// The root identity
    root: Identity,
    
    /// List of device keys
    devices: Vec<DeviceKey>,
}

impl RootIdentity {
    /// Create a new root identity
    pub fn new() -> Result<Self, IdentityError> {
        Ok(Self {
            root: Identity::generate()?,
            devices: Vec::new(),
        })
    }
    
    /// Get the root identity
    pub fn root(&self) -> &Identity {
        &self.root
    }
    
    /// Add a new device
    pub fn add_device(
        &mut self,
        device_identity: &Identity,
        device_name: String,
    ) -> Result<DeviceId, IdentityError> {
        let device_id = DeviceId::generate();
        
        let device_key = DeviceKey::new(
            device_id.clone(),
            device_identity.verifying_key().clone(),
            device_identity.encryption_public_key().clone(),
            device_name,
            &self.root,
        )?;
        
        self.devices.push(device_key);
        
        Ok(device_id)
    }
    
    /// Revoke a device
    pub fn revoke_device(&mut self, device_id: &DeviceId) -> Result<(), IdentityError> {
        let device = self
            .devices
            .iter_mut()
            .find(|d| &d.device_id == device_id)
            .ok_or_else(|| IdentityError::DeviceNotFound(device_id.to_string()))?;
        
        device.revoke();
        Ok(())
    }
    
    /// Get all devices
    pub fn devices(&self) -> &[DeviceKey] {
        &self.devices
    }
    
    /// Get active (non-revoked) devices
    pub fn active_devices(&self) -> Vec<&DeviceKey> {
        self.devices.iter().filter(|d| !d.is_revoked()).collect()
    }
    
    /// Check if device is valid
    pub fn is_device_valid(&self, device_id: &DeviceId) -> bool {
        self.devices
            .iter()
            .find(|d| &d.device_id == device_id)
            .map(|d| !d.is_revoked())
            .unwrap_or(false)
    }
}

impl Default for RootIdentity {
    fn default() -> Self {
        Self::new().expect("Failed to generate root identity")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_identity_generation() {
        let identity = Identity::generate().unwrap();
        assert!(!identity.peer_id().as_str().is_empty());
    }
    
    #[test]
    fn test_signing_and_verification() {
        let identity = Identity::generate().unwrap();
        let message = b"Hello, Otter!";
        
        let signature = identity.sign(message);
        
        let public_identity = PublicIdentity::from_identity(&identity);
        assert!(public_identity.verify(message, &signature).is_ok());
    }
    
    #[test]
    fn test_identity_export_import() {
        let identity = Identity::generate().unwrap();
        let json = identity.to_json().unwrap();
        
        let restored = Identity::from_json(&json).unwrap();
        
        assert_eq!(identity.peer_id(), restored.peer_id());
        
        // Verify they can sign and verify the same way
        let message = b"test message";
        let sig = identity.sign(message);
        let pub_restored = PublicIdentity::from_identity(&restored);
        assert!(pub_restored.verify(message, &sig).is_ok());
    }
    
    #[test]
    fn test_peer_id_uniqueness() {
        let id1 = Identity::generate().unwrap();
        let id2 = Identity::generate().unwrap();
        
        assert_ne!(id1.peer_id(), id2.peer_id());
    }
    
    #[test]
    fn test_device_key_creation() {
        let root = Identity::generate().unwrap();
        let device = Identity::generate().unwrap();
        
        let device_key = DeviceKey::new(
            DeviceId::generate(),
            device.verifying_key().clone(),
            device.encryption_public_key().clone(),
            "My Device".to_string(),
            &root,
        ).unwrap();
        
        assert!(!device_key.is_revoked());
    }
    
    #[test]
    fn test_device_key_verification() {
        let root = Identity::generate().unwrap();
        let device = Identity::generate().unwrap();
        let root_public = PublicIdentity::from_identity(&root);
        
        let device_key = DeviceKey::new(
            DeviceId::generate(),
            device.verifying_key().clone(),
            device.encryption_public_key().clone(),
            "Test Device".to_string(),
            &root,
        ).unwrap();
        
        assert!(device_key.verify_signature(&root_public).is_ok());
    }
    
    #[test]
    fn test_root_identity_multi_device() {
        let mut root_identity = RootIdentity::new().unwrap();
        
        // Add first device
        let device1 = Identity::generate().unwrap();
        let device1_id = root_identity.add_device(&device1, "Device 1".to_string()).unwrap();
        
        // Add second device
        let device2 = Identity::generate().unwrap();
        let device2_id = root_identity.add_device(&device2, "Device 2".to_string()).unwrap();
        
        assert_eq!(root_identity.devices().len(), 2);
        assert_eq!(root_identity.active_devices().len(), 2);
        
        // Revoke first device
        root_identity.revoke_device(&device1_id).unwrap();
        
        assert_eq!(root_identity.active_devices().len(), 1);
        assert!(!root_identity.is_device_valid(&device1_id));
        assert!(root_identity.is_device_valid(&device2_id));
    }
    
    #[test]
    fn test_device_revocation() {
        let mut root_identity = RootIdentity::new().unwrap();
        let device = Identity::generate().unwrap();
        
        let device_id = root_identity.add_device(&device, "Test Device".to_string()).unwrap();
        assert!(root_identity.is_device_valid(&device_id));
        
        root_identity.revoke_device(&device_id).unwrap();
        assert!(!root_identity.is_device_valid(&device_id));
    }
}
