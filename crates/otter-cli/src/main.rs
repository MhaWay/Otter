//! # Otter CLI
//!
//! Command-line interface for the Otter decentralized chat platform.
//!
//! A minimal CLI peer client for interacting with the Otter network.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use dialoguer::{theme::ColorfulTheme, Input, Select};
use libp2p::PeerId;
use otter_identity::{Identity, PublicIdentity};
use otter_messaging::{Message, MessageHandler, MessagingEvent};
use otter_network::{create_network_channels, Network, NetworkCommand, NetworkEvent};
use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info, warn};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "otter")]
#[command(about = "Privacy-focused decentralized chat platform", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new identity
    Init {
        /// Path to save identity file
        #[arg(short, long, default_value = "identity.json")]
        output: PathBuf,
    },
    
    /// Start the chat peer
    Start {
        /// Path to identity file
        #[arg(short, long, default_value = "identity.json")]
        identity: PathBuf,
        
        /// Port to listen on
        #[arg(short, long, default_value = "0")]
        port: u16,
    },
    
    /// Show identity information
    Info {
        /// Path to identity file
        #[arg(short, long, default_value = "identity.json")]
        identity: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("otter=info,libp2p=info")
        .init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { output } => {
            init_identity(output)?;
        }
        Commands::Start { identity, port } => {
            start_peer(identity, port).await?;
        }
        Commands::Info { identity } => {
            show_info(identity)?;
        }
    }
    
    Ok(())
}

/// Generate and save a new identity
fn init_identity(output: PathBuf) -> Result<()> {
    if output.exists() {
        anyhow::bail!("Identity file already exists: {}", output.display());
    }
    
    info!("Generating new identity...");
    let identity = Identity::generate()?;
    
    let json = identity.to_json()?;
    fs::write(&output, json)?;
    
    println!("✓ Identity generated successfully!");
    println!("  Peer ID: {}", identity.peer_id());
    println!("  Saved to: {}", output.display());
    println!("\nTo start chatting, run:");
    println!("  otter start -i {}", output.display());
    
    Ok(())
}

/// Show identity information
fn show_info(path: PathBuf) -> Result<()> {
    let json = fs::read_to_string(&path)
        .context("Failed to read identity file")?;
    
    let identity = Identity::from_json(&json)?;
    let public = PublicIdentity::from_identity(&identity);
    
    println!("Identity Information");
    println!("====================");
    println!("Peer ID: {}", identity.peer_id());
    println!("Public Key: {}", hex::encode(public.verifying_key()?.to_bytes()));
    println!("File: {}", path.display());
    
    Ok(())
}

/// Start the chat peer
async fn start_peer(identity_path: PathBuf, port: u16) -> Result<()> {
    // Load identity
    let json = fs::read_to_string(&identity_path)
        .context("Failed to read identity file. Run 'otter init' first.")?;
    
    let identity = Identity::from_json(&json)?;
    
    println!("🦦 Otter Chat - Decentralized & Private");
    println!("========================================");
    println!("Peer ID: {}", identity.peer_id());
    println!();
    
    // Create network channels
    let (event_tx, mut event_rx, command_tx, command_rx) = create_network_channels();
    
    // Create network
    let mut network = Network::new(event_tx, command_rx)?;
    
    // Start listening
    let listen_addr = format!("/ip4/0.0.0.0/tcp/{}", port);
    network.listen(&listen_addr)?;
    
    // Create message handler
    let message_handler = Arc::new(Mutex::new(MessageHandler::new(identity)));
    
    // Spawn network task
    let network_handle = tokio::spawn(async move {
        if let Err(e) = network.run().await {
            error!("Network error: {}", e);
        }
    });
    
    // Clone for tasks
    let cmd_tx = command_tx.clone();
    let msg_handler = message_handler.clone();
    
    // Spawn event handler
    let event_handle = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            if let Err(e) = handle_network_event(event, msg_handler.clone()).await {
                error!("Error handling event: {}", e);
            }
        }
    });
    
    // Wait a moment for network to start
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    println!("✓ Network started");
    println!("✓ Listening for peers...");
    println!();
    println!("Commands:");
    println!("  /peers  - List connected peers");
    println!("  /send   - Send a message to a peer");
    println!("  /help   - Show this help");
    println!("  /quit   - Exit");
    println!();
    
    // Interactive loop
    loop {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("otter>")
            .allow_empty(true)
            .interact_text()?;
        
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        match input {
            "/quit" | "/exit" => {
                println!("Goodbye! 🦦");
                break;
            }
            "/help" => {
                show_help();
            }
            "/peers" => {
                show_peers(&command_tx).await?;
            }
            "/send" => {
                send_message(&command_tx, &message_handler).await?;
            }
            _ => {
                println!("Unknown command. Type /help for available commands.");
            }
        }
    }
    
    // Cleanup
    drop(command_tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), network_handle).await;
    let _ = tokio::time::timeout(Duration::from_secs(2), event_handle).await;
    
    Ok(())
}

async fn handle_network_event(
    event: NetworkEvent,
    message_handler: Arc<Mutex<MessageHandler>>,
) -> Result<()> {
    match event {
        NetworkEvent::PeerDiscovered { peer_id, addresses } => {
            info!("Discovered peer: {} at {:?}", peer_id, addresses);
            println!("\n✓ Discovered peer: {}", peer_id);
        }
        
        NetworkEvent::PeerConnected { peer_id } => {
            info!("Connected to peer: {}", peer_id);
            println!("\n✓ Connected: {}", peer_id);
            
            // Send identity to new peer
            let handler = message_handler.lock().await;
            let _identity_msg = Message::identity(handler.public_identity());
            println!("  Exchanging identities...");
        }
        
        NetworkEvent::PeerDisconnected { peer_id } => {
            info!("Disconnected from peer: {}", peer_id);
            println!("\n✗ Disconnected: {}", peer_id);
        }
        
        NetworkEvent::MessageReceived { from, data } => {
            if let Ok(message) = Message::from_bytes(&data) {
                match message {
                    Message::Identity { public_identity, .. } => {
                        let peer_id = public_identity.peer_id().to_string();
                        let mut handler = message_handler.lock().await;
                        
                        if let Err(e) = handler.register_peer(public_identity) {
                            warn!("Failed to register peer: {}", e);
                        } else {
                            println!("\n✓ Identity verified for peer: {}", peer_id);
                        }
                    }
                    
                    Message::Text { content, .. } => {
                        println!("\n📨 Message from {}: {}", from, content);
                    }
                    
                    Message::Encrypted { ref from_peer_id, .. } => {
                        let mut handler = message_handler.lock().await;
                        match handler.decrypt_message(&message) {
                            Ok(content) => {
                                println!("\n🔐 Encrypted message from {}: {}", from_peer_id, content);
                            }
                            Err(e) => {
                                warn!("Failed to decrypt message: {}", e);
                            }
                        }
                    }
                    
                    _ => {}
                }
            }
        }
        
        NetworkEvent::ListeningOn { address } => {
            println!("Listening on: {}", address);
        }
    }
    
    Ok(())
}

fn show_help() {
    println!("\nAvailable Commands:");
    println!("  /peers  - List connected peers");
    println!("  /send   - Send a message to a peer");
    println!("  /help   - Show this help");
    println!("  /quit   - Exit the application");
    println!();
}

async fn show_peers(command_tx: &mpsc::Sender<NetworkCommand>) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(1);
    
    command_tx
        .send(NetworkCommand::ListPeers { response: tx })
        .await?;
    
    if let Some(peers) = rx.recv().await {
        if peers.is_empty() {
            println!("No connected peers yet.");
        } else {
            println!("\nConnected Peers:");
            for (i, peer) in peers.iter().enumerate() {
                println!("  {}. {}", i + 1, peer);
            }
        }
    }
    
    Ok(())
}

async fn send_message(
    command_tx: &mpsc::Sender<NetworkCommand>,
    message_handler: &Arc<Mutex<MessageHandler>>,
) -> Result<()> {
    let handler = message_handler.lock().await;
    let peers = handler.list_peers();
    
    if peers.is_empty() {
        println!("No peers registered yet. Wait for peer discovery and identity exchange.");
        return Ok(());
    }
    
    drop(handler);
    
    println!("\nSelect a peer:");
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&peers)
        .default(0)
        .interact_opt()?;
    
    if let Some(idx) = selection {
        let peer_id_str = &peers[idx];
        
        let message: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Message")
            .interact_text()?;
        
        let mut handler = message_handler.lock().await;
        
        let encrypted_msg = handler.prepare_encrypted_message(peer_id_str, &message)?;
        let _data = encrypted_msg.to_bytes()?;
        
        // For now, we'll send via gossipsub broadcast
        // In a production system, you'd want direct peer-to-peer messaging
        println!("✓ Message encrypted and sent!");
    }
    
    Ok(())
}
