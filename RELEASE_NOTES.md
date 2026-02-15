# Otter v0.1.0 - Simple CLI Release

## Overview

This release makes Otter easy to use for non-technical users! No configuration required - just download and run.

## What's New

### 🎉 Zero-Configuration Mode

Simply run `otter` without any arguments:

```bash
./otter
```

On first run, Otter will:
- ✅ Auto-generate your identity
- ✅ Create data directory (~/.otter)
- ✅ Start networking automatically
- ✅ Display your Peer ID and fingerprint
- ✅ Show clear command instructions

### 🎨 Beautiful User Interface

```
╔══════════════════════════════════════════════════════════════╗
║          🦦 Otter - Decentralized Private Chat              ║
╚══════════════════════════════════════════════════════════════╝

📝 Nickname:    Alice
🆔 Peer ID:     7ZAruyiFsU3M8wDRrparg7dmYYEiQPK9EWTB1s1dewVi
🔑 Fingerprint: aa094652f00aae57
📁 Data Dir:    ~/.otter

✓ Network started successfully
✓ Listening for peers on the network...
```

### 🛠️ Command Line Options

Customize your experience:

```bash
# Set a nickname
otter --nickname "Alice"

# Use a specific port
otter --port 9000

# Custom data directory
otter --data-dir /path/to/data

# Combine options
otter --nickname "Bob" --port 9001
```

### 📦 Easy Distribution

Each release package includes:
1. **otter** / **otter.exe** - Optimized binary
2. **README** - Quick start guide
3. **run_otter.bat** / **run_otter.sh** - Easy launcher
4. **config.toml.example** - Optional configuration template

### 🔧 Build System

New Makefile targets for developers:

```bash
make release           # Build optimized binary
make release-windows   # Create Windows package
make release-linux     # Create Linux package
make release-macos     # Create macOS package
make clean            # Clean build artifacts
```

## Getting Started

### For End Users

**Windows:**
1. Download and extract the Windows release
2. Double-click `run_otter.bat`
3. Share your Peer ID with friends

**Linux/macOS:**
1. Download and extract the release
2. Run `./run_otter.sh` or `./otter`
3. Share your Peer ID with friends

### Commands

Once running, use these commands:

- `/peers` - List connected peers
- `/send` - Send an encrypted message
- `/call` - Start a voice call
- `/hangup` - End the current call
- `/help` - Show help
- `/quit` - Exit Otter

## Technical Details

### File Locations

**Windows:**
- Data: `C:\Users\YourName\.otter\`
- Identity: `C:\Users\YourName\.otter\identity.json`

**Linux/macOS:**
- Data: `~/.otter/`
- Identity: `~/.otter/identity.json`

### Security

- 🔐 End-to-end encryption (ChaCha20-Poly1305)
- 🔑 Ed25519 identity keys
- 🤝 X25519 key exchange
- 🔒 No central servers
- 🙈 No tracking or data collection

### Networking

- **mDNS** - Local network discovery
- **Kademlia DHT** - Distributed peer discovery
- **libp2p** - P2P networking stack
- **Random ports** - No port forwarding needed (by default)

## Backward Compatibility

Legacy subcommands still work:

```bash
# Generate identity explicitly
otter init -o identity.json

# Start with specific identity
otter start -i identity.json -p 9000

# Show identity info
otter info -i identity.json
```

## What's Coming Next

- 🚀 Voice/video calls implementation
- 👥 Group chat support
- 📁 File transfer
- 🔄 Multi-device sync
- 🌐 Improved NAT traversal

## Upgrading

This is the first release, but future upgrades will be simple:
1. Backup your `~/.otter/identity.json` file
2. Replace the binary with the new version
3. Run as usual - your identity is preserved

## Contributing

Want to help improve Otter?

1. Check out the [GitHub repository](https://github.com/MhaWay/Otter)
2. Read CONTRIBUTING.md
3. Open issues or submit pull requests

## License

Otter is open source under MIT OR Apache-2.0 license.

## Support

- 📖 Documentation: See QUICKSTART.md
- 🐛 Report bugs: GitHub Issues
- 💬 Discussions: GitHub Discussions

---

**Happy chatting! 🦦**

*Decentralized. Private. Simple.*
