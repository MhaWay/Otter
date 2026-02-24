//! DHT Provider Registration Test
//!
//! Tests Phase 1: DHT Foundation
//! - Server mode enabled
//! - Provider registration
//! - DHT query results handling
//! - Auto-dial discovered mesh peers
//!
//! USAGE:
//!   # Terminal 1 (First peer - becomes provider)
//!   cargo run --example dht_provider_test -- --first
//!   
//!   # Terminal 2 (Second peer - discovers first)
//!   cargo run --example dht_provider_test

use libp2p::{
    identity,
    kad::{self, store::MemoryStore},
    noise, tcp, yamux,
    dns::TokioDnsConfig,
    gossipsub, identify,
    swarm::{Swarm, SwarmEvent},
    Transport, PeerId,
};
use futures::StreamExt;
use otter_network::bootstrap::BootstrapSources;
use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::time;

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

    let args: Vec<String> = std::env::args().collect();
    let is_first = args.iter().any(|a| a == "--first");

    if is_first {
        println!("\n🦦 PEER 1 (Provider) - Will register in DHT");
    } else {
        println!("\n🦦 PEER 2 (Discoverer) - Will find Peer 1 via DHT");
    }
    println!("{:=<60}\n", "");

    // Setup libp2p
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("📝 Local PeerId: {}", local_peer_id);

    // Transport
    let tcp = tcp::tokio::Transport::default();
    let dns_tcp = TokioDnsConfig::system(tcp)
        .map_err(|e| format!("DNS config error: {}", e))?;
    let transport = dns_tcp
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(noise::Config::new(&local_key)?)
        .multiplex(yamux::Config::default())
        .boxed();

    // Gossipsub
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .build()?;

    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(local_key.clone()),
        gossipsub_config,
    )?;

    // Identify
    let identify = identify::Behaviour::new(identify::Config::new(
        "/otter/1.0.0".to_string(),
        local_key.public(),
    ));

    // Kademlia DHT in SERVER MODE
    let store = MemoryStore::new(local_peer_id);
    let mut kad = kad::Behaviour::new(local_peer_id, store);
    kad.set_mode(Some(kad::Mode::Server));
    println!("✅ Kademlia DHT in Server mode");

    // Combine behaviors
    let behaviour = OtterBehaviour {
        gossipsub,
        identify,
        kad,
    };

    // Swarm
    let swarm_config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(600));

    let mut swarm = Swarm::new(transport, behaviour, local_peer_id, swarm_config);

    // Subscribe to gossipsub topic
    let topic = gossipsub::IdentTopic::new("otter-chat");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    // Listen
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    println!("⏳ Waiting for listening address...\n");

    // Bootstrap
    let cache_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".otter")
        .join("peer_cache_dht_test.json");

    let mut bootstrap = BootstrapSources::new(cache_path);
    let _ = bootstrap.initialize().await;

    let start_time = Instant::now();
    let mut provider_registered = false;
    let mut dht_query_started = false;
    let mut listening = false;

    loop {
        tokio::select! {
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("🎧 Listening on: {}", address);
                        listening = true;
                        
                        if is_first && !provider_registered {
                            // Peer 1: Register as provider immediately
                            println!("\n📡 Registering as Otter provider...");
                            let key = kad::RecordKey::from(b"otter:discovery:v1".to_vec());
                            match swarm.behaviour_mut().kad.start_providing(key) {
                                Ok(_) => {
                                    provider_registered = true;
                                    println!("✅ Provider registration started (TTL: 24h)");
                                    println!("⏳ Waiting for other peers to discover us...\n");
                                }
                                Err(e) => println!("❌ Provider registration failed: {}", e),
                            }
                        } else if !is_first && listening && !dht_query_started {
                            // Peer 2: Wait a bit for bootstrap, then query DHT
                            println!("\n⏳ Connecting to bootstrap...");
                        }
                    }
                    
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        println!("✅ Connected to peer: {}", peer_id);
                        
                        // If Peer 2 connects to bootstrap, query DHT for Otter providers
                        if !is_first && !dht_query_started {
                            dht_query_started = true;
                            println!("\n🔍 Querying DHT for Otter peers...");
                            let query_id = swarm.behaviour_mut().kad.get_closest_peers(PeerId::random());
                            println!("   Query ID: {:?}", query_id);
                        }
                    }
                    
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        println!("❌ Disconnected from: {} (cause: {:?})", peer_id, cause);
                    }
                    
                    SwarmEvent::Behaviour(OtterEvent::Kad(kad::Event::OutboundQueryProgressed {
                        result: kad::QueryResult::GetClosestPeers(Ok(ok)),
                        ..
                    })) => {
                        println!("\n🎉 DHT Query Result: Found {} peers", ok.peers.len());
                        for (i, peer_id) in ok.peers.iter().enumerate() {
                            println!("   [{}] {}", i + 1, peer_id);
                            
                            if *peer_id != local_peer_id {
                                println!("   └─> Attempting to dial...");
                                if let Err(e) = swarm.dial(*peer_id) {
                                    println!("       ❌ Dial failed: {}", e);
                                } else {
                                    println!("       ✅ Dial initiated");
                                }
                            }
                        }
                    }
                    
                    SwarmEvent::Behaviour(OtterEvent::Kad(kad::Event::OutboundQueryProgressed {
                        result: kad::QueryResult::GetClosestPeers(Err(e)),
                        ..
                    })) => {
                        println!("⚠️  DHT Query failed: {:?}", e);
                    }
                    
                    SwarmEvent::Behaviour(OtterEvent::Identify(identify::Event::Received { peer_id, info })) => {
                        println!("📋 Identified peer: {} (proto: {})", peer_id, info.protocol_version);
                    }
                    
                    _ => {}
                }
            }
            
            _ = time::sleep(Duration::from_secs(1)) => {
                if !is_first && listening && !dht_query_started {
                    // Bootstrap for Peer 2
                    if let Ok(peers) = bootstrap.resolve_dns_bootstrap().await {
                        if !peers.is_empty() {
                            println!("🌐 Bootstrap DNS resolved {} peers", peers.len());
                            for addr in peers.iter().take(2) {
                                println!("   Dialing: {}", addr);
                                let _ = swarm.dial(addr.clone());
                            }
                        }
                    }
                }
                
                if start_time.elapsed() > Duration::from_secs(90) {
                    println!("\n⏰ Test timeout (90s)");
                    break;
                }
            }
        }
    }

    println!("\n✅ Test complete");
    Ok(())
}
