//! Heartbeat module for peer presence tracking
//!
//! Provides mechanisms to broadcast and track peer online status
//! using Gossipsub heartbeat messages.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Heartbeat message broadcasted periodically to indicate presence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    /// Peer ID as string
    pub peer_id: String,
    
    /// UNIX timestamp when heartbeat was sent
    pub timestamp: i64,
    
    /// Protocol version for future compatibility
    pub version: u32,
}

impl HeartbeatMessage {
    /// Create a new heartbeat message
    pub fn new(peer_id: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        Self {
            peer_id,
            timestamp,
            version: 1,
        }
    }
    
    /// Serialize to JSON bytes for Gossipsub
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
    
    /// Deserialize from JSON bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
    
    /// Check if heartbeat is fresh (less than max_age_secs old)
    pub fn is_fresh(&self, max_age_secs: i64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        (now - self.timestamp) < max_age_secs
    }
    
    /// Get age in seconds
    pub fn age_secs(&self) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        now - self.timestamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_serialization() {
        let msg = HeartbeatMessage::new("12D3Koo...".to_string());
        
        let bytes = msg.to_bytes().unwrap();
        let decoded = HeartbeatMessage::from_bytes(&bytes).unwrap();
        
        assert_eq!(msg.peer_id, decoded.peer_id);
        assert_eq!(msg.version, decoded.version);
    }
    
    #[test]
    fn test_heartbeat_freshness() {
        let msg = HeartbeatMessage::new("test".to_string());
        
        // Should be fresh immediately
        assert!(msg.is_fresh(60));
        
        // Age should be ~0 seconds
        assert!(msg.age_secs() < 2, "Age should be very recent");
    }
}
