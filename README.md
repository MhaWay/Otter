# 🦦 Otter

A privacy-focused decentralized chat platform built with Rust.

## Overview

Otter is a peer-to-peer chat platform that prioritizes privacy, security, and decentralization. It uses modern cryptographic primitives and peer-to-peer networking to enable secure communication without central servers.

### Key Features

- **Fully Decentralized**: No central servers required. Peers discover and connect to each other directly.
- **End-to-End Encryption**: All messages are encrypted using ChaCha20-Poly1305 with X25519 key exchange.
- **Public-Key Identities**: Each peer has a unique identity based on Ed25519 keypairs.
- **Peer Discovery**: Automatic peer discovery using mDNS and Kademlia DHT.
- **Modular Architecture**: Clean separation of concerns with dedicated crates for identity, crypto, networking, and messaging.
- **Privacy-First**: No metadata collection, no tracking, no central authority.

## Architecture

Otter is built as a Rust workspace with six core crates:

### Core Crates

1. **otter-identity** - Identity management and public-key cryptography
   - Ed25519 keypairs for signing and identity
   - X25519 keypairs for encryption key exchange
   - Peer ID generation and verification
   - Key serialization and persistence
   - **Multi-device support** with root identity and device subkeys
   - Device revocation and trust chains

2. **otter-crypto** - End-to-end encryption primitives
   - X25519 Diffie-Hellman key exchange
   - ChaCha20-Poly1305 authenticated encryption
   - Secure session management
   - Message encryption/decryption

3. **otter-protocol** - Protocol versioning and capability negotiation
   - Protocol version negotiation
   - Capability discovery (voice, video, file transfer, etc.)
   - Handshake protocol
   - Protocol upgrade mechanisms
   - Ensures E2E encryption is mandatory

4. **otter-network** - Peer-to-peer networking layer
   - libp2p-based networking stack
   - mDNS for local peer discovery
   - Kademlia DHT for distributed peer discovery
   - Gossipsub for message propagation
   - Connection management
   - **WebRTC/ICE** negotiation for NAT traversal

5. **otter-messaging** - High-level messaging protocol
   - Message types and serialization
   - Conversation management
   - Integration with crypto layer
   - Message routing

6. **otter-cli** - Command-line peer client
   - Interactive chat interface
   - Peer management
   - Identity management
   - Network control

## Technology Stack

- **Language**: Rust 2021 edition
- **Networking**: libp2p for P2P communication
- **Cryptography**: 
  - Ed25519 (signing/identity)
  - X25519 (key exchange)
  - ChaCha20-Poly1305 (encryption)
  - BLAKE3 (hashing)
- **Async Runtime**: Tokio
- **Serialization**: Serde (JSON/bincode)

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Cargo

### Installation

```bash
# Clone the repository
git clone https://github.com/MhaWay/Otter.git
cd Otter

# Build the project
cargo build --release

# The binary will be at target/release/otter
```

### Quick Start

1. **Generate an identity**:
```bash
cargo run --release -p otter-cli -- init
```

This creates an `identity.json` file with your unique peer identity.

2. **Start the chat peer**:
```bash
cargo run --release -p otter-cli -- start
```

3. **Start another peer** (in a different terminal):
```bash
cargo run --release -p otter-cli -- init -o identity2.json
cargo run --release -p otter-cli -- start -i identity2.json -p 0
```

4. **Chat**:
- Use `/peers` to see connected peers
- Use `/send` to send encrypted messages
- Use `/help` for more commands

## Usage

### CLI Commands

```bash
# Initialize a new identity
otter init [-o identity.json]

# Start the chat peer
otter start [-i identity.json] [-p port]

# Show identity information
otter info [-i identity.json]
```

### Interactive Commands

Once the peer is running, you can use these commands:

- `/peers` - List connected peers
- `/send` - Send an encrypted message to a peer
- `/help` - Show available commands
- `/quit` - Exit the application

## Development

### Building

```bash
# Build all crates
cargo build

# Build specific crate
cargo build -p otter-identity

# Run tests
cargo test

# Run tests for specific crate
cargo test -p otter-crypto

# Build with optimizations
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_identity_generation
```

### Code Structure

```
Otter/
├── crates/
│   ├── otter-identity/     # Identity & key management
│   ├── otter-crypto/       # Encryption primitives
│   ├── otter-network/      # P2P networking
│   ├── otter-messaging/    # Message protocol
│   └── otter-cli/          # CLI client
├── Cargo.toml              # Workspace configuration
└── README.md
```

## Security Considerations

### Cryptographic Primitives

- **Ed25519**: Industry-standard elliptic curve signatures for identity and message authentication
- **X25519**: Elliptic curve Diffie-Hellman for key exchange
- **ChaCha20-Poly1305**: Authenticated encryption with associated data (AEAD)
- **BLAKE3**: Cryptographic hashing for key derivation and peer IDs

### Threat Model

Otter is designed to protect against:

- **Eavesdropping**: All messages are end-to-end encrypted
- **Man-in-the-middle**: Public keys are verified through identity exchange
- **Message tampering**: AEAD provides authentication
- **Identity spoofing**: Ed25519 signatures verify peer identity

### Limitations

- **Metadata privacy**: Network-level metadata (IP addresses, connection timing) is visible to network observers
- **Forward secrecy**: Current implementation doesn't implement perfect forward secrecy (future enhancement)
- **Peer discovery**: mDNS reveals presence on local network

## Future Enhancements

With the recently added protocol versioning, multi-device support, and WebRTC/ICE foundations, these features are now ready for implementation:

- [ ] **Voice Calls** - WebRTC audio with ICE negotiation (foundation complete)
- [ ] **Video Calls** - WebRTC video support (foundation complete)
- [ ] **Perfect Forward Secrecy** - Ephemeral key exchange
- [ ] **Group Chat** - Multi-party encrypted messaging (capability negotiation ready)
- [ ] **File Transfer** - Encrypted file sharing (protocol extensibility ready)
- [ ] **Key Rotation** - Device key rotation (multi-device model supports it)
- [ ] **Persistent Storage** - Encrypted message history
- [ ] **Mobile Apps** - iOS and Android clients
- [ ] **Bridge Support** - Connect to other networks
- [ ] **Improved NAT Traversal** - Enhanced STUN/TURN support

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues.

### Guidelines

- Follow Rust best practices and idioms
- Add tests for new functionality
- Update documentation as needed
- Keep the modular architecture clean
- Focus on security and privacy

## License

This project is licensed under MIT OR Apache-2.0.

## Acknowledgments

Built with:
- [libp2p](https://libp2p.io/) - Modular P2P networking stack
- [Tokio](https://tokio.rs/) - Async runtime
- [dalek-cryptography](https://github.com/dalek-cryptography) - Cryptographic primitives
- [RustCrypto](https://github.com/RustCrypto) - Cryptographic algorithms

## Contact

For questions or discussions, please open an issue on GitHub.

---

**Note**: This is a demonstration project focusing on clean architecture and privacy-preserving design. While it implements strong cryptography, it should undergo security audit before production use.
