//! Local Peer Discovery Test
//!
//! Testa la scoperta di peer Otter sulla rete locale tramite mDNS.
//! Avvia 2 peer Otter locali e verifica che si scoprano reciprocamente.
//!
//! OBIETTIVI:
//! 1. ✅ Avvio di 2 peer locali
//! 2. ✅ Scoperta tramite mDNS
//! 3. ✅ Connessione P2P stabilita
//! 4. ✅ Scambio messaggi Identity
//!
//! USAGE:
//!   cargo run --example local_peer_discovery_test --release

use otter_identity::Identity;
use otter_messaging::MessageHandler;
use otter_network::{create_network_channels, Network, NetworkCommand, NetworkEvent};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::time;
use tracing::{error, info, warn};

#[derive(Debug)]
struct PeerMetrics {
    peer_id: String,
    nickname: String,
    discovered_peers: usize,
    connected_peers: usize,
    identity_received: usize,
}

impl PeerMetrics {
    fn new(peer_id: String, nickname: String) -> Self {
        Self {
            peer_id,
            nickname,
            discovered_peers: 0,
            connected_peers: 0,
            identity_received: 0,
        }
    }

    fn print_summary(&self) {
        println!("\n📊 PEER: {}", self.nickname);
        println!("   ID: {}", self.peer_id);
        println!("   Discovered: {} peers", self.discovered_peers);
        println!("   Connected:  {} peers", self.connected_peers);
        println!("   Identities: {} received", self.identity_received);
    }
}

async fn spawn_peer(nickname: String, port: u16) -> Result<(Arc<Mutex<PeerMetrics>>, mpsc::Sender<NetworkCommand>), Box<dyn std::error::Error>> {
    // Generate identity
    let identity = Identity::generate()?;
    let peer_id = identity.peer_id().to_string();
    
    info!("🦦 Spawning peer '{}' ({})", nickname, peer_id);
    
    // Create network channels
    let (event_tx, mut event_rx, command_tx, command_rx) = create_network_channels();
    
    // Create network
    let mut network = Network::new(event_tx, command_rx)?;
    
    // Start listening
    let listen_addr = format!("/ip4/0.0.0.0/tcp/{}", port);
    network.listen(&listen_addr)?;
    
    // Create message handler
    let message_handler = Arc::new(Mutex::new(MessageHandler::new(identity.clone())));
    
    // Create metrics
    let metrics = Arc::new(Mutex::new(PeerMetrics::new(peer_id.clone(), nickname.clone())));
    let metrics_clone = metrics.clone();
    
    // Clone nickname for tasks
    let nickname_network = nickname.clone();
    
    // Spawn network task
    tokio::spawn(async move {
        if let Err(e) = network.run().await {
            error!("Network error for {}: {}", nickname_network, e);
        }
    });
    
    // Spawn event handler
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                NetworkEvent::ListeningOn { address } => {
                    info!("Peer '{}' listening on: {}", nickname, address);
                }
                
                NetworkEvent::PeerDiscovered { peer_id, addresses } => {
                    info!("Peer '{}' discovered: {} at {:?}", nickname, peer_id, addresses);
                    metrics_clone.lock().await.discovered_peers += 1;
                }
                
                NetworkEvent::PeerOnline { peer_id, nickname: peer_nick, .. } => {
                    info!("Peer '{}' sees peer online: {} ({})", 
                        nickname, peer_id, peer_nick.unwrap_or_default());
                    metrics_clone.lock().await.connected_peers += 1;
                }
                
                NetworkEvent::MessageReceived { from, data } => {
                    if let Ok(message) = otter_messaging::Message::from_bytes(&data) {
                        if let otter_messaging::Message::Identity { public_identity, .. } = message {
                            let received_peer_id = public_identity.peer_id().to_string();
                            info!("Peer '{}' received identity from: {}", nickname, received_peer_id);
                            
                            let mut handler = message_handler.lock().await;
                            if let Err(e) = handler.register_peer(public_identity) {
                                warn!("Failed to register peer: {}", e);
                            } else {
                                metrics_clone.lock().await.identity_received += 1;
                            }
                        }
                    } else {
                        warn!("Peer '{}' received invalid message from {}", nickname, from);
                    }
                }
                
                NetworkEvent::PeerOffline { peer_id } => {
                    info!("Peer '{}' sees peer offline: {}", nickname, peer_id);
                }
                
                NetworkEvent::DiscoveringPeers { connected_count } => {
                    info!("Peer '{}' discovering peers (connected: {})", nickname, connected_count);
                }
                
                _ => {}
            }
        }
    });
    
    Ok((metrics, command_tx))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("info,libp2p=warn,libp2p_mdns=info")
    ).init();
    
    println!("\n{:=<60}", "");
    println!("🚀 OTTER LOCAL PEER DISCOVERY TEST");
    println!("{:=<60}", "");
    println!();
    
    // Spawn 2 local peers
    println!("🔧 Spawning peer 1...");
    let (peer1_metrics, _peer1_cmd) = spawn_peer("Peer1".to_string(), 0).await?;
    
    println!("🔧 Spawning peer 2...");
    let (peer2_metrics, _peer2_cmd) = spawn_peer("Peer2".to_string(), 0).await?;
    
    // Wait for peers to initialize
    println!("\n⏳ Waiting for peers to initialize (2s)...");
    time::sleep(Duration::from_secs(2)).await;
    
    // Wait for mDNS discovery (can take 5-10 seconds)
    println!("🔍 Waiting for mDNS discovery (20s)...");
    println!("   (mDNS broadcasts every 5 seconds)\n");
    
    for i in 1..=20 {
        time::sleep(Duration::from_secs(1)).await;
        
        let p1 = peer1_metrics.lock().await;
        let p2 = peer2_metrics.lock().await;
        
        if i % 5 == 0 {
            println!("   [{}s] Peer1: {} discovered, {} connected | Peer2: {} discovered, {} connected",
                i, p1.discovered_peers, p1.connected_peers, 
                p2.discovered_peers, p2.connected_peers);
        }
        
        // Check if both peers discovered each other
        if p1.discovered_peers > 0 && p2.discovered_peers > 0 {
            println!("\n✅ SUCCESS: Peers discovered each other!");
            break;
        }
    }
    
    // Wait a bit more for identity exchange
    println!("\n⏳ Waiting for identity exchange (5s)...");
    time::sleep(Duration::from_secs(5)).await;
    
    // Print final metrics
    println!("\n{:=<60}", "");
    println!("📊 FINAL RESULTS");
    println!("{:=<60}", "");
    
    let p1 = peer1_metrics.lock().await;
    let p2 = peer2_metrics.lock().await;
    
    p1.print_summary();
    p2.print_summary();
    
    println!("\n{:=<60}", "");
    
    // Evaluate results
    let success = p1.discovered_peers > 0 && p2.discovered_peers > 0 
                  && p1.connected_peers > 0 && p2.connected_peers > 0;
    
    if success {
        println!("✅ LOCAL PEER DISCOVERY TEST PASSED");
        println!("   - mDNS discovery: WORKING");
        println!("   - P2P connection: WORKING");
        
        if p1.identity_received > 0 && p2.identity_received > 0 {
            println!("   - Identity exchange: WORKING");
            println!("\n🌟 READY FOR ANDROID DEPLOYMENT");
        } else {
            println!("   - Identity exchange: PARTIAL");
            println!("\n⚠️  FUNCTIONAL (identity sync needs work)");
        }
    } else {
        println!("❌ LOCAL PEER DISCOVERY TEST FAILED");
        
        if p1.discovered_peers == 0 && p2.discovered_peers == 0 {
            println!("   Reason: No mDNS discovery occurred");
            println!("   Check: Firewall blocking port 5353 (mDNS)?");
        } else if p1.connected_peers == 0 || p2.connected_peers == 0 {
            println!("   Reason: Discovery OK but connection failed");
            println!("   Check: Network transport issues?");
        }
    }
    
    println!("{:=<60}", "");
    println!();
    
    // Keep running for a bit to observe behavior
    println!("Keeping peers alive for 10 more seconds...");
    time::sleep(Duration::from_secs(10)).await;
    
    Ok(())
}
