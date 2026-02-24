//! Phase 7 Integration Tests: Production Scenarios
//!
//! Tests:
//! - Multi-peer discovery (5 peers)
//! - Cold start scenario
//! - Partition recovery simulation

use otter_network::{create_network_channels, Network, NetworkCommand, NetworkEvent};
use tokio::time::{timeout, Duration};

async fn start_network() -> (
    tokio::sync::mpsc::Sender<NetworkCommand>,
    tokio::sync::mpsc::Receiver<NetworkEvent>,
    libp2p::PeerId,
    tokio::task::JoinHandle<()>,
    String,
) {
    let (event_tx, event_rx, command_tx, command_rx) = create_network_channels();
    let mut network = Network::new(event_tx, command_rx).unwrap();
    network.listen("/ip4/127.0.0.1/tcp/0").unwrap();
    let peer_id = network.local_peer_id();

    let handle = tokio::spawn(async move {
        let _ = network.run().await;
    });

    let listen_addr = timeout(Duration::from_secs(5), async {
        let mut rx = event_rx;
        loop {
            if let Some(event) = rx.recv().await {
                if let NetworkEvent::ListeningOn { address } = event {
                    return (rx, address);
                }
            }
        }
    }).await.expect("Timeout waiting for listen address");

    (command_tx, listen_addr.0, peer_id, handle, listen_addr.1)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multi_peer_discovery() {
    let (cmd1, mut rx1, peer1, handle1, addr1) = start_network().await;

    let mut handles = vec![handle1];
    let mut cmd_senders = vec![cmd1];

    for _ in 0..4 {
        let (cmd, _rx, _peer_id, handle, _addr) = start_network().await;
        handles.push(handle);
        cmd_senders.push(cmd);
    }

    let addr_with_peer = format!("{}/p2p/{}", addr1, peer1);
    for cmd in cmd_senders.iter().skip(1) {
        cmd.send(NetworkCommand::DialPeer {
            peer_id: peer1,
            address: addr_with_peer.clone(),
        }).await.unwrap();
    }

    let result = timeout(Duration::from_secs(10), async {
        let mut online_count = 0usize;
        while let Some(event) = rx1.recv().await {
            if let NetworkEvent::PeerOnline { .. } = event {
                online_count += 1;
                if online_count >= 4 {
                    break;
                }
            }
        }
    }).await;

    for handle in handles {
        handle.abort();
    }

    assert!(result.is_ok(), "Expected 4 peers to come online");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cold_start_scenario() {
    let (_cmd1, mut rx1, peer1, handle1, addr1) = start_network().await;

    // No peers yet; should not see PeerOnline immediately
    let idle = timeout(Duration::from_secs(2), async {
        while let Some(event) = rx1.recv().await {
            if let NetworkEvent::PeerOnline { .. } = event {
                panic!("Unexpected peer online during cold start idle period");
            }
        }
    }).await;
    assert!(idle.is_err(), "Expected no PeerOnline events during idle period");

    let (cmd2, _rx2, _peer2, handle2, _addr2) = start_network().await;
    let addr_with_peer = format!("{}/p2p/{}", addr1, peer1);
    cmd2.send(NetworkCommand::DialPeer {
        peer_id: peer1,
        address: addr_with_peer,
    }).await.unwrap();

    let result = timeout(Duration::from_secs(5), async {
        while let Some(event) = rx1.recv().await {
            if let NetworkEvent::PeerOnline { .. } = event {
                break;
            }
        }
    }).await;

    handle1.abort();
    handle2.abort();

    assert!(result.is_ok(), "Expected PeerOnline after second peer joins");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_partition_recovery() {
    let (_cmd1, mut rx1, peer1, handle1, addr1) = start_network().await;

    let (cmd2, _rx2, _peer2, handle2, _addr2) = start_network().await;
    let (cmd3, _rx3, _peer3, handle3, _addr3) = start_network().await;

    let addr_with_peer = format!("{}/p2p/{}", addr1, peer1);
    cmd2.send(NetworkCommand::DialPeer {
        peer_id: peer1,
        address: addr_with_peer.clone(),
    }).await.unwrap();

    cmd3.send(NetworkCommand::DialPeer {
        peer_id: peer1,
        address: addr_with_peer,
    }).await.unwrap();

    // Wait for at least one peer online
    let _ = timeout(Duration::from_secs(5), async {
        while let Some(event) = rx1.recv().await {
            if let NetworkEvent::PeerOnline { .. } = event {
                break;
            }
        }
    }).await;

    // Simulate partition by stopping peers 2 and 3
    handle2.abort();
    handle3.abort();

    // Start a new peer and reconnect
    let (cmd4, _rx4, _peer4, handle4, _addr4) = start_network().await;
    let addr_with_peer = format!("{}/p2p/{}", addr1, peer1);
    cmd4.send(NetworkCommand::DialPeer {
        peer_id: peer1,
        address: addr_with_peer,
    }).await.unwrap();

    let result = timeout(Duration::from_secs(6), async {
        while let Some(event) = rx1.recv().await {
            if let NetworkEvent::PeerOnline { .. } = event {
                break;
            }
        }
    }).await;

    handle1.abort();
    handle4.abort();

    assert!(result.is_ok(), "Expected recovery after new peer joins");
}
