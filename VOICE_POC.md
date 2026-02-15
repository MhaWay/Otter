# Voice Proof-of-Concept Documentation

## Overview

Otter now includes minimal 1-to-1 voice calling functionality using WebRTC for real-time audio communication. This implementation prioritizes connection stability and simplicity over advanced features.

## Features

### Core Capabilities

- **1-to-1 Voice Calls**: Exactly two peers can establish a voice connection
- **Mono Audio**: Single channel audio at 48kHz sample rate
- **Fixed Bitrate**: 64 kbps for predictable network usage
- **Opus Codec**: Industry-standard codec for WebRTC voice
- **Secure Signaling**: All call setup messages transmitted over encrypted channels
- **NAT Traversal**: Automatic ICE/STUN negotiation for connectivity

### Security

- **End-to-End Authenticated**: Peers authenticate via identity keys
- **Encrypted Signaling**: SDP and ICE candidates sent over existing E2E encrypted messaging
- **DTLS Transport**: WebRTC provides additional transport-level encryption
- **No Plaintext Leakage**: All call metadata protected

## Usage

### Prerequisites

1. Two Otter peers with generated identities
2. Network connectivity (peer discovery via mDNS or manual connection)
3. Identity exchange completed (automatic on connection)

### Starting a Call

**Peer A (Initiator):**
```bash
$ otter start -i alice.json -p 9000

otter> /call 12D3KooWBobPeerId...
📞 Calling 12D3KooWBobPeerId...
✓ Call initiated (session: abc-123-def)
Waiting for peer to answer...
```

**Peer B (Receiver):**
```
📞 Incoming call from 12D3KooWAlicePeerId...! Type /call to answer

otter> /call
📞 Answering call from 12D3KooWAlicePeerId...
✓ Call answered
Connecting...
✓ Call connected with 12D3KooWAlicePeerId...
```

### During a Call

Once connected, audio streams bidirectionally. The connection status is displayed in the CLI.

**Connection states:**
- `Calling` - Waiting for peer to answer
- `Ringing` - Incoming call notification
- `Connecting` - ICE negotiation in progress
- `Connected` - Voice call active

### Ending a Call

Either peer can hang up:

```bash
otter> /hangup
📞 Hanging up call with 12D3KooWPeerId...
✓ Call ended
```

## Architecture

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                      Otter CLI                               │
│  /call, /hangup commands                                    │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   Voice Manager                              │
│  - Call state management                                    │
│  - Signaling coordination                                   │
│  - Session lifecycle                                        │
└──────────┬──────────────────────────┬───────────────────────┘
           │                          │
           ▼                          ▼
┌──────────────────────┐   ┌──────────────────────────────┐
│  Signaling Channel   │   │   WebRTC Peer Connection      │
│  (Encrypted)         │   │   - Audio tracks              │
│  - Offer/Answer      │   │   - ICE candidates            │
│  - ICE candidates    │   │   - DTLS transport            │
└──────────────────────┘   └───────────────────────────────┘
```

### Call Flow

```
Initiator                                           Responder
    |                                                   |
    |------- Discover peers via mDNS/DHT ------------->|
    |<------ Exchange identities (encrypted) ----------|
    |                                                   |
    |------- /call command ----------------------------|
    |                                                   |
    |------- Offer (SDP via encrypted msg) ----------->|
    |                                                   | [Ringing state]
    |                                                   |
    |                                                   |------- /call command
    |<------ Answer (SDP via encrypted msg) -----------|
    |                                                   |
    |<====== ICE Candidates (encrypted) =============>|
    |                                                   |
    |<=== WebRTC Connection (DTLS encrypted audio) ===>|
    |                                                   |
    |------- /hangup command --------------------------|
    |------- Hangup (via encrypted msg) ------------->|
    |                                                   |
```

### Call State Machine

```
       Idle
        │
        ├──(/call peer_id)──> Calling ──(answer)──> Connecting
        │                                               │
        └──(incoming)──> Ringing ──(/call)──> Connecting
                                                        │
                                                        ▼
                                                   Connected
                                                        │
                                                   (/hangup)
                                                        │
                                                        ▼
                                                     Ended
```

## Configuration

### Default Settings

```rust
CallConfig {
    sample_rate: 48000,      // 48 kHz
    channels: 1,             // Mono
    bitrate: 64000,          // 64 kbps
    stun_servers: [
        "stun:stun.l.google.com:19302",
        "stun:stun1.l.google.com:19302"
    ],
    turn_servers: [],        // Optional relay servers
}
```

### Custom Configuration

To use custom STUN/TURN servers, modify the `CallConfig` in the voice manager initialization:

```rust
let config = CallConfig {
    stun_servers: vec!["stun:mystun.server.com:3478".to_string()],
    turn_servers: vec!["turn:myturn.server.com:3478".to_string()],
    ..Default::default()
};

voice_manager.initiate_call("peer_id", config).await?;
```

## Technical Details

### Audio Codec

**Opus** is used for audio encoding:
- Sample rate: 48 kHz
- Channels: 1 (mono)
- Bitrate: 64 kbps (fixed)
- Frame size: 20ms (960 samples at 48kHz)

### WebRTC Stack

- **Peer Connection**: Manages WebRTC connection lifecycle
- **ICE**: Interactive Connectivity Establishment for NAT traversal
- **DTLS**: Datagram Transport Layer Security for media encryption
- **SRTP**: Secure Real-time Transport Protocol for audio packets

### NAT Traversal

The voice implementation uses ICE (Interactive Connectivity Establishment) with STUN servers to establish connectivity across NAT devices:

1. **Host candidates**: Direct peer-to-peer if on same network
2. **Server reflexive**: Via STUN server for public IP discovery
3. **Relay**: Via TURN server if direct connection fails (optional)

## Limitations

### By Design

The following are intentionally NOT implemented to maintain simplicity:

- ❌ Group calls or conference rooms
- ❌ Video support
- ❌ Screen sharing
- ❌ Call recording
- ❌ Call history
- ❌ Call quality metrics display
- ❌ Audio device selection
- ❌ Echo cancellation controls
- ❌ Noise suppression controls

### Current Constraints

- **Single call only**: Only one active call per peer at a time
- **CLI interface**: No graphical user interface
- **Manual peer ID**: Must know peer ID to initiate call
- **No call queueing**: Incoming calls during active call are rejected

## Troubleshooting

### "Call failed to connect"

**Possible causes:**
1. Peer is not online or reachable
2. NAT/firewall blocking UDP traffic
3. No STUN server connectivity

**Solutions:**
- Verify peer is running and connected
- Check firewall rules for UDP traffic
- Try adding TURN server for relay

### "No audio heard"

**Possible causes:**
1. Call state not showing "Connected"
2. Audio devices not configured
3. Network packet loss

**Solutions:**
- Verify call state is "Connected"
- Check system audio settings
- Monitor network connectivity

### "Cannot initiate call"

**Possible causes:**
1. Already in an active call
2. Peer ID incorrect or not connected
3. Identity exchange not completed

**Solutions:**
- Run `/hangup` to end current call
- Verify peer ID with `/peers` command
- Wait for identity exchange after connection

## Performance

### Network Usage

- **Baseline**: ~64 kbps (8 KB/s) for audio
- **Overhead**: ~10-15% for protocol headers
- **Total**: ~70-75 kbps typical

### Latency

- **Local network**: 10-50ms
- **Internet (same region)**: 50-150ms
- **Cross-region**: 150-300ms

Latency depends primarily on network conditions, not the implementation.

## Security Considerations

### Threat Model

**Protected against:**
- ✅ Eavesdropping on signaling messages (encrypted)
- ✅ MITM attacks on signaling (authenticated via identity keys)
- ✅ Unauthorized call initiation (peer authentication)
- ✅ Replay attacks (session IDs and counters)

**Not protected against:**
- ⚠️ Traffic analysis (metadata about when calls occur)
- ⚠️ Network-level correlation attacks
- ⚠️ Endpoint compromise (OS/hardware backdoors)

### Best Practices

1. **Verify peer fingerprints** out-of-band for high-security scenarios
2. **Use trusted STUN/TURN servers** or run your own
3. **Keep software updated** for security patches
4. **Monitor for key changes** (TOFU trust model)

## Future Enhancements

### Planned Improvements

- **Audio quality controls**: Adjustable bitrate and quality
- **Video support**: Add video tracks to calls
- **Group calls**: Multi-party voice rooms
- **Call history**: Log of past calls
- **Device selection**: Choose audio input/output devices
- **Call metrics**: Display connection quality and statistics

### Not Planned

- Mobile app support (out of scope for core platform)
- Web interface (CLI-focused design)
- Legacy protocol compatibility (modern WebRTC only)

## API Reference

### VoiceManager

Main interface for voice calling:

```rust
impl VoiceManager {
    /// Create a new voice manager
    pub fn new() -> Result<Self>;
    
    /// Set signaling channel for sending messages
    pub fn set_signaling_channel(&mut self, tx: mpsc::UnboundedSender<...>);
    
    /// Initiate a call to a peer
    pub async fn initiate_call(&mut self, peer_id: &str, config: CallConfig) -> Result<String>;
    
    /// Handle incoming signaling message
    pub async fn handle_signaling(&mut self, peer_id: &str, message: SignalingMessage) -> Result<()>;
    
    /// Answer an incoming call
    pub async fn answer_call(&mut self) -> Result<()>;
    
    /// Hang up the current call
    pub async fn hangup(&mut self) -> Result<()>;
    
    /// Get current call state
    pub async fn get_call_state(&self) -> CallState;
    
    /// Check if there's an active call
    pub async fn has_active_call(&self) -> bool;
    
    /// Get current peer ID if in call
    pub async fn get_current_peer(&self) -> Option<String>;
}
```

### CallState

```rust
pub enum CallState {
    Idle,        // No active call
    Calling,     // Outgoing call, waiting for answer
    Ringing,     // Incoming call, waiting for user to answer
    Connecting,  // Call being established (ICE)
    Connected,   // Call active
    Ended,       // Call terminated
}
```

### SignalingMessage

```rust
pub enum SignalingMessage {
    Offer { sdp: String, media_type: MediaType, session_id: String },
    Answer { sdp: String, session_id: String },
    IceCandidate { candidate: String, session_id: String, ... },
    IceComplete { session_id: String },
    Hangup { session_id: String, reason: Option<String> },
}
```

## Testing

### Unit Tests

Run voice-specific tests:
```bash
cargo test -p otter-voice
```

### Integration Testing

Test with two peers:

**Terminal 1:**
```bash
otter init -o peer1.json
otter start -i peer1.json -p 9000
```

**Terminal 2:**
```bash
otter init -o peer2.json
otter start -i peer2.json -p 9001

# Wait for peer discovery
otter> /peers
Connected Peers:
  1. 12D3KooWPeer1...

otter> /call 12D3KooWPeer1...
```

## Contributing

To contribute to voice functionality:

1. Familiarize yourself with WebRTC concepts
2. Review the `otter-voice` crate source
3. Test changes with real peer connections
4. Ensure all tests pass: `cargo test --all`
5. Document any new features or changes

## License

Voice PoC is part of the Otter project and is dual-licensed under MIT OR Apache-2.0.
