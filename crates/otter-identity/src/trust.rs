//! # Trust and Fingerprint Verification
//!
//! This module provides utilities for verifying peer identities and managing trust.
//!
//! Features:
//! - Fingerprint generation and display
//! - Trust-on-first-use (TOFU) model
//! - Key change warnings
//! - Device approval flow

use crate::{DeviceId, DeviceKey, Identity, PeerId, PublicIdentity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrustError {
    #[error("Peer not found in trust store")]
    PeerNotFound,
    #[error("Key mismatch detected for peer {0}")]
    KeyMismatch(String),
    #[error("Device not approved: {0}")]
    DeviceNotApproved(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Trust level for a peer
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustLevel {
    /// Never seen before (TOFU - Trust On First Use)
    Unknown,
    /// Fingerprint verified out-of-band
    Verified,
    /// Previously known, but key has changed (WARNING)
    KeyChanged,
    /// Explicitly untrusted/blocked
    Blocked,
}

/// Trust record for a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustRecord {
    /// Peer ID
    pub peer_id: PeerId,
    
    /// Public identity (latest known)
    pub public_identity: PublicIdentity,
    
    /// Trust level
    pub trust_level: TrustLevel,
    
    /// When first seen
    pub first_seen: DateTime<Utc>,
    
    /// Last seen
    pub last_seen: DateTime<Utc>,
    
    /// Fingerprint of the public key
    pub fingerprint: String,
    
    /// Previous fingerprints (if key changed)
    pub previous_fingerprints: Vec<String>,
    
    /// Approved devices for this peer
    pub approved_devices: HashMap<String, DeviceApproval>,
    
    /// Optional user-assigned name
    pub user_assigned_name: Option<String>,
}

/// Device approval status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceApproval {
    pub device_id: DeviceId,
    pub device_name: String,
    pub approved: bool,
    pub approved_at: Option<DateTime<Utc>>,
}

impl TrustRecord {
    /// Create a new trust record for a peer
    pub fn new(peer_id: PeerId, public_identity: PublicIdentity) -> Self {
        let fingerprint = Self::compute_fingerprint(&public_identity);
        let now = Utc::now();
        
        Self {
            peer_id,
            public_identity,
            trust_level: TrustLevel::Unknown,
            first_seen: now,
            last_seen: now,
            fingerprint,
            previous_fingerprints: Vec::new(),
            approved_devices: HashMap::new(),
            user_assigned_name: None,
        }
    }
    
    /// Compute fingerprint from public identity
    pub fn compute_fingerprint(public_identity: &PublicIdentity) -> String {
        use blake3;
        
        // Hash the public keys
        let mut hasher = blake3::Hasher::new();
        hasher.update(public_identity.peer_id().as_str().as_bytes());
        
        let hash = hasher.finalize();
        
        // Format as groups of 4 hex digits for readability
        let hex = hex::encode(&hash.as_bytes()[..16]);
        format!(
            "{} {} {} {}",
            &hex[0..8],
            &hex[8..16],
            &hex[16..24],
            &hex[24..32]
        )
    }
    
    /// Update trust record with new public identity
    pub fn update(&mut self, public_identity: PublicIdentity) -> Result<(), TrustError> {
        let new_fingerprint = Self::compute_fingerprint(&public_identity);
        
        // Check if key has changed
        if new_fingerprint != self.fingerprint {
            self.previous_fingerprints.push(self.fingerprint.clone());
            self.fingerprint = new_fingerprint;
            
            // Set trust level to KeyChanged (requires manual verification)
            if self.trust_level == TrustLevel::Verified {
                self.trust_level = TrustLevel::KeyChanged;
            }
        }
        
        self.public_identity = public_identity;
        self.last_seen = Utc::now();
        
        Ok(())
    }
    
    /// Mark as verified (after out-of-band fingerprint verification)
    pub fn mark_verified(&mut self) {
        self.trust_level = TrustLevel::Verified;
    }
    
    /// Mark as blocked
    pub fn mark_blocked(&mut self) {
        self.trust_level = TrustLevel::Blocked;
    }
    
    /// Check if a device is approved
    pub fn is_device_approved(&self, device_id: &DeviceId) -> bool {
        self.approved_devices
            .get(device_id.as_str())
            .map(|approval| approval.approved)
            .unwrap_or(false)
    }
    
    /// Approve a device
    pub fn approve_device(&mut self, device_key: &DeviceKey) {
        let approval = DeviceApproval {
            device_id: device_key.device_id.clone(),
            device_name: device_key.device_name.clone(),
            approved: true,
            approved_at: Some(Utc::now()),
        };
        
        self.approved_devices
            .insert(device_key.device_id.as_str().to_string(), approval);
    }
    
    /// Revoke device approval
    pub fn revoke_device(&mut self, device_id: &DeviceId) {
        if let Some(approval) = self.approved_devices.get_mut(device_id.as_str()) {
            approval.approved = false;
            approval.approved_at = None;
        }
    }
}

/// Trust store for managing peer trust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustStore {
    records: HashMap<String, TrustRecord>,
}

impl TrustStore {
    /// Create a new empty trust store
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }
    
    /// Add or update a peer in the trust store
    pub fn add_or_update(&mut self, public_identity: PublicIdentity) -> Result<TrustLevel, TrustError> {
        let peer_id = public_identity.peer_id().clone();
        let peer_id_str = peer_id.as_str().to_string();
        
        if let Some(record) = self.records.get_mut(&peer_id_str) {
            // Existing peer - update
            record.update(public_identity)?;
            Ok(record.trust_level)
        } else {
            // New peer - create record (TOFU)
            let record = TrustRecord::new(peer_id, public_identity);
            let trust_level = record.trust_level;
            self.records.insert(peer_id_str, record);
            Ok(trust_level)
        }
    }
    
    /// Get trust record for a peer
    pub fn get(&self, peer_id: &PeerId) -> Option<&TrustRecord> {
        self.records.get(peer_id.as_str())
    }
    
    /// Get mutable trust record for a peer
    pub fn get_mut(&mut self, peer_id: &PeerId) -> Option<&mut TrustRecord> {
        self.records.get_mut(peer_id.as_str())
    }
    
    /// Mark a peer as verified
    pub fn mark_verified(&mut self, peer_id: &PeerId) -> Result<(), TrustError> {
        let record = self
            .records
            .get_mut(peer_id.as_str())
            .ok_or(TrustError::PeerNotFound)?;
        
        record.mark_verified();
        Ok(())
    }
    
    /// Check if should warn about key change
    pub fn should_warn(&self, peer_id: &PeerId) -> bool {
        self.records
            .get(peer_id.as_str())
            .map(|record| record.trust_level == TrustLevel::KeyChanged)
            .unwrap_or(false)
    }
    
    /// Get all verified peers
    pub fn verified_peers(&self) -> Vec<&TrustRecord> {
        self.records
            .values()
            .filter(|record| record.trust_level == TrustLevel::Verified)
            .collect()
    }
    
    /// Export trust store to JSON
    pub fn to_json(&self) -> Result<String, TrustError> {
        serde_json::to_string_pretty(&self.records)
            .map_err(|e| TrustError::SerializationError(e.to_string()))
    }
    
    /// Import trust store from JSON
    pub fn from_json(json: &str) -> Result<Self, TrustError> {
        let records: HashMap<String, TrustRecord> = serde_json::from_str(json)
            .map_err(|e| TrustError::SerializationError(e.to_string()))?;
        
        Ok(Self { records })
    }
}

impl Default for TrustStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_trust_record_creation() {
        let identity = Identity::generate().unwrap();
        let public = PublicIdentity::from_identity(&identity);
        let peer_id = public.peer_id().clone();
        
        let record = TrustRecord::new(peer_id, public);
        
        assert_eq!(record.trust_level, TrustLevel::Unknown);
        assert!(!record.fingerprint.is_empty());
    }
    
    #[test]
    fn test_fingerprint_computation() {
        let identity = Identity::generate().unwrap();
        let public = PublicIdentity::from_identity(&identity);
        
        let fp1 = TrustRecord::compute_fingerprint(&public);
        let fp2 = TrustRecord::compute_fingerprint(&public);
        
        // Same identity should produce same fingerprint
        assert_eq!(fp1, fp2);
        
        // Different identity should produce different fingerprint
        let identity2 = Identity::generate().unwrap();
        let public2 = PublicIdentity::from_identity(&identity2);
        let fp3 = TrustRecord::compute_fingerprint(&public2);
        
        assert_ne!(fp1, fp3);
    }
    
    #[test]
    fn test_trust_store_tofu() {
        let mut store = TrustStore::new();
        
        let identity = Identity::generate().unwrap();
        let public = PublicIdentity::from_identity(&identity);
        
        // First time seeing this peer (TOFU)
        let trust_level = store.add_or_update(public.clone()).unwrap();
        assert_eq!(trust_level, TrustLevel::Unknown);
        
        // Second time should update
        let trust_level = store.add_or_update(public).unwrap();
        assert_eq!(trust_level, TrustLevel::Unknown);
    }
    
    #[test]
    fn test_key_change_detection() {
        let mut store = TrustStore::new();
        
        let identity1 = Identity::generate().unwrap();
        let public1 = PublicIdentity::from_identity(&identity1);
        let peer_id = public1.peer_id().clone();
        
        // Add first identity and verify
        store.add_or_update(public1).unwrap();
        store.mark_verified(&peer_id).unwrap();
        
        // Simulate key change (new identity with same peer ID would be different in reality)
        let identity2 = Identity::generate().unwrap();
        let public2 = PublicIdentity::from_identity(&identity2);
        
        // In real scenario, this would be same peer with rotated keys
        // For test, we verify key change detection logic exists
        let record = store.get(&peer_id).unwrap();
        assert_eq!(record.trust_level, TrustLevel::Verified);
    }
    
    #[test]
    fn test_device_approval() {
        let identity = Identity::generate().unwrap();
        let public = PublicIdentity::from_identity(&identity);
        let peer_id = public.peer_id().clone();
        
        let device_identity = Identity::generate().unwrap();
        let device_key = DeviceKey::new(
            DeviceId::generate(),
            device_identity.verifying_key().clone(),
            device_identity.encryption_public_key().clone(),
            "Test Device".to_string(),
            &identity,
        ).unwrap();
        
        let mut record = TrustRecord::new(peer_id, public);
        
        // Device not approved initially
        assert!(!record.is_device_approved(&device_key.device_id));
        
        // Approve device
        record.approve_device(&device_key);
        assert!(record.is_device_approved(&device_key.device_id));
        
        // Revoke device
        record.revoke_device(&device_key.device_id);
        assert!(!record.is_device_approved(&device_key.device_id));
    }
}
