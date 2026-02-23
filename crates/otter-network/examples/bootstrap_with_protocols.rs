//! Bootstrap Test WITH Protocols (Gossipsub + Identify)
//!
//! Questo test simula la configurazione REALE della GUI per debug
//! 
//! USAGE:
//!   cargo run --example bootstrap_with_protocols

use libp2p::{
    identity,
    kad::{self, store::MemoryStore},
    noise, tcp, yamux,
    dns::TokioDnsConfig,
    gossipsub, identify,
    swarm::{Swarm, SwarmEvent},
    Transport,
};
use futures::StreamExt;
use otter_network::bootstrap::BootstrapSources;
use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::time;

#[derive(Debug)]
struct TestMetrics {
    start_time: Instant,
    first_peer_connected_at: Option<Instant>,
    total_peers_discovered: usize,
    total_connections_attempted: usize,
    total_connections_successful: usize,
    stable_connections: usize,  // Connections that stayed connected after 30s
    routing_table_size: usize,
}

impl TestMetrics {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            first_peer_connected_at: None,
            total_peers_discovered: 0,
            total_connections_attempted: 0,
            total_connections_successful: 0,
            stable_connections: 0,
            routing_table_size: 0,
        }
    }

    fn total_elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    fn print_summary(&self) {
        println!("\n{:=<60}", "");
        println!("📊 BOOTSTRAP WITH PROTOCOLS TEST SUMMARY");
        println!("{:=<60}", "");
        
        println!("\n✅ RESULTS:");
        println!("  Connected: {}/{} peers", self.total_connections_successful, self.total_connections_attempted);
        println!("  Stable after 30s: {} peers", self.stable_connections);
        println!("  DHT routing table: {} peers", self.routing_table_size);
        println!("  Total elapsed: {:.1}s", self.total_elapsed().as_secs_f64());
        
        let stable = self.stable_connections > 0;
        let has_dht = self.routing_table_size > 5;
        
        if stable && has_dht {
            println!("\n✅ BOOTSTRAP SUCCESSFUL (Protocols working!)");
        } else if self.total_connections_successful > 0 {
            println!("\n⚠️  PARTIAL SUCCESS");
            if !stable {
                println!("   Issue: Connections not stable (disconnecting)");
            }
            if !has_dht {
                println!("   Issue: DHT not growing");
            }
        } else {
            println!("\n❌ BOOTSTRAP FAILED");
        }
        println!("{:=<60}\n", "");
    }
}

#[derive(libp2p::swarm::NetworkBehaviour)]
#[behaviour(out_event = "OtterEvent")]
struct OtterBehaviour {
    gossipsub: gossipsub::Behaviour,
    identify: identify::Behaviour,
    kad: kad::Behaviour<MemoryStore>,
}

#[derive(Debug)]
enum OtterEvent {
    Gossipsub(gossipsub::Event),
    Identify(identify::Event),
    Kad(kad::Event),
}

impl From<gossipsub::Event> for OtterEvent {
    fn from(e: gossipsub::Event) -> Self {
        OtterEvent::Gossipsub(e)
    }
}

impl From<identify::Event> for OtterEvent {
    fn from(e: identify::Event) -> Self {
        OtterEvent::Identify(e)
    }
}

impl From<kad::Event> for OtterEvent {
    fn from(e: kad::Event) -> Self {
        OtterEvent::Kad(e)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("\n{:=<60}", "");
    println!("🚀 BOOTSTRAP WITH PROTOCOLS TEST");
    println!("   (Simula configurazione GUI reale)");
    println!("{:=<60}\n", "");

    let mut metrics = TestMetrics::new();

    // 1. Setup libp2p swarm COME NELLA GUI
    println!("🔧 Setting up libp2p swarm with Gossipsub + Identify...");
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = libp2p::PeerId::from(local_key.public());
    println!("📝 Local PeerId: {}", local_peer_id);

    // Build transport
    let tcp = tcp::tokio::Transport::default();
    let dns_tcp = TokioDnsConfig::system(tcp)
        .map_err(|e| format!("DNS config error: {}", e))?;
    let transport = dns_tcp
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(noise::Config::new(&local_key)?)
        .multiplex(yamux::Config::default())
        .boxed();

    // Gossipsub (come nella GUI)
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .build()?;

    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(local_key.clone()),
        gossipsub_config,
    )?;

    // Identify (come nella GUI)
    let identify = identify::Behaviour::new(identify::Config::new(
        "/otter/1.0.0".to_string(),
        local_key.public(),
    ));

    // Kademlia DHT
    let store = MemoryStore::new(local_peer_id);
    let kad = kad::Behaviour::new(local_peer_id, store);

    // Combine behaviors
    let behaviour = OtterBehaviour {
        gossipsub,
        identify,
        kad,
    };

    // Swarm con 10 minuti idle timeout (come nella GUI)
    let swarm_config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(600));

    let mut swarm = Swarm::new(transport, behaviour, local_peer_id, swarm_config);

    // Subscribe to gossipsub topic (come nella GUI)
    let topic = gossipsub::IdentTopic::new("otter-chat");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    // Listen
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    println!("✅ Swarm initialized with protocols\n");

    // 2. Bootstrap
    println!("🌐 Initializing bootstrap...");
    let cache_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".otter")
        .join("peer_cache_test.json");

    let mut bootstrap = BootstrapSources::new(cache_path);
    let _ = bootstrap.initialize().await;

    // Resolve DNS bootstrap
    println!("\n🌍 Resolving DNS bootstrap...");
    match bootstrap.resolve_dns_bootstrap().await {
        Ok(peers) => {
            metrics.total_peers_discovered = peers.len();
            println!("✅ Found {} peers", peers.len());
            
            for (i, addr) in peers.iter().take(5).enumerate() {
                println!("   [{}] Dialing: {}", i + 1, addr);
                if let Err(e) = swarm.dial(addr.clone()) {
                    println!("      ⚠️  Error: {}", e);
                } else {
                    metrics.total_connections_attempted += 1;
                }
            }
        }
        Err(e) => {
            println!("❌ DNS bootstrap failed: {}", e);
            return Err(e.into());
        }
    }

    // 3. Run event loop for 90 seconds
    println!("\n⏳ Monitoring connections (timeout: 90s)...\n");
    
    let timeout = time::sleep(Duration::from_secs(90));
    tokio::pin!(timeout);

    let mut tick_interval = time::interval(Duration::from_secs(15));
    let connection_check_time = Instant::now() + Duration::from_secs(30);

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
                        }
                        
                        println!("✅ Connected to: {} ({})", peer_id, endpoint.get_remote_address());
                        
                        // Add to DHT
                        swarm.behaviour_mut().kad.add_address(&peer_id, endpoint.get_remote_address().clone());
                        
                        // Se passati 30 secondi, conta come stabile
                        if Instant::now() > connection_check_time {
                            metrics.stable_connections += 1;
                        }
                    }
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        println!("❌ Disconnected: {} (cause: {:?})", peer_id, cause);
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                        if let Some(peer) = peer_id {
                            println!("⚠️  Connection error for {}: {}", peer, error);
                        }
                    }
                    SwarmEvent::Behaviour(OtterEvent::Identify(identify::Event::Received { peer_id, info })) => {
                        println!("📋 Identified peer: {} (proto: {})", peer_id, info.protocol_version);
                    }
                    SwarmEvent::Behaviour(OtterEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, topic })) => {
                        println!("🔔 Peer {} subscribed to {}", peer_id, topic);
                    }
                    SwarmEvent::Behaviour(OtterEvent::Kad(kad::Event::RoutingUpdated { peer, .. })) => {
                        println!("📡 DHT updated: {}", peer);
                    }
                    _ => {}
                }
            }
            
            _ = tick_interval.tick() => {
                let current_rt_size = swarm.behaviour_mut().kad.kbuckets().count();
                metrics.routing_table_size = current_rt_size;
                
                println!("\n📊 Status ({:.0}s): {} connections, {} DHT peers",
                    metrics.total_elapsed().as_secs_f64(),
                    metrics.total_connections_successful,
                    current_rt_size
                );
            }
            
            _ = &mut timeout => {
                println!("\n⏰ Timeout reached");
                break;
            }
        }
    }

    metrics.print_summary();
    Ok(())
}
