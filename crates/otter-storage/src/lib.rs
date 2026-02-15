//! # Otter Storage
//!
//! Persistence layer for the Otter decentralized chat platform.
//!
//! This crate provides:
//! - Storage trait for pluggable backends
//! - File-based storage implementation with atomic writes
//! - Identity key persistence
//! - Trust store persistence
//! - Session state management
//! - Peer cache persistence

use otter_identity::{Identity, PublicIdentity, trust::TrustStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

/// Persisted identity data
#[derive(Serialize, Deserialize, Clone)]
pub struct IdentityData {
    pub signing_key_bytes: Vec<u8>,
    pub encryption_secret_bytes: Vec<u8>,
    pub peer_id: String,
    pub created_at: i64,
}

/// Persisted session state
#[derive(Serialize, Deserialize, Clone)]
pub struct SessionData {
    pub peer_id: String,
    pub shared_secret_bytes: Vec<u8>,
    pub send_counter: u64,
    pub receive_counter: u64,
    pub created_at: i64,
    pub last_used: i64,
}

/// Cached peer information
#[derive(Serialize, Deserialize, Clone)]
pub struct PeerCacheEntry {
    pub peer_id: String,
    pub public_identity: PublicIdentity,
    pub addresses: Vec<String>,
    pub last_seen: i64,
}

/// Trait for storage backends
#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    /// Load identity from storage
    async fn load_identity(&self) -> Result<Option<IdentityData>, StorageError>;
    
    /// Save identity to storage
    async fn save_identity(&self, identity: &IdentityData) -> Result<(), StorageError>;
    
    /// Load trust store from storage
    async fn load_trust_store(&self) -> Result<Option<TrustStore>, StorageError>;
    
    /// Save trust store to storage
    async fn save_trust_store(&self, trust_store: &TrustStore) -> Result<(), StorageError>;
    
    /// Load all session data
    async fn load_sessions(&self) -> Result<HashMap<String, SessionData>, StorageError>;
    
    /// Save a session
    async fn save_session(&self, peer_id: &str, session: &SessionData) -> Result<(), StorageError>;
    
    /// Delete a session
    async fn delete_session(&self, peer_id: &str) -> Result<(), StorageError>;
    
    /// Load peer cache
    async fn load_peer_cache(&self) -> Result<HashMap<String, PeerCacheEntry>, StorageError>;
    
    /// Save peer cache entry
    async fn save_peer_cache_entry(&self, entry: &PeerCacheEntry) -> Result<(), StorageError>;
    
    /// Clear all data (for testing)
    async fn clear_all(&self) -> Result<(), StorageError>;
}

/// File-based storage implementation
pub struct FileStorage {
    base_path: PathBuf,
}

impl FileStorage {
    /// Create a new file storage instance
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }
    
    /// Get path for identity file
    fn identity_path(&self) -> PathBuf {
        self.base_path.join("identity.json")
    }
    
    /// Get path for trust store file
    fn trust_store_path(&self) -> PathBuf {
        self.base_path.join("trust_store.json")
    }
    
    /// Get path for sessions directory
    fn sessions_dir(&self) -> PathBuf {
        self.base_path.join("sessions")
    }
    
    /// Get path for specific session file
    fn session_path(&self, peer_id: &str) -> PathBuf {
        self.sessions_dir().join(format!("{}.json", peer_id))
    }
    
    /// Get path for peer cache file
    fn peer_cache_path(&self) -> PathBuf {
        self.base_path.join("peer_cache.json")
    }
    
    /// Atomically write data to a file
    async fn atomic_write(&self, path: &Path, data: &[u8]) -> Result<(), StorageError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        // Write to temp file first
        let temp_path = path.with_extension("tmp");
        let mut file = fs::File::create(&temp_path).await?;
        file.write_all(data).await?;
        file.sync_all().await?;
        
        // Atomic rename
        fs::rename(&temp_path, path).await?;
        
        Ok(())
    }
    
    /// Read file contents
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, StorageError> {
        if !path.exists() {
            return Err(StorageError::NotFound(format!("{:?}", path)));
        }
        
        let mut file = fs::File::open(path).await?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await?;
        Ok(contents)
    }
}

#[async_trait::async_trait]
impl Storage for FileStorage {
    async fn load_identity(&self) -> Result<Option<IdentityData>, StorageError> {
        let path = self.identity_path();
        if !path.exists() {
            return Ok(None);
        }
        
        let data = self.read_file(&path).await?;
        let identity: IdentityData = serde_json::from_slice(&data)
            .map_err(|e| StorageError::DeserializationError(e.to_string()))?;
        
        Ok(Some(identity))
    }
    
    async fn save_identity(&self, identity: &IdentityData) -> Result<(), StorageError> {
        let data = serde_json::to_vec_pretty(identity)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        self.atomic_write(&self.identity_path(), &data).await
    }
    
    async fn load_trust_store(&self) -> Result<Option<TrustStore>, StorageError> {
        let path = self.trust_store_path();
        if !path.exists() {
            return Ok(None);
        }
        
        let data = self.read_file(&path).await?;
        let trust_store: TrustStore = serde_json::from_slice(&data)
            .map_err(|e| StorageError::DeserializationError(e.to_string()))?;
        
        Ok(Some(trust_store))
    }
    
    async fn save_trust_store(&self, trust_store: &TrustStore) -> Result<(), StorageError> {
        let data = serde_json::to_vec_pretty(trust_store)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        self.atomic_write(&self.trust_store_path(), &data).await
    }
    
    async fn load_sessions(&self) -> Result<HashMap<String, SessionData>, StorageError> {
        let sessions_dir = self.sessions_dir();
        if !sessions_dir.exists() {
            return Ok(HashMap::new());
        }
        
        let mut sessions = HashMap::new();
        let mut entries = fs::read_dir(&sessions_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.read_file(&path).await {
                    Ok(data) => {
                        match serde_json::from_slice::<SessionData>(&data) {
                            Ok(session) => {
                                sessions.insert(session.peer_id.clone(), session);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to deserialize session {:?}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read session {:?}: {}", path, e);
                    }
                }
            }
        }
        
        Ok(sessions)
    }
    
    async fn save_session(&self, peer_id: &str, session: &SessionData) -> Result<(), StorageError> {
        let data = serde_json::to_vec_pretty(session)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        self.atomic_write(&self.session_path(peer_id), &data).await
    }
    
    async fn delete_session(&self, peer_id: &str) -> Result<(), StorageError> {
        let path = self.session_path(peer_id);
        if path.exists() {
            fs::remove_file(path).await?;
        }
        Ok(())
    }
    
    async fn load_peer_cache(&self) -> Result<HashMap<String, PeerCacheEntry>, StorageError> {
        let path = self.peer_cache_path();
        if !path.exists() {
            return Ok(HashMap::new());
        }
        
        let data = self.read_file(&path).await?;
        let cache: HashMap<String, PeerCacheEntry> = serde_json::from_slice(&data)
            .map_err(|e| StorageError::DeserializationError(e.to_string()))?;
        
        Ok(cache)
    }
    
    async fn save_peer_cache_entry(&self, entry: &PeerCacheEntry) -> Result<(), StorageError> {
        let mut cache = self.load_peer_cache().await?;
        cache.insert(entry.peer_id.clone(), entry.clone());
        
        let data = serde_json::to_vec_pretty(&cache)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        self.atomic_write(&self.peer_cache_path(), &data).await
    }
    
    async fn clear_all(&self) -> Result<(), StorageError> {
        if self.base_path.exists() {
            fs::remove_dir_all(&self.base_path).await?;
        }
        fs::create_dir_all(&self.base_path).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use otter_identity::Identity;
    
    async fn create_test_storage() -> (FileStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage::new(temp_dir.path());
        (storage, temp_dir)
    }
    
    #[tokio::test]
    async fn test_identity_persistence() {
        let (storage, _temp) = create_test_storage().await;
        
        // Initially no identity
        assert!(storage.load_identity().await.unwrap().is_none());
        
        // Save identity
        let identity_data = IdentityData {
            signing_key_bytes: vec![1, 2, 3],
            encryption_secret_bytes: vec![4, 5, 6],
            peer_id: "test_peer".to_string(),
            created_at: 12345,
        };
        
        storage.save_identity(&identity_data).await.unwrap();
        
        // Load identity
        let loaded = storage.load_identity().await.unwrap().unwrap();
        assert_eq!(loaded.peer_id, "test_peer");
        assert_eq!(loaded.signing_key_bytes, vec![1, 2, 3]);
    }
    
    #[tokio::test]
    async fn test_session_persistence() {
        let (storage, _temp) = create_test_storage().await;
        
        let session = SessionData {
            peer_id: "peer1".to_string(),
            shared_secret_bytes: vec![1, 2, 3, 4],
            send_counter: 10,
            receive_counter: 5,
            created_at: 1000,
            last_used: 2000,
        };
        
        storage.save_session("peer1", &session).await.unwrap();
        
        let sessions = storage.load_sessions().await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions.get("peer1").unwrap().send_counter, 10);
        
        // Delete session
        storage.delete_session("peer1").await.unwrap();
        let sessions = storage.load_sessions().await.unwrap();
        assert_eq!(sessions.len(), 0);
    }
    
    #[tokio::test]
    async fn test_peer_cache_persistence() {
        let (storage, _temp) = create_test_storage().await;
        
        // Create a test identity and get public identity from it
        let identity = Identity::generate().unwrap();
        let public_identity = PublicIdentity::from_identity(&identity);
        
        let entry = PeerCacheEntry {
            peer_id: identity.peer_id().as_str().to_string(),
            public_identity,
            addresses: vec!["addr1".to_string(), "addr2".to_string()],
            last_seen: 5000,
        };
        
        storage.save_peer_cache_entry(&entry).await.unwrap();
        
        let cache = storage.load_peer_cache().await.unwrap();
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.values().next().unwrap().addresses.len(), 2);
    }
    
    #[tokio::test]
    async fn test_atomic_write() {
        let (storage, _temp) = create_test_storage().await;
        
        let identity = IdentityData {
            signing_key_bytes: vec![1, 2, 3],
            encryption_secret_bytes: vec![4, 5, 6],
            peer_id: "test".to_string(),
            created_at: 100,
        };
        
        // Write multiple times - should always succeed atomically
        for i in 0..5 {
            let mut id = identity.clone();
            id.created_at = i;
            storage.save_identity(&id).await.unwrap();
        }
        
        // Last write should win
        let loaded = storage.load_identity().await.unwrap().unwrap();
        assert_eq!(loaded.created_at, 4);
    }
    
    #[tokio::test]
    async fn test_clear_all() {
        let (storage, _temp) = create_test_storage().await;
        
        // Save some data
        let identity = IdentityData {
            signing_key_bytes: vec![1],
            encryption_secret_bytes: vec![2],
            peer_id: "test".to_string(),
            created_at: 100,
        };
        storage.save_identity(&identity).await.unwrap();
        
        // Clear all
        storage.clear_all().await.unwrap();
        
        // Should be empty
        assert!(storage.load_identity().await.unwrap().is_none());
    }
}
