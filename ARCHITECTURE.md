# Otter Architecture

This document describes the architecture and design decisions of the Otter decentralized chat platform.

## Overview

Otter is designed as a modular system with clear separation of concerns. Each layer builds upon the previous, creating a clean and extensible architecture.

```
┌─────────────────────────────────────────┐
│         otter-cli (Application)         │
│    Interactive CLI, User Interface      │
└─────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────┐
│      otter-messaging (Protocol)         │
│  Message Types, Routing, Conversations  │
└─────────────────────────────────────────┘
                    ↓
┌──────────────────────┬──────────────────┐
│   otter-network      │  otter-crypto    │
│  P2P Networking      │  Encryption      │
│  Peer Discovery      │  Key Exchange    │
└──────────────────────┴──────────────────┘
                    ↓
┌─────────────────────────────────────────┐
│         otter-identity (Core)           │
│     Keypairs, Peer ID, Signatures       │
└─────────────────────────────────────────┘
```

## Layer Descriptions

### Identity Layer (otter-identity)

**Purpose**: Foundation for all security and identity in the system.

**Key Components**:
- `Identity`: Complete identity with private keys
- `PublicIdentity`: Shareable public identity
- `PeerId`: Unique identifier derived from public key

**Cryptography**:
- **Ed25519**: For signing and identity
  - Public key → PeerId via BLAKE3 hash
  - Used for message authentication
- **X25519**: For encryption key exchange
  - Separate from signing keys (security best practice)
  - Used in Diffie-Hellman key agreement

**Design Decisions**:
- Separate signing and encryption keys following cryptographic best practices
- PeerId derived from public key for verifiable identity
- JSON serialization for easy key backup
- No password protection (could be added later)

### Crypto Layer (otter-crypto)

**Purpose**: End-to-end encryption for all communications.

**Key Components**:
- `CryptoSession`: Manages encryption between two peers
- `EncryptedMessage`: Encrypted message envelope
- `MessageCrypto`: Utility functions for common operations

**Encryption Flow**:
1. X25519 key exchange creates shared secret
2. BLAKE3 derives symmetric key from shared secret
3. ChaCha20-Poly1305 encrypts messages with unique nonce

**Design Decisions**:
- ChaCha20-Poly1305 chosen for:
  - Speed (faster than AES on non-hardware-accelerated systems)
  - Security (modern AEAD cipher)
  - Simplicity (single primitive for encryption + authentication)
- Random nonce per message prevents replay attacks
- Optional associated data for authenticated metadata

### Network Layer (otter-network)

**Purpose**: Peer-to-peer networking and discovery.

**Key Components**:
- `Network`: Main network manager
- `OtterBehaviour`: libp2p behavior combining multiple protocols
- `NetworkEvent`: Events from network (connections, messages)
- `NetworkCommand`: Commands to network (send, dial)

**Protocols Used**:
- **Gossipsub**: For message propagation (pub/sub pattern)
- **mDNS**: For local peer discovery
- **Kademlia DHT**: For distributed peer discovery and routing
- **Identify**: For peer information exchange
- **Noise**: For secure transport encryption
- **Yamux/Mplex**: For stream multiplexing

**Design Decisions**:
- Gossipsub for broadcasting (simple, efficient for small networks)
- mDNS for zero-config local discovery
- Kademlia for Internet-scale discovery
- Event-driven architecture with channels for loose coupling
- Separate transport encryption (Noise) from message encryption (E2E)

### Messaging Layer (otter-messaging)

**Purpose**: High-level messaging protocol and conversation management.

**Key Components**:
- `Message`: Enum of all message types
- `MessageHandler`: Manages peers and crypto sessions
- `MessagingEvent`: High-level events (text received, etc.)
- `MessagingCommand`: High-level commands

**Message Types**:
- **Text**: Plain text (for testing/public channels)
- **Encrypted**: E2E encrypted content
- **Identity**: Public key announcement
- **Status**: Presence updates
- **Typing**: Typing indicators

**Design Decisions**:
- Both binary (bincode) and JSON serialization supported
- Handler maintains mapping of peers to crypto sessions
- Automatic session establishment on identity exchange
- Extensible message type system

### Application Layer (otter-cli)

**Purpose**: User-facing command-line interface.

**Key Components**:
- CLI argument parsing
- Interactive command processor
- Event display
- User input handling

**Design Decisions**:
- Simple, focused on functionality over UI polish
- Interactive dialogues for complex operations
- Async architecture for responsive UI
- Clean separation from core logic (enables future GUI)

## Security Architecture

### Threat Model

**Protected Against**:
- Message eavesdropping (E2E encryption)
- Message tampering (AEAD authentication)
- Identity spoofing (signature verification)
- Replay attacks (unique nonces)

**Not Protected Against** (future work):
- Network metadata analysis (timing, size, participants)
- Long-term key compromise (no forward secrecy yet)
- Denial of service attacks
- Sybil attacks

### Security Boundaries

```
┌──────────────────────────────────────────────┐
│  Application Layer                           │
│  - Key management                            │
│  - User input validation                     │
└──────────────────────────────────────────────┘
         ↓ Identity, Commands
┌──────────────────────────────────────────────┐
│  E2E Encryption Boundary                     │
│  - Message encryption/decryption             │
│  - Session key management                    │
└──────────────────────────────────────────────┘
         ↓ Encrypted messages
┌──────────────────────────────────────────────┐
│  Transport Encryption (Noise)                │
│  - Connection encryption                     │
│  - Peer authentication                       │
└──────────────────────────────────────────────┘
         ↓ Network packets
┌──────────────────────────────────────────────┐
│  Network Layer                               │
│  - Routing, discovery                        │
│  - Connection management                     │
└──────────────────────────────────────────────┘
```

## Data Flow

### Message Sending Flow

```
User Input
    ↓
CLI Layer
    ↓ (text)
Message Handler
    ↓ (encrypt with peer session)
Crypto Layer
    ↓ (EncryptedMessage)
Message Serialization
    ↓ (bytes)
Network Layer
    ↓ (gossipsub broadcast)
libp2p Transport
    ↓ (Noise-encrypted)
Network
```

### Peer Discovery Flow

```
Network Start
    ↓
libp2p Listen
    ↓
mDNS Broadcast ──────────→ mDNS Listener (other peer)
    ↓                              ↓
    ←────────────────────────────────
          Peer Discovered Event
    ↓
Add to Kademlia DHT
    ↓
Connection Established
    ↓
Identity Exchange
    ↓
Crypto Session Creation
    ↓
Ready for Messaging
```

## Performance Considerations

### Cryptography

- **Ed25519**: ~100k signatures/sec, very fast verification
- **X25519**: ~50k key agreements/sec
- **ChaCha20-Poly1305**: ~1-2 GB/sec encryption throughput

These primitives are fast enough for real-time chat with minimal latency.

### Network

- **Gossipsub**: O(n) message propagation, suitable for <1000 peers
- **Kademlia DHT**: O(log n) lookup time
- **mDNS**: Minimal overhead, local broadcast only

### Memory

- Each peer stores:
  - Own identity (~100 bytes)
  - Public identity per known peer (~100 bytes)
  - Crypto session per peer (~64 bytes)
  - Network state (variable)

## Scalability

### Current Design

- Suitable for: Small to medium groups (2-100 peers)
- Gossipsub broadcasts to all peers
- Each peer maintains direct connections

### Future Improvements

- Relay peers for larger networks
- Message routing instead of broadcasting
- Partial mesh topologies
- Store-and-forward for offline peers

## Extension Points

The modular architecture enables easy extension:

1. **New Message Types**: Add to `Message` enum
2. **New Protocols**: Add to `OtterBehaviour`
3. **New Crypto**: Implement in `otter-crypto`
4. **New Interfaces**: Build on top of messaging layer
5. **Persistence**: Add storage layer below messaging

### Example: Adding Voice Chat

```
New Crate: otter-voice
    ↓ depends on
otter-network (for WebRTC signaling)
otter-crypto (for DTLS-SRTP keys)
otter-identity (for peer verification)
```

### Example: Adding File Transfer

```
New Module: messaging::files
    ↓ uses
otter-crypto (encrypt file chunks)
otter-network (transfer chunks)
Custom protocol (chunk management)
```

## Testing Strategy

### Unit Tests

- Each crate has comprehensive unit tests
- Crypto operations tested for correctness
- Identity generation and verification tested
- Message serialization tested

### Integration Tests

- Full message flow: encrypt → send → receive → decrypt
- Peer discovery and connection
- Session establishment

### Future Tests

- Performance benchmarks
- Stress tests (many peers, large messages)
- Security audits
- Fuzzing

## Future Architecture Considerations

### WebRTC Integration

- Add WebRTC data channels for direct peer-to-peer
- Use DTLS for transport security
- Maintain E2E encryption for application data
- Use libp2p for signaling

### Persistent Storage

- Optional encrypted local storage
- Message history
- Peer contacts
- Use SQLite or similar embedded database

### Mobile Support

- Core crates remain unchanged
- New mobile UI crate
- Consider battery and bandwidth
- Background service for notifications

### Federation/Bridging

- Gateway nodes to other networks
- Maintain E2E encryption across bridges
- Careful trust model needed

## Conclusion

Otter's architecture prioritizes:

1. **Security**: Multiple layers of protection
2. **Modularity**: Clean separation enables testing and extension
3. **Simplicity**: Easy to understand and audit
4. **Extensibility**: New features can be added without breaking existing code
5. **Privacy**: Minimal metadata, maximum encryption

The layered design ensures that each component has a single responsibility and can be developed, tested, and reasoned about independently.
