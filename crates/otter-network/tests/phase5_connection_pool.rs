//! Phase 5 Integration Test: Connection Pool & Quality Management
//!
//! Tests:
//! - Auto-discovery trigger when below minimum peers
//! - Pruning when exceeding maximum peers

use otter_network::{create_network_channels, Network, NetworkCommand, NetworkEvent};
use tokio::time::{timeout, Duration};

#[tokio::test(flavor = "multi_thread")]
async fn test_discovery_triggers_below_min_peers() {
    let (event_tx, mut event_rx, _command_tx, command_rx) = create_network_channels();
    let mut network = Network::new(event_tx, command_rx).unwrap();
    network.listen("/ip4/127.0.0.1/tcp/0").unwrap();
    network.set_pool_params(2, 10, 6);

    let handle = tokio::spawn(async move {
        let _ = network.run().await;
    });

    let result = timeout(Duration::from_secs(5), async {
        while let Some(event) = event_rx.recv().await {
            if let NetworkEvent::DiscoveringPeers { connected_count } = event {
                assert!(connected_count < 2);
                break;
            }
        }
    }).await;

    handle.abort();
    assert!(result.is_ok(), "Expected DiscoveringPeers event");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_prunes_excess_peers() {
    let (event_tx1, mut event_rx1, _command_tx1, command_rx1) = create_network_channels();
    let mut net1 = Network::new(event_tx1, command_rx1).unwrap();
    net1.listen("/ip4/127.0.0.1/tcp/0").unwrap();
    net1.set_pool_params(1, 1, 1);

    let peer_id1 = net1.local_peer_id();

    let handle1 = tokio::spawn(async move {
        let _ = net1.run().await;
    });

    let listen_addr = timeout(Duration::from_secs(5), async {
        loop {
            if let Some(event) = event_rx1.recv().await {
                if let NetworkEvent::ListeningOn { address } = event {
                    break address;
                }
            }
        }
    }).await.expect("Timeout waiting for listen addr");

    let (event_tx2, _event_rx2, command_tx2, command_rx2) = create_network_channels();
    let mut net2 = Network::new(event_tx2, command_rx2).unwrap();
    net2.listen("/ip4/127.0.0.1/tcp/0").unwrap();

    let handle2 = tokio::spawn(async move {
        let _ = net2.run().await;
    });

    let (event_tx3, _event_rx3, command_tx3, command_rx3) = create_network_channels();
    let mut net3 = Network::new(event_tx3, command_rx3).unwrap();
    net3.listen("/ip4/127.0.0.1/tcp/0").unwrap();

    let handle3 = tokio::spawn(async move {
        let _ = net3.run().await;
    });

    let addr_with_peer = format!("{}/p2p/{}", listen_addr, peer_id1);

    command_tx2.send(NetworkCommand::DialPeer {
        peer_id: peer_id1,
        address: addr_with_peer.clone(),
    }).await.unwrap();

    command_tx3.send(NetworkCommand::DialPeer {
        peer_id: peer_id1,
        address: addr_with_peer,
    }).await.unwrap();

    let prune_result = timeout(Duration::from_secs(8), async {
        while let Some(event) = event_rx1.recv().await {
            if let NetworkEvent::PeerOffline { .. } = event {
                break;
            }
        }
    }).await;

    handle1.abort();
    handle2.abort();
    handle3.abort();

    assert!(prune_result.is_ok(), "Expected PeerOffline due to pruning");
}
