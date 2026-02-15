# Identity Exchange Debugging Guide

## Problem
Peers connect successfully but identity exchange doesn't complete:
- `/peers` shows connected peers (network layer)
- `/send` shows "No peers registered" (messaging layer)

## Two Separate Systems

### 1. Network Layer (libp2p)
- **Tracks**: TCP connections
- **Command**: `/peers`
- **Source**: `Network.connected_peers` HashSet
- **Works**: ✅ Peers connect via auto-dial

### 2. Messaging Layer (Otter)
- **Tracks**: Peers with exchanged identities
- **Command**: `/send`
- **Source**: `MessageHandler.peers` HashMap
- **Broken**: ❌ Identity messages not received

## Identity Exchange Flow

```
Peer A                    Network                    Peer B
  |                          |                          |
  |-- ConnectionEstablished->|<-- ConnectionEstablished-|
  |                          |                          |
  | PeerConnected event      |      PeerConnected event |
  |                          |                          |
  | Create Identity msg      |      Create Identity msg |
  |                          |                          |
  | SendMessage command      |      SendMessage command |
  |     (via gossipsub)      |      (via gossipsub)     |
  |                          |                          |
  | Publish to topic ------->|<-------- Publish to topic|
  |                          |                          |
  | ??? gossipsub mesh ???   |   ??? gossipsub mesh ??? |
  |                          |                          |
  | MessageReceived? ◄-------+-------► MessageReceived? |
  |                          |                          |
```

## Potential Issues

### Issue 1: Timing
**Problem**: Identity sent immediately after connection
**Why it fails**: Gossipsub mesh not established yet
**Fix attempted**: Added 500ms delay ⏰

### Issue 2: Gossipsub Mesh Formation
**Problem**: Gossipsub requires both peers in mesh
**Requirements**:
1. Both peers subscribed to same topic ✅ (done in listen())
2. Peers must discover each other in gossipsub
3. Mesh formation takes time

**Observations**:
- Subscribe happens in `listen()` ✅
- But mesh formation is async
- No event fires when mesh ready

### Issue 3: Message Propagation
**Gossipsub behavior**:
- Messages are broadcast to ALL mesh peers
- Not point-to-point
- Requires stable mesh
- Has heartbeat interval (10 seconds in config)

### Issue 4: Connection vs Subscription
**libp2p layers**:
1. Transport (TCP): Connection ✅
2. Noise: Encryption ✅
3. Yamux: Multiplexing ✅
4. Application protocols: Independent!

**Gossipsub is separate protocol**:
- Connection != Gossipsub mesh membership
- Need to wait for Subscribed event
- But we don't currently handle it! ❌

## Debugging Steps

### 1. Check if messages are published
**Add logging**:
```rust
// In NetworkCommand::SendMessage handler
debug!("Publishing identity message, size: {}", data.len());
```

### 2. Check if messages are received
**Existing**:
```rust
// In gossipsub::Event::Message handler
debug!("Received message from {}", propagation_source);
```

### 3. Check gossipsub subscription
**Added**:
```rust
// Handle Subscribed event
gossipsub::Event::Subscribed { peer_id, .. } => {
    info!("Peer {} subscribed to gossipsub topic", peer_id);
}
```

### 4. Check message deserialization
**Existing**:
```rust
if let Ok(message) = Message::from_bytes(&data) {
    match message {
        Message::Identity { ... } => // handle
    }
}
```

## Solutions to Try

### Solution 1: Increase Delay ⏰
**Current**: 500ms
**Try**: 2-3 seconds (gossipsub heartbeat is 10s)
```rust
tokio::time::sleep(Duration::from_secs(2)).await;
```

### Solution 2: Wait for Subscribed Event 🔔
**Approach**: Only send identity after receiving Subscribed event
**Implementation**:
1. Track subscribed peers in Network
2. Fire `PeerSubscribed` network event
3. Send identity in response to that event

### Solution 3: Use Request-Response Protocol 🔄
**Approach**: Direct protocol instead of gossipsub
**Why**: More reliable for critical messages
**libp2p has**: request-response protocol

### Solution 4: Retry Mechanism 🔁
**Approach**: Retry identity send if not verified
**Implementation**:
1. Send identity
2. Wait 5 seconds
3. Check if peer registered
4. Retry if not

### Solution 5: Use Identify Protocol 🆔
**libp2p identify**: Built-in protocol for peer info
**Could piggyback**: Add our identity to identify info
**Currently**: Not using identify for our identity

## Recommended Fix

**Best approach**: Solution 2 (Wait for Subscribed)

**Why**:
1. Most correct - respects gossipsub protocol
2. No arbitrary delays
3. Event-driven

**Implementation**:
```rust
// In network layer
gossipsub::Event::Subscribed { peer_id, .. } => {
    self.event_tx.send(NetworkEvent::PeerReadyForMessages { 
        peer_id 
    }).await;
}

// In CLI
NetworkEvent::PeerReadyForMessages { peer_id } => {
    // Send identity now
}
```

## Testing

**Test 1**: Enable debug logging
```bash
RUST_LOG=otter=debug,libp2p=debug ./otter
```

**Test 2**: Check if Subscribed events fire
```bash
# Look for: "Peer XXX subscribed to gossipsub topic"
```

**Test 3**: Check message reception
```bash
# Look for: "Received message from XXX"
```

**Test 4**: Timing test
```bash
# If delay helps, we know it's timing
```

## Current Status

- [x] Auto-dial implemented
- [x] Connections established
- [x] 500ms delay added
- [x] Subscribed event handler added
- [ ] Test with actual peers
- [ ] Verify messages are received
- [ ] Implement proper fix based on results
