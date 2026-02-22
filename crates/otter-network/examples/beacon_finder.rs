// Headless beacon finder - finds peers on the network without GUI
// Non necessita di GUI, funziona su server/headless systems

use futures::prelude::*;
use libp2p::{
    core::transport::upgrade,
    Transport,
    gossipsub, identify, kad, mdns, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, PeerId, Swarm,
};
use std::time::Duration;

#[derive(NetworkBehaviour)]
struct FinderBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
    kad: kad::Behaviour<kad::store::MemoryStore>,
    identify: identify::Behaviour,
}

#[tokio::main]
async fn main() {
    println!("🔍 Headless Beacon Finder - Looking for 'hana' (34260f86)");
    println!("────────────────────────────────────────────────────────\n");

    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    println!("Local peer ID: {}", local_peer_id);

    // Create basic transport
    let tcp_transport = tcp::tokio::Transport::default();
    let noise_config = match noise::Config::new(&local_key) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };
    let yamux_config = yamux::Config::default();

    let transport = tcp_transport
        .upgrade(upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(yamux_config)
        .boxed();

    // Create behaviors
    let gossiop_config = match gossipsub::ConfigBuilder::default()
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error creating gossipsub config: {}", e);
            return;
        }
    };

    let gossipsub = match gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(local_key.clone()),
        gossiop_config,
    ) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Error creating gossipsub: {}", e);
            return;
        }
    };

    let store = kad::store::MemoryStore::new(local_peer_id);
    let kad_behaviour = kad::Behaviour::with_config(local_peer_id, store, kad::Config::default());

    let mdns = match mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error creating mDNS: {}", e);
            return;
        }
    };

    let identify = identify::Behaviour::new(identify::Config::new(
        "/ipfs/0.1.0".into(),
        local_key.public(),
    ));

    let behaviour = FinderBehaviour {
        gossipsub,
        mdns,
        kad: kad_behaviour,
        identify,
    };

    let mut swarm = Swarm::new(
        transport,
        behaviour,
        local_peer_id,
        libp2p::swarm::Config::with_tokio_executor(),
    );

    // Listen on TCP
    if let Err(e) = swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap()) {
        eprintln!("Error listening: {}", e);
        return;
    }

    // Subscribe to topic
    let topic = gossipsub::IdentTopic::new("otter-peers");
    if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&topic) {
        eprintln!("Error subscribing: {}", e);
        return;
    }

    println!("✓ Subscribed to gossipsub topic: {}\n", topic);
    println!("⏳ Scanning for beacons for 30 seconds...\n");

    let mut found_hana = false;
    let mut beacon_count = 0;
    let start = std::time::Instant::now();

    while start.elapsed() < Duration::from_secs(30) {
        match swarm.select_next_some().await {
            SwarmEvent::Behaviour(FinderBehaviourEvent::Gossipsub(
                gossipsub::Event::Message {
                    message,
                    ..
                }
            )) => {
                if let Ok(msg_str) = std::str::from_utf8(&message.data) {
                    if msg_str.starts_with("BEACON:") {
                        beacon_count += 1;
                        let parts: Vec<&str> =
                            msg_str.strip_prefix("BEACON:").unwrap().splitn(2, ':').collect();

                        if parts.len() == 2 {
                            let peer_id = parts[0];
                            let nickname = parts[1];

                            if nickname == "hana" || peer_id.contains("34260f86") {
                                println!(
                                    "🎉 FOUND! Peer: {} ({})",
                                    nickname, peer_id
                                );
                                found_hana = true;
                            } else {
                                println!(
                                    "  Beacon #{}: {} ({})",
                                    beacon_count, nickname, peer_id
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    println!("\n════════════════════════════════════════════════════════════");
    if found_hana {
        println!("✅ SUCCESS! Found peer 'hana' on the network!");
    } else if beacon_count > 0 {
        println!("⚠️  Found {} beacons but NOT 'hana'", beacon_count);
        println!("Is your friend's app running?");
    } else {
        println!("❌ No beacons found on the network");
        println!("Possible issues:");
        println!("  - Bootstrap nodes not reachable");
        println!("  - Friend's app not running");
        println!("  - Network connectivity issue");
    }
    println!("════════════════════════════════════════════════════════════");
}
