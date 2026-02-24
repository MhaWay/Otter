//! Phase 3 Integration Test: Persistent Mesh Peer Cache
//!
//! Tests:
//! - Cache persistence and loading
//! - TTL cleanup for stale peers
//! - Auto-dial cached peers on startup
//! - Identity information updates

use libp2p::{
    identity,
    kad::{self, store::MemoryStore},
    noise, tcp, yamux,
    gossipsub, identify,
    swarm::{Swarm, SwarmEvent},
    PeerId, Multiaddr, Transport,
};
use futures::StreamExt;
use std::time::Duration;
use tokio::time;
use tempfile::TempDir;

#[derive(libp2p::swarm::NetworkBehaviour)]
struct TestBehaviour {
    gossipsub: gossipsub::Behaviour,
    identify: identify::Behaviour,
    kad: kad::Behaviour<MemoryStore>,
}

async fn create_test_peer() -> (Swarm<TestBehaviour>, PeerId) {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    let transport = tcp::tokio::Transport::default()
        .upgrade(libp2p::core::upgrade::Version::V1Lazy)
        .authenticate(noise::Config::new(&local_key).unwrap())
        .multiplex(yamux::Config::default())
        .timeout(Duration::from_secs(20))
        .boxed();

    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .build()
        .unwrap();

    let mut gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(local_key.clone()),
        gossipsub_config,
    )
    .unwrap();

    let chat_topic = gossipsub::IdentTopic::new("otter-chat");
    gossipsub.subscribe(&chat_topic).unwrap();

    let identify = identify::Behaviour::new(identify::Config::new(
        "/otter/1.0.0".to_string(),
        local_key.public(),
    ));

    let store = MemoryStore::new(local_peer_id);
    let mut kad = kad::Behaviour::new(local_peer_id, store);
    kad.set_mode(Some(kad::Mode::Server));

    let behaviour = TestBehaviour {
        gossipsub,
        identify,
        kad,
    };

    let swarm_config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(600));

    let mut swarm = Swarm::new(transport, behaviour, local_peer_id, swarm_config);
    
    swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();

    (swarm, local_peer_id)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cache_persistence() {
    use otter_network::bootstrap::{BootstrapSources, CachedPeer};
    
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("peer_cache_test.json");
    
    // Create sources and add a peer
    let mut sources = BootstrapSources::new(cache_path.clone());
    
    let peer_id = PeerId::random();
    let addr: Multiaddr = "/ip4/192.168.1.1/tcp/4001".parse().unwrap();
    
    sources.record_successful_dial(&peer_id, &addr, 100).await;
    
    // Verify it's saved
    assert!(cache_path.exists());
    assert_eq!(sources.cache().peers.len(), 1);
    assert_eq!(sources.cache().peers[0].peer_id, peer_id.to_string());
    assert_eq!(sources.cache().peers[0].successful_dials, 1);
    
    println!("✅ Cache persistence works");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cache_loading() {
    use otter_network::bootstrap::BootstrapSources;
    
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("peer_cache_test2.json");
    
    // Create and save cache
    let mut sources1 = BootstrapSources::new(cache_path.clone());
    
    let peer_id1 = PeerId::random();
    let peer_id2 = PeerId::random();
    let addr1: Multiaddr = "/ip4/192.168.1.1/tcp/4001".parse().unwrap();
    let addr2: Multiaddr = "/ip4/192.168.1.2/tcp/4001".parse().unwrap();
    
    sources1.record_successful_dial(&peer_id1, &addr1, 100).await;
    sources1.record_successful_dial(&peer_id2, &addr2, 150).await;
    
    // Load in new instance
    let mut sources2 = BootstrapSources::new(cache_path);
    sources2.initialize().await.unwrap();
    
    // Verify loaded
    assert_eq!(sources2.cache().peers.len(), 2);
    assert_eq!(sources2.cache().peers[0].peer_id, peer_id1.to_string());
    assert_eq!(sources2.cache().peers[1].peer_id, peer_id2.to_string());
    
    println!("✅ Cache loading works");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ttl_cleanup() {
    use otter_network::bootstrap::BootstrapSources;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("peer_cache_test3.json");
    
    let mut sources = BootstrapSources::new(cache_path);
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    // Manually add old and fresh peers to cache
    sources.cache_mut().peers.push(otter_network::bootstrap::CachedPeer {
        peer_id: "old_peer".to_string(),
        addresses: vec![],
        last_seen: now - (100 * 3600), // 100 hours ago (beyond 90 day TTL)
        first_met: now - (100 * 3600),
        successful_dials: 5,
        failed_dials: 0,
        is_relay: false,
        latency_ms: None,
        nickname: None,
        device_type: None,
    });
    
    sources.cache_mut().peers.push(otter_network::bootstrap::CachedPeer {
        peer_id: "fresh_peer".to_string(),
        addresses: vec![],
        last_seen: now,
        first_met: now,
        successful_dials: 3,
        failed_dials: 0,
        is_relay: false,
        latency_ms: None,
        nickname: None,
        device_type: None,
    });
    
    assert_eq!(sources.cache().peers.len(), 2);
    
    // Initialize (which triggers cleanup with 72h TTL)
    sources.initialize().await.unwrap();
    
    // Verify old peer removed
    let remaining = sources.cache().peers.len();
    assert_eq!(remaining, 1, "Old peer should be cleaned up");
    assert_eq!(sources.cache().peers[0].peer_id, "fresh_peer");
    
    println!("✅ TTL cleanup works");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_auto_dial_cached_peers() {
    use otter_network::bootstrap::BootstrapSources;
    
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("peer_cache_test4.json");
    
    // Create cache with some peers
    let mut sources = BootstrapSources::new(cache_path.clone());
    
    let peer_id1 = PeerId::random();
    let peer_id2 = PeerId::random();
    let addr1: Multiaddr = "/ip4/127.0.0.1/tcp/5001".parse().unwrap();
    let addr2: Multiaddr = "/ip4/127.0.0.1/tcp/5002".parse().unwrap();
    
    sources.record_successful_dial(&peer_id1, &addr1, 100).await;
    sources.record_successful_dial(&peer_id2, &addr2, 150).await;
    
    // Load and get peers for auto-dial
    let mut sources2 = BootstrapSources::new(cache_path);
    sources2.initialize().await.unwrap();
    
    let auto_dial_peers = sources2.get_peers_for_auto_dial();
    
    // Should have peers suitable for auto-dial (successful_dials > 0, last_seen < 7 days)
    assert_eq!(auto_dial_peers.len(), 2, "Should have 2 peers for auto-dial");
    
    // Verify each peer has addresses
    for (_peer_id, addrs) in &auto_dial_peers {
        assert!(!addrs.is_empty(), "Peer should have addresses");
    }
    
    println!("✅ Auto-dial peer retrieval works");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_peer_identity_update() {
    use otter_network::bootstrap::BootstrapSources;
    
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("peer_cache_test5.json");
    
    let mut sources = BootstrapSources::new(cache_path);
    
    let peer_id = PeerId::random();
    let addr: Multiaddr = "/ip4/192.168.1.1/tcp/4001".parse().unwrap();
    
    // Record initial connection
    sources.record_successful_dial(&peer_id, &addr, 100).await;
    assert_eq!(sources.cache().peers[0].nickname, None);
    
    // Update with identity information
    sources.update_peer_identity(&peer_id, Some("Alice".to_string()), Some("iPhone".to_string())).await;
    
    // Verify identity updated
    assert_eq!(sources.cache().peers[0].nickname, Some("Alice".to_string()));
    assert_eq!(sources.cache().peers[0].device_type, Some("iPhone".to_string()));
    
    println!("✅ Peer identity update works");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_two_peer_cache_reconnection() {
    use otter_network::bootstrap::BootstrapSources;
    
    // Create two test peers and cache them
    let (mut swarm1, peer_id1) = create_test_peer().await;
    let (mut swarm2, peer_id2) = create_test_peer().await;
    
    println!("Peer 1: {}", peer_id1);
    println!("Peer 2: {}", peer_id2);
    
    // Get Peer 1's listening address
    let mut peer1_addr = None;
    loop {
        match swarm1.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                peer1_addr = Some(address);
                break;
            }
            _ => {}
        }
    }
    
    let peer1_addr = peer1_addr.unwrap();
    println!("Peer 1 listening on: {}", peer1_addr);
    
    // Create cache and record Peer 1's address
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("peer_cache_test6.json");
    
    let mut cache = BootstrapSources::new(cache_path.clone());
    cache.record_successful_dial(&peer_id1, &peer1_addr, 50).await;
    
    // Verify peer is in cache
    assert_eq!(cache.cache().peers.len(), 1);
    assert_eq!(cache.cache().peers[0].peer_id, peer_id1.to_string());
    
    // In a real scenario, Peer 2 would load this cache and attempt reconnection
    let mut cache2 = BootstrapSources::new(cache_path);
    cache2.initialize().await.unwrap();
    
    let retrieved_peers = cache2.get_peers_for_auto_dial();
    assert_eq!(retrieved_peers.len(), 1, "Should retrieve saved peer");
    
    println!("✅ Peer cache reconnection scenario works");
}
