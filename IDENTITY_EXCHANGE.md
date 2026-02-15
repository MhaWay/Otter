# Identity Exchange in Otter

## Overview

Identity exchange is the fundamental process that enables secure, encrypted communication between peers in Otter. This document explains how identity exchange works and how to implement it correctly.

## What is Identity Exchange?

When two Otter peers connect, they need to exchange their **public identities** before they can:
- Encrypt messages to each other
- Verify message authenticity
- Establish secure communication channels

An identity contains:
- **Peer ID**: Unique identifier derived from the public key
- **Ed25519 Verifying Key**: For signature verification
- **X25519 Public Key**: For encryption key exchange

## Architecture

### Components

1. **otter-identity**: Identity generation and management
   - `Identity`: Full identity with private keys
   - `PublicIdentity`: Shareable public keys only
   - `PeerId`: Unique peer identifier

2. **otter-messaging**: Message types and handling
   - `Message::Identity`: Identity announcement message
   - `MessageHandler`: Manages peer identities and crypto sessions

3. **otter-network**: P2P networking
   - `NetworkEvent::PeerConnected`: Peer connection notification
   - `NetworkCommand::SendMessage`: Send messages to peers
   - Handles message routing and delivery

## Identity Exchange Flow

### Automatic Exchange on Connection

```
Peer A                          Network                         Peer B
  |                                |                                |
  |------ Connect --------------->|<------- Connect ---------------|
  |                                |                                |
  |<-- PeerConnected(B) -----------|--- PeerConnected(A) -------->|
  |                                |                                |
  |-- Identity(A) --------------->|----------------------------->|
  |                                |                                |
  |                                |<---------- Identity(B) -------|
  |<------------------------------|<-- Identity(B) ---------------|
  |                                |                                |
  |  Register B's identity         |         Register A's identity  |
  |  Create crypto session         |         Create crypto session  |
  |                                |                                |
  |  ✓ Ready for encrypted chat   |    ✓ Ready for encrypted chat |
```

### Message Flow

1. **Connection Established**
   - libp2p establishes P2P connection
   - Both peers receive `NetworkEvent::PeerConnected`

2. **Identity Announcement**
   - Each peer creates `Message::Identity` with their `PublicIdentity`
   - Message sent via `NetworkCommand::SendMessage`
   - Network layer broadcasts to connected peer

3. **Identity Reception**
   - Peer receives `NetworkEvent::MessageReceived`
   - Deserializes `Message::Identity`
   - Calls `MessageHandler::register_peer()`

4. **Session Establishment**
   - `register_peer()` creates `CryptoSession`
   - Session performs X25519 key exchange
   - Both peers ready for encrypted messaging

## Implementation Guide

### Sending Your Identity

```rust
use otter_messaging::Message;
use otter_network::NetworkCommand;

// On peer connection
let handler = message_handler.lock().await;
let identity_msg = Message::identity(handler.public_identity());
let identity_bytes = identity_msg.to_bytes()?;

// Send via network
command_tx.send(NetworkCommand::SendMessage {
    peer_id: connected_peer_id,
    data: identity_bytes,
}).await?;
```

### Receiving Peer Identity

```rust
// On message received
match message {
    Message::Identity { public_identity, .. } => {
        let mut handler = message_handler.lock().await;
        handler.register_peer(public_identity)?;
        println!("✓ Identity verified for peer");
    }
    // ... other message types
}
```

### Complete Example

```rust
async fn handle_network_event(
    event: NetworkEvent,
    message_handler: Arc<Mutex<MessageHandler>>,
    command_tx: mpsc::Sender<NetworkCommand>,
) -> Result<()> {
    match event {
        NetworkEvent::PeerConnected { peer_id } => {
            info!("Connected to peer: {}", peer_id);
            
            // Auto-send our identity
            let handler = message_handler.lock().await;
            let identity_msg = Message::identity(handler.public_identity());
            let data = identity_msg.to_bytes()?;
            drop(handler); // Release lock
            
            command_tx.send(NetworkCommand::SendMessage {
                peer_id: peer_id.clone(),
                data,
            }).await?;
            
            println!("✓ Sent identity to {}", peer_id);
        }
        
        NetworkEvent::MessageReceived { from, data } => {
            if let Ok(message) = Message::from_bytes(&data) {
                match message {
                    Message::Identity { public_identity, .. } => {
                        let mut handler = message_handler.lock().await;
                        handler.register_peer(public_identity)?;
                        println!("✓ Identity received from {}", from);
                    }
                    // ... handle other messages
                }
            }
        }
        
        // ... other events
    }
    
    Ok(())
}
```

## Security Considerations

### What Identity Exchange Provides

✅ **Authenticity**: Peer ID is derived from public key (cryptographic binding)  
✅ **Integrity**: Identities are signed and verifiable  
✅ **Confidentiality**: Only public keys shared (private keys never transmitted)  
✅ **Forward Setup**: Enables subsequent encrypted communication  

### What Identity Exchange Does NOT Provide

❌ **Trust**: You must verify Peer ID out-of-band (TOFU - Trust On First Use)  
❌ **Privacy from Network**: Observers see identity messages  
❌ **Anonymity**: Peer IDs are consistent across sessions  

### Best Practices

1. **Verify Fingerprints**: Compare first 8 bytes of public key with peer out-of-band
2. **Trust On First Use**: Accept first identity, warn on changes
3. **Key Pinning**: Store verified identities locally
4. **Detect Changes**: Alert if peer changes keys unexpectedly

## Network Commands

### Required NetworkCommand Enum

```rust
pub enum NetworkCommand {
    /// Send a message to a specific peer
    SendMessage {
        peer_id: PeerId,
        data: Vec<u8>,
    },
    
    /// Broadcast message to all connected peers
    BroadcastMessage {
        data: Vec<u8>,
    },
    
    /// List connected peers
    ListPeers {
        response: mpsc::Sender<Vec<PeerId>>,
    },
}
```

## Troubleshooting

### Identity Not Received

**Symptom**: Cannot encrypt messages, "Peer not found" error

**Causes**:
- Identity message not sent after connection
- Network command not implemented
- Message serialization failed
- Network disconnected before exchange completed

**Solution**:
```bash
# Check logs for identity exchange
RUST_LOG=otter=debug ./otter

# Look for these messages:
# "Connected to peer: <peer_id>"
# "Sent identity to <peer_id>"
# "Identity received from <peer_id>"
# "Registered peer <peer_id>"
```

### Session Creation Failed

**Symptom**: "Encryption error" or "Session not found"

**Causes**:
- `register_peer()` not called
- Key derivation failed
- Incompatible key formats

**Solution**: Check that both peers successfully registered each other

### Keys Changed Warning

**Symptom**: Different Peer ID or fingerprint than expected

**Causes**:
- Peer regenerated identity
- Man-in-the-middle attack
- Different peer using same endpoint

**Solution**: Verify fingerprint out-of-band before proceeding

## Testing

### Unit Test: Identity Exchange

```rust
#[tokio::test]
async fn test_identity_exchange() {
    let alice = Identity::generate().unwrap();
    let bob = Identity::generate().unwrap();
    
    // Alice's handler
    let mut alice_handler = MessageHandler::new(alice);
    let alice_public = alice_handler.public_identity();
    
    // Bob's handler  
    let mut bob_handler = MessageHandler::new(bob);
    let bob_public = bob_handler.public_identity();
    
    // Alice registers Bob
    alice_handler.register_peer(bob_public.clone()).unwrap();
    
    // Bob registers Alice
    bob_handler.register_peer(alice_public).unwrap();
    
    // Both should be able to encrypt now
    let msg = alice_handler
        .prepare_encrypted_message(bob_public.peer_id().as_str(), "Hello Bob!")
        .unwrap();
    
    let decrypted = bob_handler.decrypt_message(&msg).unwrap();
    assert_eq!(decrypted, "Hello Bob!");
}
```

### Integration Test: Full Flow

```bash
# Terminal 1: Start Alice
./otter --nickname Alice --port 9000

# Terminal 2: Start Bob  
./otter --nickname Bob --port 9001

# Expected output in both terminals:
# ✓ Connected: <peer_id>
# ✓ Sent identity to <peer_id>
# ✓ Identity received from <peer_id>
# ✓ Identity verified for peer: <peer_id>

# Now try encrypted messaging:
# Alice> /send
# Select Bob
# Message: Hello!
# ✓ Message encrypted and sent!

# Bob should receive:
# 🔐 Encrypted message from <alice_peer_id>: Hello!
```

## FAQ

### Q: When does identity exchange happen?

**A**: Automatically on every peer connection. Each peer sends their identity immediately after connecting.

### Q: Can I send encrypted messages before identity exchange?

**A**: No. You need the peer's public keys to encrypt. Wait for "Identity verified" message.

### Q: What if identity exchange fails?

**A**: You can manually trigger it with `/announce` command or by reconnecting.

### Q: How do I verify a peer's identity?

**A**: Compare the fingerprint (first 8 bytes of public key, shown in hex) out-of-band (phone, in-person, etc.).

### Q: Can I trust the first identity I receive?

**A**: Only if you've verified the fingerprint. Otherwise, use Trust On First Use (TOFU) model: trust first identity, warn on changes.

### Q: What happens if a peer changes keys?

**A**: Their Peer ID will change. Otter should warn you about this potential security issue.

## References

- **Ed25519**: Digital signatures (RFC 8032)
- **X25519**: Key exchange (RFC 7748)  
- **BLAKE3**: Peer ID hashing
- **ChaCha20-Poly1305**: Message encryption (RFC 7539)

## See Also

- `ARCHITECTURE.md`: Overall system design
- `SECURITY_ENHANCEMENTS.md`: Security improvements
- `crates/otter-identity/`: Identity implementation
- `crates/otter-messaging/`: Message types
- `crates/otter-crypto/`: Encryption layer
