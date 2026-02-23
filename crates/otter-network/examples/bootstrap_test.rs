//! Bootstrap Real Network Test
//!
//! Testa il bootstrap module contro la rete pubblica libp2p/IPFS
//!
//! OBIETTIVI:
//! 1. ✅ Risolvere DNS bootstrap reali
//! 2. ✅ Connettersi ad almeno 1 peer pubblico
//! 3. ✅ Ricevere peer list via DHT
//! 4. ✅ Popolare cache locale
//! 5. ✅ Riavvio senza DNS
//!
//! USAGE:
//!   # Clean start
//!   rm -rf ~/.otter/peer_cache.json
//!   cargo run --example bootstrap_test

use libp2p::{
    identity,
    kad::{self, store::MemoryStore},
    noise,
    tcp, yamux,
    dns::TokioDnsConfig,
    swarm::{Swarm, SwarmEvent},
    Transport,
};
use futures::StreamExt;
use otter_network::bootstrap::BootstrapSources;
use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::time;

// Metriche obiettivo
const TARGET_FIRST_PEER_SECS: u64 = 15;
const TARGET_ROUTING_TABLE_SIZE: usize = 20;
const TARGET_TIMEOUT_SECS: u64 = 120;

#[derive(Debug)]
struct TestMetrics {
    start_time: Instant,
    first_peer_connected_at: Option<Instant>,
    total_peers_discovered: usize,
    total_connections_attempted: usize,
    total_connections_successful: usize,
    routing_table_size: usize,
    cache_loaded_peers: usize,
    dns_resolved_peers: usize,
}

impl TestMetrics {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            first_peer_connected_at: None,
            total_peers_discovered: 0,
            total_connections_attempted: 0,
            total_connections_successful: 0,
            routing_table_size: 0,
            cache_loaded_peers: 0,
            dns_resolved_peers: 0,
        }
    }

    fn time_to_first_peer(&self) -> Option<Duration> {
        self.first_peer_connected_at.map(|t| t.duration_since(self.start_time))
    }

    fn total_elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    fn success_rate(&self) -> f64 {
        if self.total_connections_attempted == 0 {
            0.0
        } else {
            (self.total_connections_successful as f64 / self.total_connections_attempted as f64) * 100.0
        }
    }

    fn print_summary(&self) {
        println!("\n{:=<60}", "");
        println!("📊 TEST SUMMARY");
        println!("{:=<60}", "");
        
        println!("\n⏱️  TIMING:");
        if let Some(time_to_first) = self.time_to_first_peer() {
            let status = if time_to_first.as_secs() <= TARGET_FIRST_PEER_SECS {
                "✅"
            } else {
                "⚠️"
            };
            println!("  {} First peer: {:.2}s (target: {}s)", 
                status, time_to_first.as_secs_f64(), TARGET_FIRST_PEER_SECS);
        } else {
            println!("  ❌ First peer: NONE");
        }
        println!("  Total elapsed: {:.2}s", self.total_elapsed().as_secs_f64());

        println!("\n🔍 DISCOVERY:");
        println!("  Cache loaded: {} peers", self.cache_loaded_peers);
        println!("  DNS resolved: {} peers", self.dns_resolved_peers);
        println!("  Total discovered: {} peers", self.total_peers_discovered);

        println!("\n🔌 CONNECTIONS:");
        println!("  Attempted: {}", self.total_connections_attempted);
        println!("  Successful: {} ({:.1}%)", 
            self.total_connections_successful, self.success_rate());

        println!("\n📡 DHT:");
        let rt_status = if self.routing_table_size >= TARGET_ROUTING_TABLE_SIZE {
            "✅"
        } else if self.routing_table_size >= 10 {
            "⚠️"
        } else {
            "❌"
        };
        println!("  {} Routing table: {} peers (target: {})", 
            rt_status, self.routing_table_size, TARGET_ROUTING_TABLE_SIZE);

        println!("\n{:=<60}", "");
        
        // Verdetto finale
        let passed = self.first_peer_connected_at.is_some()
            && self.routing_table_size >= 10
            && self.total_connections_successful > 0;

        if passed {
            println!("✅ BOOTSTRAP TEST PASSED");
            if self.routing_table_size >= TARGET_ROUTING_TABLE_SIZE 
                && self.time_to_first_peer().map_or(false, |t| t.as_secs() <= TARGET_FIRST_PEER_SECS) {
                println!("🌟 PRODUCTION READY");
            } else {
                println!("⚠️  FUNCTIONAL (needs optimization)");
            }
        } else {
            println!("❌ BOOTSTRAP TEST FAILED");
            if self.first_peer_connected_at.is_none() {
                println!("   Reason: No peers connected");
            }
            if self.routing_table_size < 10 {
                println!("   Reason: Insufficient DHT peers (< 10)");
            }
        }
        println!("{:=<60}\n", "");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("\n{:=<60}", "");
    println!("🚀 OTTER BOOTSTRAP REAL NETWORK TEST");
    println!("{:=<60}\n", "");

    let mut metrics = TestMetrics::new();

    // 1. Setup libp2p swarm
    println!("🔧 Setting up libp2p swarm...");
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = local_key.public().to_peer_id();
    println!("📝 Local PeerId: {}", local_peer_id);

    // Build transport with DNS resolver
    let tcp = tcp::tokio::Transport::default();
    let dns_tcp = TokioDnsConfig::system(tcp)
        .map_err(|e| format!("DNS config error: {}", e))?;
    let transport = dns_tcp
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(noise::Config::new(&local_key)?)
        .multiplex(yamux::Config::default())
        .boxed();

    // Build Kademlia behaviour
    let store = MemoryStore::new(local_peer_id);
    let mut cfg = kad::Config::default();
    cfg.set_query_timeout(Duration::from_secs(60));
    let mut kad = kad::Behaviour::with_config(local_peer_id, store, cfg);
    kad.set_mode(Some(kad::Mode::Server));

    // Build swarm
    let mut swarm = Swarm::new(
        transport,
        kad,
        local_peer_id,
        libp2p::swarm::Config::with_tokio_executor()
            .with_idle_connection_timeout(Duration::from_secs(30)),
    );

    // Listen on all interfaces
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    println!("✅ Swarm initialized\n");

    // 2. Initialize bootstrap sources
    println!("🌐 Initializing bootstrap sources...");
    let cache_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".otter")
        .join("peer_cache.json");

    println!("📁 Cache path: {}", cache_path.display());
    
    let mut bootstrap = BootstrapSources::new(cache_path);
    
    match bootstrap.initialize().await {
        Ok(_) => println!("✅ Bootstrap initialized"),
        Err(e) => println!("⚠️  Bootstrap init warning: {} (first run?)", e),
    }

    let cache_stats = bootstrap.cache_stats();
    println!("📊 Cache stats:");
    println!("   Total peers: {}", cache_stats.total_peers);
    println!("   Relay peers: {}", cache_stats.relay_peers);
    println!("   Avg reputation: {:.2}", cache_stats.average_reputation);

    // 3. Load cached peers (Tier 2)
    println!("\n🔍 TIER 2: Loading cached peers...");
    match bootstrap.load_cached_peers().await {
        Ok(cached) => {
            metrics.cache_loaded_peers = cached.len();
            println!("✅ Loaded {} peers from cache", cached.len());
            
            // Try connecting to cached peers
            for (i, addr) in cached.iter().take(5).enumerate() {
                println!("   [{}] Dialing cached: {}", i + 1, addr);
                if let Err(e) = swarm.dial(addr.clone()) {
                    println!("      ⚠️  Dial error: {}", e);
                } else {
                    metrics.total_connections_attempted += 1;
                }
            }
        }
        Err(e) => {
            println!("⚠️  No cached peers: {}", e);
        }
    }

    // 4. DNS Bootstrap (Tier 1)
    println!("\n🌍 TIER 1: DNS Bootstrap...");
    match bootstrap.resolve_dns_bootstrap().await {
        Ok(dns_peers) => {
            metrics.dns_resolved_peers = dns_peers.len();
            println!("✅ DNS resolved {} peers", dns_peers.len());
            
            // Try connecting to DNS peers
            for (i, addr) in dns_peers.iter().take(10).enumerate() {
                println!("   [{}] Dialing DNS peer: {}", i + 1, addr);
                if let Err(e) = swarm.dial(addr.clone()) {
                    println!("      ⚠️  Dial error: {}", e);
                } else {
                    metrics.total_connections_attempted += 1;
                }
            }
        }
        Err(e) => {
            println!("❌ DNS bootstrap failed: {}", e);
            println!("   This is critical - cannot discover any peers!");
            return Err(e.into());
        }
    }

    metrics.total_peers_discovered = metrics.cache_loaded_peers + metrics.dns_resolved_peers;

    // 5. Run event loop with timeout
    println!("\n⏳ Monitoring connections and DHT (timeout: {}s)...\n", TARGET_TIMEOUT_SECS);
    
    let timeout = time::sleep(Duration::from_secs(TARGET_TIMEOUT_SECS));
    tokio::pin!(timeout);

    let mut tick_interval = time::interval(Duration::from_secs(10));
    let mut last_rt_size = 0;

    loop {
        tokio::select! {
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("🎧 Listening on: {}", address);
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        metrics.total_connections_successful += 1;
                        
                        if metrics.first_peer_connected_at.is_none() {
                            metrics.first_peer_connected_at = Some(Instant::now());
                            let elapsed = metrics.time_to_first_peer().unwrap();
                            println!("\n🎉 FIRST PEER CONNECTED in {:.2}s!", elapsed.as_secs_f64());
                        }
                        
                        println!("✅ Connected to: {} ({})", peer_id, endpoint.get_remote_address());
                        
                        // Add to DHT routing table
                        swarm.behaviour_mut().add_address(&peer_id, endpoint.get_remote_address().clone());
                        
                        // Track success in bootstrap module
                        bootstrap.record_successful_dial(
                            &peer_id, 
                            endpoint.get_remote_address(), 
                            100 // Simplified latency
                        ).await;
                    }
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        println!("❌ Disconnected from: {} (cause: {:?})", peer_id, cause);
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                        if let Some(peer) = peer_id {
                            println!("⚠️  Connection failed to {}: {}", peer, error);
                            bootstrap.record_failed_dial(&peer).await;
                        }
                    }
                    SwarmEvent::Behaviour(kad::Event::RoutingUpdated { peer, .. }) => {
                        println!("📡 DHT routing updated: {} added", peer);
                    }
                    SwarmEvent::Behaviour(kad::Event::InboundRequest { request }) => {
                        println!("📥 Inbound DHT request: {:?}", request);
                    }
                    _ => {}
                }
            }
            
            _ = tick_interval.tick() => {
                // Periodic status update
                let current_rt_size = swarm.behaviour_mut().kbuckets().count();
                metrics.routing_table_size = current_rt_size;
                
                if current_rt_size != last_rt_size {
                    println!("\n📊 Status Update:");
                    println!("   Connected peers: {}", metrics.total_connections_successful);
                    println!("   DHT routing table: {} peers", current_rt_size);
                    println!("   Elapsed: {:.0}s", metrics.total_elapsed().as_secs_f64());
                    
                    last_rt_size = current_rt_size;
                    
                    // Check if we reached target early
                    if current_rt_size >= TARGET_ROUTING_TABLE_SIZE {
                        println!("\n🎯 Target routing table size reached!");
                        break;
                    }
                }
            }
            
            _ = &mut timeout => {
                println!("\n⏰ Timeout reached ({}s)", TARGET_TIMEOUT_SECS);
                break;
            }
        }
    }

    // Final routing table count
    metrics.routing_table_size = swarm.behaviour_mut().kbuckets().count();

    // Print metrics summary
    metrics.print_summary();

    // Save cache stats
    let final_stats = bootstrap.cache_stats();
    println!("💾 Final cache stats:");
    println!("   Total peers: {}", final_stats.total_peers);
    println!("   Relay peers: {}", final_stats.relay_peers);
    println!("   Avg reputation: {:.2}\n", final_stats.average_reputation);

    Ok(())
}
