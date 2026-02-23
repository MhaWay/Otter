//! Bootstrap module for peer discovery and network initialization
//!
//! This module implements a 4-tier bootstrap architecture:
//! - Tier 0: Hardcoded fallback peers (emergency when all else fails)
//! - Tier 1: DNS multiaddr resolution from public domains (runtime)
//! - Tier 2: Local peer cache (persistent, with TTL)
//! - Tier 3: Peer gossip discovery (decentralized)
//!
//! This ensures the network is:
//! - Self-sustaining (no dependency on bootstrap after initial connection)
//! - Resilient (survives bootstrap node failures)
//! - Fully decentralized (no proprietary servers)
//! - Always accessible (hardcoded fallback guarantees connectivity)

use libp2p::{Multiaddr, PeerId};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error as ThisError;
use tracing::{debug, info, warn, error};

#[derive(ThisError, Debug)]
pub enum BootstrapError {
    #[error("DNS resolution failed: {0}")]
    DnsResolutionFailed(String),
    #[error("No bootstrap peers available")]
    NoBootstrapPeers,
    #[error("Invalid multiaddr: {0}")]
    InvalidMultiaddr(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// DNS bootstrap domains (Tier 1)
/// These domains provide TXT records with multiaddr from public libp2p/IPFS infrastructure
pub const DNS_BOOTSTRAP_DOMAINS: &[&str] = &[
    "_dnsaddr.bootstrap.libp2p.io",
    "_dnsaddr.discovery.ipfs.io",
    "dnsaddr.bootstrap.libp2p.io",  // Fallback without underscore prefix
];

/// Hardcoded fallback bootstrap peers (Tier 0 - Emergency fallback)
/// These are well-known IPFS bootstrap nodes that should always be available
pub const HARDCODED_BOOTSTRAP_PEERS: &[&str] = &[
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
    "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
    "/ip4/104.131.131.82/udp/4001/quic-v1/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
];

/// Bootstrap peer entry with multiaddr
#[derive(Clone, Debug)]
pub struct BootstrapPeer {
    pub peer_id: PeerId,
    pub multiaddr: Multiaddr,
}

/// Cached peer entry with metadata
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CachedPeer {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub last_seen: i64,      // Unix timestamp
    pub successful_dials: u32, // Reputation scoring
    pub failed_dials: u32,
    pub is_relay: bool,
    pub latency_ms: Option<u32>,
}

impl CachedPeer {
    /// Calculate peer score for reputation-based selection
    pub fn reputation_score(&self) -> f64 {
        let total = (self.successful_dials + self.failed_dials) as f64;
        if total == 0.0 {
            return 0.5; // neutral score
        }
        
        let success_rate = self.successful_dials as f64 / total;
        
        // Bonus for relay capability
        let relay_bonus = if self.is_relay { 0.1 } else { 0.0 };
        
        // Penalty for high latency
        let latency_penalty = match self.latency_ms {
            Some(ms) if ms > 1000 => -0.1,
            Some(ms) if ms > 500 => -0.05,
            _ => 0.0,
        };
        
        (success_rate + relay_bonus + latency_penalty).clamp(0.0, 1.0)
    }
}

/// Peer cache with TTL management
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PeerCache {
    pub version: u32,
    pub last_updated: i64,
    pub peers: Vec<CachedPeer>,
}

impl PeerCache {
    /// Create new empty cache
    pub fn new() -> Self {
        Self {
            version: 1,
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            peers: Vec::new(),
        }
    }
    
    /// Remove peers older than TTL
    pub fn cleanup_stale(&mut self, ttl_hours: i64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        let threshold = now - (ttl_hours * 3600);
        
        let before = self.peers.len();
        self.peers.retain(|peer| peer.last_seen > threshold);
        let after = self.peers.len();
        
        if before != after {
            info!("🧹 Cleaned {} stale peers (TTL: {}h)", before - after, ttl_hours);
        }
    }
    
    /// Get peers sorted by reputation score
    pub fn get_sorted_by_reputation(&self) -> Vec<&CachedPeer> {
        let mut peers: Vec<&CachedPeer> = self.peers.iter().collect();
        peers.sort_by(|a, b| {
            b.reputation_score()
                .partial_cmp(&a.reputation_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        peers
    }
}

impl Default for PeerCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Bootstrap sources with 3-tier architecture
pub struct BootstrapSources {
    /// Peer cache (Tier 2)
    cache: PeerCache,
    /// Cache file path
    cache_path: PathBuf,
    /// DNS cache with TTL
    dns_cache: HashMap<String, (Vec<Multiaddr>, SystemTime)>,
    /// Relay peers discovered
    relay_peers: HashMap<PeerId, Vec<Multiaddr>>,
}

impl BootstrapSources {
    /// Create a new bootstrap sources manager
    pub fn new(cache_path: PathBuf) -> Self {
        Self {
            cache: PeerCache::new(),
            cache_path,
            dns_cache: HashMap::new(),
            relay_peers: HashMap::new(),
        }
    }
    
    /// Initialize from disk cache
    pub async fn initialize(&mut self) -> Result<(), BootstrapError> {
        if self.cache_path.exists() {
            match self.load_cache().await {
                Ok(_) => info!("📂 Loaded peer cache from disk"),
                Err(e) => warn!("⚠️ Failed to load cache: {}", e),
            }
        } else {
            info!("📝 No existing cache, starting fresh");
        }
        
        // Cleanup stale entries (72h TTL)
        self.cache.cleanup_stale(72);
        
        Ok(())
    }

    // =================================================================
    // TIER 1: DNS BOOTSTRAP (Runtime resolution from public sources)
    // =================================================================
    
    /// Resolve DNS TXT records for bootstrap multiaddr
    /// Uses DNS-over-HTTPS for privacy and reliability
    pub async fn resolve_dns_bootstrap(&mut self) -> Result<Vec<Multiaddr>, BootstrapError> {
        info!("🌐 [Tier 1] DNS Bootstrap: Resolving from public domains...");
        
        let mut all_addrs = Vec::new();
        
        for domain in DNS_BOOTSTRAP_DOMAINS {
            // Check DNS cache first (TTL: 24h)
            if let Some((cached, timestamp)) = self.dns_cache.get(&domain.to_string()) {
                if timestamp.elapsed().unwrap_or(Duration::MAX) < Duration::from_secs(86400) {
                    debug!("📦 Using cached DNS results for {}", domain);
                    all_addrs.extend(cached.clone());
                    continue;
                }
            }
            
            match self.resolve_dns_txt_records(domain).await {
                Ok(addrs) => {
                    info!("✅ Resolved {} addresses from {}", addrs.len(), domain);
                    
                    // Update DNS cache
                    self.dns_cache.insert(
                        domain.to_string(),
                        (addrs.clone(), SystemTime::now())
                    );
                    
                    all_addrs.extend(addrs);
                }
                Err(e) => {
                    warn!("❌ Failed to resolve {}: {}", domain, e);
                }
            }
        }
        
        info!("🎯 Total DNS bootstrap addresses: {}", all_addrs.len());
        Ok(all_addrs)
    }
    
    /// Resolve TXT records for a domain using DNS-over-HTTPS
    async fn resolve_dns_txt_records(&self, domain: &str) -> Result<Vec<Multiaddr>, BootstrapError> {
        // Use Cloudflare DNS-over-HTTPS API
        let url = format!(
            "https://cloudflare-dns.com/dns-query?name={}&type=TXT",
            domain
        );
        
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Accept", "application/dns-json")
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| BootstrapError::DnsResolutionFailed(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(BootstrapError::DnsResolutionFailed(
                format!("HTTP {}", response.status())
            ));
        }
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| BootstrapError::DnsResolutionFailed(e.to_string()))?;
        
        // Parse TXT records
        let mut multiaddrs = Vec::new();
        
        if let Some(answers) = json["Answer"].as_array() {
            for answer in answers {
                if let Some(data) = answer["data"].as_str() {
                    // Remove quotes from TXT record
                    let txt_data = data.trim_matches('"');
                    
                    // Parse as multiaddr
                    if let Ok(addr) = Multiaddr::from_str(txt_data) {
                        multiaddrs.push(addr);
                    } else if txt_data.starts_with("dnsaddr=") {
                        // Handle dnsaddr= format
                        let addr_str = txt_data.trim_start_matches("dnsaddr=");
                        if let Ok(addr) = Multiaddr::from_str(addr_str) {
                            multiaddrs.push(addr);
                        }
                    }
                }
            }
        }
        
        Ok(multiaddrs)
    }
    
    // =================================================================
    // TIER 2: LOCAL CACHE (Persistent peer storage with TTL)
    // =================================================================
    
    /// Load cached peers from disk
    pub async fn load_cached_peers(&mut self) -> Result<Vec<Multiaddr>, BootstrapError> {
        info!("📂 [Tier 2] Local Cache: Loading persisted peers...");
        
        self.load_cache().await?;
        
        // Get peers sorted by reputation
        let top_peers = self.cache.get_sorted_by_reputation();
        
        let mut addrs = Vec::new();
        for peer in top_peers.iter().take(50) {  // Top 50 by reputation
            for addr_str in &peer.addresses {
                if let Ok(addr) = Multiaddr::from_str(addr_str) {
                    addrs.push(addr);
                }
            }
        }
        
        info!("✅ Loaded {} cached addresses (top reputation)", addrs.len());
        Ok(addrs)
    }
    
    /// Save cache to disk
    async fn save_cache(&self) -> Result<(), BootstrapError> {
        let json = serde_json::to_string_pretty(&self.cache)
            .map_err(|e| BootstrapError::SerializationError(e.to_string()))?;
        
        // Ensure parent directory exists
        if let Some(parent) = self.cache_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        tokio::fs::write(&self.cache_path, json).await?;
        debug!("💾 Saved peer cache to {:?}", self.cache_path);
        
        Ok(())
    }
    
    /// Load cache from disk
    async fn load_cache(&mut self) -> Result<(), BootstrapError> {
        let contents = tokio::fs::read_to_string(&self.cache_path).await?;
        self.cache = serde_json::from_str(&contents)
            .map_err(|e| BootstrapError::SerializationError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Update cache with successful connection
    pub async fn record_successful_dial(&mut self, peer_id: &PeerId, addr: &Multiaddr, latency_ms: u32) {
        let peer_id_str = peer_id.to_string();
        let addr_str = addr.to_string();
        
        // Find or create peer entry
        if let Some(peer) = self.cache.peers.iter_mut().find(|p| p.peer_id == peer_id_str) {
            peer.successful_dials += 1;
            peer.last_seen = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            peer.latency_ms = Some(latency_ms);
            
            if !peer.addresses.contains(&addr_str) {
                peer.addresses.push(addr_str);
            }
        } else {
            // New peer
            self.cache.peers.push(CachedPeer {
                peer_id: peer_id_str,
                addresses: vec![addr_str],
                last_seen: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
                successful_dials: 1,
                failed_dials: 0,
                is_relay: false,
                latency_ms: Some(latency_ms),
            });
        }
        
        // Save to disk asynchronously
        if let Err(e) = self.save_cache().await {
            warn!("Failed to save cache: {}", e);
        }
    }
    
    /// Record failed dial attempt
    pub async fn record_failed_dial(&mut self, peer_id: &PeerId) {
        let peer_id_str = peer_id.to_string();
        
        if let Some(peer) = self.cache.peers.iter_mut().find(|p| p.peer_id == peer_id_str) {
            peer.failed_dials += 1;
        }
    }
    
    // =================================================================
    // TIER 3: PEER GOSSIP (Decentralized discovery)
    // =================================================================
    
    /// Request known peers from a connected peer (gossip discovery)
    /// This is called after initial connection to build routing table
    pub async fn gossip_discovery(&self, _peer_id: &PeerId) -> Vec<Multiaddr> {
        info!("🗣️ [Tier 3] Peer Gossip: Requesting known peers...");
        
        // This will be implemented by sending a custom protocol message
        // to request peer list from the connected peer
        // For now, return empty (to be implemented in network layer)
        
        Vec::new()
    }

    // =================================================================
    // RELAY SUPPORT (NAT Traversal)
    // =================================================================

    /// Mark peer as relay-capable
    pub async fn mark_as_relay(&mut self, peer_id: &PeerId, addrs: Vec<Multiaddr>) {
        info!("🔁 Relay peer discovered: {}", peer_id);
        
        self.relay_peers.insert(*peer_id, addrs.clone());
        
        // Update in cache
        let peer_id_str = peer_id.to_string();
        if let Some(peer) = self.cache.peers.iter_mut().find(|p| p.peer_id == peer_id_str) {
            peer.is_relay = true;
            
            // Add relay addresses
            for addr in addrs {
                let addr_str = addr.to_string();
                if !peer.addresses.contains(&addr_str) {
                    peer.addresses.push(addr_str);
                }
            }
        }
        
        let _ = self.save_cache().await;
    }
    
    /// Get known relay peers
    pub fn get_relay_peers(&self) -> Vec<(PeerId, Vec<Multiaddr>)> {
        self.relay_peers
            .iter()
            .map(|(id, addrs)| (*id, addrs.clone()))
            .collect()
    }
    
    /// Get relay peers from cache
    pub fn get_cached_relays(&self) -> Vec<Multiaddr> {
        self.cache
            .peers
            .iter()
            .filter(|p| p.is_relay)
            .flat_map(|p| {
                p.addresses
                    .iter()
                    .filter_map(|a| Multiaddr::from_str(a).ok())
            })
            .collect()
    }
    
    // =================================================================
    // COMPLETE BOOTSTRAP SEQUENCE
    // =================================================================
    
    /// Execute complete bootstrap sequence (all 3 tiers)
    /// Returns discovered multiaddrs sorted by priority
    pub async fn bootstrap(&mut self) -> Vec<Multiaddr> {
        info!("🚀 Starting complete bootstrap sequence...");
        
        let mut all_addrs = Vec::new();
        
        // Tier 2: Try local cache first (fastest)
        match self.load_cached_peers().await {
            Ok(cached) => {
                info!("✅ Tier 2: Found {} cached peers", cached.len());
                all_addrs.extend(cached);
            }
            Err(e) => {
                warn!("⚠️ Tier 2: Cache load failed: {}", e);
            }
        }
        
        // Tier 1: DNS bootstrap (if cache insufficient)
        if all_addrs.len() < 5 {
            match self.resolve_dns_bootstrap().await {
                Ok(dns_addrs) => {
                    info!("✅ Tier 1: Found {} DNS bootstrap peers", dns_addrs.len());
                    all_addrs.extend(dns_addrs);
                }
                Err(e) => {
                    error!("❌ Tier 1: DNS bootstrap failed: {}", e);
                }
            }
        }
        
        // Tier 0: Hardcoded fallback (emergency - when all else fails)
        if all_addrs.is_empty() {
            warn!("🆘 Tier 0: Using hardcoded fallback bootstrap peers");
            for addr_str in HARDCODED_BOOTSTRAP_PEERS {
                match Multiaddr::from_str(addr_str) {
                    Ok(addr) => {
                        info!("  ✓ Added hardcoded peer: {}", addr);
                        all_addrs.push(addr);
                    }
                    Err(e) => {
                        warn!("  ✗ Invalid hardcoded addr {}: {}", addr_str, e);
                    }
                }
            }
            info!("✅ Tier 0: Added {} hardcoded bootstrap peers", all_addrs.len());
        }
        
        // Tier 3: Gossip discovery happens after connection (in network layer)
        
        info!("🎯 Bootstrap complete: {} total addresses", all_addrs.len());
        all_addrs
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        let relay_count = self.cache.peers.iter().filter(|p| p.is_relay).count();
        let avg_score = if self.cache.peers.is_empty() {
            0.0
        } else {
            self.cache.peers.iter().map(|p| p.reputation_score()).sum::<f64>()
                / self.cache.peers.len() as f64
        };
        
        CacheStats {
            total_peers: self.cache.peers.len(),
            relay_peers: relay_count,
            average_reputation: avg_score,
            last_updated: self.cache.last_updated,
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_peers: usize,
    pub relay_peers: usize,
    pub average_reputation: f64,
    pub last_updated: i64,
}

impl Default for BootstrapSources {
    fn default() -> Self {
        Self::new(PathBuf::from(".otter/peer_cache.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_bootstrap_sources_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("peer_cache.json");
        let sources = BootstrapSources::new(cache_path);
        assert_eq!(sources.cache.peers.len(), 0);
    }

    #[test]
    fn test_peer_reputation_score() {
        let mut peer = CachedPeer {
            peer_id: "test".to_string(),
            addresses: vec![],
            last_seen: 0,
            successful_dials: 8,
            failed_dials: 2,
            is_relay: false,
            latency_ms: Some(100),
        };
        
        // 80% success rate
        let score = peer.reputation_score();
        assert!(score > 0.7 && score < 0.9);
        
        // With relay bonus
        peer.is_relay = true;
        let relay_score = peer.reputation_score();
        assert!(relay_score > score);
    }

    #[test]
    fn test_cache_cleanup_stale() {
        let mut cache = PeerCache::new();
        
        // Add old peer (100 hours ago)
        let old_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - (100 * 3600);
        
        cache.peers.push(CachedPeer {
            peer_id: "old_peer".to_string(),
            addresses: vec![],
            last_seen: old_timestamp,
            successful_dials: 5,
            failed_dials: 0,
            is_relay: false,
            latency_ms: None,
        });
        
        // Add fresh peer
        cache.peers.push(CachedPeer {
            peer_id: "fresh_peer".to_string(),
            addresses: vec![],
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            successful_dials: 3,
            failed_dials: 0,
            is_relay: false,
            latency_ms: None,
        });
        
        assert_eq!(cache.peers.len(), 2);
        
        // Cleanup with 72h TTL
        cache.cleanup_stale(72);
        
        // Only fresh peer should remain
        assert_eq!(cache.peers.len(), 1);
        assert_eq!(cache.peers[0].peer_id, "fresh_peer");
    }

    #[tokio::test]
    async fn test_cache_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("test_cache.json");
        
        let mut sources = BootstrapSources::new(cache_path.clone());
        
        // Add a test peer
        let peer_id = PeerId::random();
        let addr: Multiaddr = "/ip4/192.168.1.1/tcp/4001".parse().unwrap();
        
        sources.record_successful_dial(&peer_id, &addr, 150).await;
        
        // Verify saved
        assert!(cache_path.exists());
        
        // Load in new instance
        let mut sources2 = BootstrapSources::new(cache_path);
        sources2.initialize().await.unwrap();
        
        assert_eq!(sources2.cache.peers.len(), 1);
        assert_eq!(sources2.cache.peers[0].peer_id, peer_id.to_string());
    }
}
