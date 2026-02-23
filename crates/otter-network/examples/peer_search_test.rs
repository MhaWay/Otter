// Peer Search Real Network Test
//
// Usage:
//   cargo run --example peer_search_test -- <peer_id_or_otter_id_or_nickname>
//
// This test bootstraps to the public libp2p network, dials bootstrap peers,
// runs a DHT closest-peers query for libp2p PeerIds, and listens for Otter
// Identity messages on gossipsub to match by Otter PeerId or nickname.

use libp2p::{
    dns::TokioDnsConfig,
    gossipsub,
    identity,
    kad::{self, store::MemoryStore, QueryResult},
    multiaddr::Protocol,
    noise,
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
    tcp, yamux,
    Multiaddr, PeerId, Transport,
};
use futures::StreamExt;
use otter_messaging::Message as OtterMessage;
use otter_network::bootstrap::BootstrapSources;
use std::{
    collections::HashSet,
    env,
    error::Error,
    path::PathBuf,
    str::FromStr,
    time::{Duration, Instant},
};

const TARGET_TIMEOUT_SECS: u64 = 120;

#[derive(NetworkBehaviour)]
struct TestBehaviour {
    kad: kad::Behaviour<MemoryStore>,
    gossipsub: gossipsub::Behaviour,
}

fn extract_peer_id_from_multiaddr(addr: &Multiaddr) -> Option<PeerId> {
    addr.iter().find_map(|proto| match proto {
        Protocol::P2p(peer_id) => Some(peer_id),
        _ => None,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: peer_search_test <peer_id_or_otter_id_or_nickname>");
        std::process::exit(1);
    }

    let target_arg = args[1].clone();
    let target_libp2p_peer_id = PeerId::from_str(&target_arg).ok();
    let target_otter_id = if target_libp2p_peer_id.is_none() {
        Some(target_arg.clone())
    } else {
        None
    };
    let target_nickname = Some(target_arg.clone()); // Always try nickname match

    if let Some(peer_id) = &target_libp2p_peer_id {
        println!("Target libp2p PeerId: {}", peer_id);
    } else if let Some(otter_id) = &target_otter_id {
        println!("Target Otter PeerId or Nickname: {}", otter_id);
        println!("Note: will match on identity messages (not DHT)");
    }

    // Setup libp2p swarm
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = local_key.public().to_peer_id();
    println!("Local PeerId: {}", local_peer_id);

    let tcp_transport = tcp::tokio::Transport::default();
    let dns_tcp = TokioDnsConfig::system(tcp_transport)
        .map_err(|e| format!("DNS config error: {}", e))?;
    let transport = dns_tcp
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(noise::Config::new(&local_key)?)
        .multiplex(yamux::Config::default())
        .boxed();

    let store = MemoryStore::new(local_peer_id);
    let mut cfg = kad::Config::default();
    cfg.set_query_timeout(Duration::from_secs(60));
    let mut kad = kad::Behaviour::with_config(local_peer_id, store, cfg);
    kad.set_mode(Some(kad::Mode::Server));

    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .validation_mode(gossipsub::ValidationMode::Strict)
        .build()
        .map_err(|e| format!("gossipsub config error: {}", e))?;
    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(local_key.clone()),
        gossipsub_config,
    )
    .map_err(|e| format!("gossipsub init error: {}", e))?;

    let behaviour = TestBehaviour { kad, gossipsub };

    let mut swarm = Swarm::new(
        transport,
        behaviour,
        local_peer_id,
        libp2p::swarm::Config::with_tokio_executor()
            .with_idle_connection_timeout(Duration::from_secs(30)),
    );

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let gossipsub_topic = gossipsub::IdentTopic::new("otter-chat");
    swarm
        .behaviour_mut()
        .gossipsub
        .subscribe(&gossipsub_topic)
        .map_err(|e| format!("gossipsub subscribe error: {}", e))?;

    // Initialize bootstrap sources
    let cache_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".otter")
        .join("peer_cache.json");
    let mut bootstrap = BootstrapSources::new(cache_path);
    let _ = bootstrap.initialize().await;
    let peers = bootstrap.bootstrap().await;

    println!("Bootstrap peers discovered: {}", peers.len());

    for addr in &peers {
        if let Some(peer_id) = extract_peer_id_from_multiaddr(addr) {
            swarm.behaviour_mut().kad.add_address(&peer_id, addr.clone());
        }

        if let Err(e) = swarm.dial(addr.clone()) {
            eprintln!("Dial failed for {}: {}", addr, e);
        }
    }

    // Start DHT query (target if libp2p PeerId, otherwise local peer)
    let query_target = target_libp2p_peer_id.clone().unwrap_or(local_peer_id);
    let query_id = swarm.behaviour_mut().kad.get_closest_peers(query_target);
    let mut connected_peers: HashSet<PeerId> = HashSet::new();
    let started = Instant::now();

    println!("Searching DHT (closest peers query started)...");

    loop {
        if started.elapsed().as_secs() > TARGET_TIMEOUT_SECS {
            println!("Timeout: target not found within {}s", TARGET_TIMEOUT_SECS);
            break;
        }

        match swarm.next().await {
            Some(SwarmEvent::ConnectionEstablished { peer_id, .. }) => {
                connected_peers.insert(peer_id);
                if let Some(target) = &target_libp2p_peer_id {
                    if &peer_id == target {
                        println!("Found target via direct connection: {}", peer_id);
                        break;
                    }
                }
            }
            Some(SwarmEvent::Behaviour(TestBehaviourEvent::Kad(kad_event))) => {
                if let kad::Event::OutboundQueryProgressed { id, result, .. } = kad_event {
                    if id == query_id {
                        match result {
                            QueryResult::GetClosestPeers(Ok(ok)) => {
                                if let Some(target) = &target_libp2p_peer_id {
                                    let mut found = false;
                                    for p in &ok.peers {
                                        if p == target {
                                            found = true;
                                            break;
                                        }
                                    }

                                    if found {
                                        println!("Found target in DHT closest peers: {}", target);
                                        break;
                                    } else {
                                        println!("Target not in closest peers ({} peers returned)", ok.peers.len());
                                    }
                                } else {
                                    println!("DHT query returned {} peers", ok.peers.len());
                                }
                            }
                            QueryResult::GetClosestPeers(Err(err)) => {
                                println!("DHT query error: {:?}", err);
                            }
                            _ => {}
                        }
                    }
                }
            }
            Some(SwarmEvent::Behaviour(TestBehaviourEvent::Gossipsub(
                gossipsub::Event::Message { message, .. }
            ))) => {
                if let Ok(parsed) = OtterMessage::from_bytes(&message.data) {
                    if let OtterMessage::Identity { public_identity, .. } = parsed {
                        let otter_peer_id = public_identity.peer_id().to_string();
                        let nickname = public_identity.nickname().unwrap_or("<no nickname>");
                        println!("Identity received: {} (nickname: {})", otter_peer_id, nickname);

                        // Check if matches by Otter PeerId
                        if let Some(target) = &target_otter_id {
                            if &otter_peer_id == target {
                                println!("✅ Found target Otter PeerId via identity: {}", target);
                                break;
                            }
                        }
                        
                        // Check if matches by nickname
                        if let Some(target_nick) = &target_nickname {
                            if let Some(nick) = public_identity.nickname() {
                                if nick.eq_ignore_ascii_case(target_nick) {
                                    println!("✅ Found target by nickname '{}': peer_id={}", nick, otter_peer_id);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Some(_) => {}
            None => {}
        }
    }

    println!("Connected peers observed: {}", connected_peers.len());

    Ok(())
}
