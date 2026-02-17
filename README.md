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

Otter is built as a Rust workspace with eight core crates:

### Core Crates

1. **otter-identity** - Identity management and public-key cryptography
   - Ed25519 keypairs for signing and identity
   - X25519 keypairs for encryption key exchange
   - Peer ID generation and verification
   - Key serialization and persistence
   - Multi-device support with root identity and device subkeys
   - Device revocation and trust chains

2. **otter-crypto** - End-to-end encryption primitives
   - X25519 Diffie-Hellman key exchange
   - ChaCha20-Poly1305 authenticated encryption
   - Secure session management
   - Message encryption/decryption
   - Replay protection with message counters

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
   - WebRTC/ICE negotiation for NAT traversal

5. **otter-messaging** - High-level messaging protocol
   - Message types and MessagePack serialization
   - Conversation management
   - Integration with crypto layer
   - Message routing
   - Encrypted message envelopes

6. **otter-storage** - Data persistence layer
   - Identity storage
   - Message history
   - Peer information caching

7. **otter-voice** - Voice communication
   - WebRTC audio streaming
   - Call session management
   - Codec support

8. **otter-cli** - Command-line peer client
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
- **Serialization**: MessagePack (rmp-serde) for all protocol messages and encrypted data

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

1. **Build the project**:
```bash
cargo build --release
```

2. **Run Otter**:
```bash
.\target\release\otter.exe
```

On first run, Otter will automatically:
- Generate a unique identity (stored in `~/.otter/`)
- Display your Peer ID and fingerprint
- Start listening for other peers on the network

3. **Use the chat**:
   - Type `/peers` to see connected peers
   - Type `/send` to send an encrypted message
   - Type `/help` to see all available commands
   - Type `/quit` to exit

**For Testing**: To test message exchange, run two instances in separate terminals:
```bash
# Terminal 1
.\target\release\otter.exe --nickname Alice

# Terminal 2
.\target\release\otter.exe --nickname Bob --port 9001
```

Both peers will auto-discover each other via mDNS and exchange encrypted messages.

## Usage

### CLI Options

```bash
# Start with default nickname (based on your machine name)
otter

# Start with a custom nickname
otter --nickname Alice

# Start on a specific port (default: random port)
otter --port 9001

# Combine options
otter --nickname Bob --port 9002
```

### Interactive Commands

Once the peer is running, you can use these commands:

- `/peers` - List connected peers with their identities
- `/send` - Send an encrypted message to a peer
- `/call` - Start a voice call with a peer (experimental)
- `/hangup` - End the current voice call
- `/help` - Show available commands
- `/quit` - Exit the application

### Identity Management

Each peer automatically generates a unique identity on first run, stored in `~/.otter/` directory:
- **Peer ID**: A unique identifier derived from your Ed25519 public key
- **Fingerprint**: A short hash for quick verification
- **Keys**: Ed25519 for signing, X25519 for encryption

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
│   ├── otter-protocol/     # Protocol versioning
│   ├── otter-messaging/    # Message protocol
│   ├── otter-storage/      # Data persistence
│   ├── otter-voice/        # Voice communication
│   └── otter-cli/          # CLI client
├── Cargo.toml              # Workspace configuration
├── README.md
└── LICENSE.md
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
- **Replay attacks**: Message counters prevent message replay

**Serialization**: Messages are serialized using MessagePack with struct-map encoding for efficient binary transmission while maintaining compatibility with complex nested data structures.

### Limitations

- **Metadata privacy**: Network-level metadata (IP addresses, connection timing) is visible to network observers
- **Forward secrecy**: Current implementation uses `CryptoSession` with static session keys. `PFSSession` with ephemeral key exchange is implemented but not yet integrated into the messaging layer
- **Peer discovery**: mDNS reveals presence on local network
- **Message persistence**: Message history not yet implemented
- **Group encryption**: Currently supports only 1-to-1 encrypted conversations

## Roadmap

### In Progress
- [ ] **Voice Calls** - WebRTC audio integration (basic foundation complete)
- [ ] **Message History** - Encrypted local storage for message persistence

### Planned Features
- [ ] **Perfect Forward Secrecy** - Ephemeral key ratcheting (Signal protocol)
- [ ] **Video Calls** - WebRTC video support
- [ ] **Group Chat** - Multi-party encrypted messaging
- [ ] **File Transfer** - Encrypted file sharing with chunking
- [ ] **Mobile Apps** - iOS and Android clients
- [ ] **Desktop GUI** - Native applications for Windows/macOS/Linux
- [ ] **Improved NAT Traversal** - Enhanced STUN/TURN support
- [ ] **Offline Messages** - Store-and-forward for offline peers
- [ ] **Custom Stickers/Emoji** - Enhanced media support

### Research
- [ ] **Post-Quantum Cryptography** - Quantum-resistant key exchange
- [ ] **Tor/I2P Integration** - Anonymity network support
- [ ] **Blockchain Identity** - Distributed identity verification

## Contributing

Contributions are welcome! Before submitting code, please:

1. **Sign the CLA**: Read and accept the [Contributor License Agreement](CLA.md)
2. **Follow guidelines**: See [CONTRIBUTING.md](CONTRIBUTING.md) for code standards
3. **Open an issue first**: Discuss significant changes before implementing

**Important**: By contributing, you grant the project owner full rights to use your contribution under any license, including commercial licenses. Only the project owner can release official versions.

### Guidelines

- Follow Rust best practices and idioms
- Add tests for new functionality
- Update documentation as needed
- Keep the modular architecture clean
- Focus on security and privacy

## License

Copyright (c) 2026 GGally / Emanuele D'Angelo. All rights reserved.

This software is licensed under a custom proprietary license. See [LICENSE](LICENSE) for details.

### Key Terms:
- ✅ Free for personal and educational use
- ✅ Source code available for study and contribution
- ✅ Community contributions welcome (requires signing [CLA](CLA.md))
- ❌ No unauthorized forks or commercial derivatives
- ❌ Only official releases from this repository are supported
- 💼 Commercial licensing available - contact info@ggally.net

**Security Notice**: Only use official binaries from this repository. Unofficial forks may contain malicious modifications.

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
