# 🦦 Otter - Quick Start Guide

Welcome to Otter, a privacy-focused decentralized chat platform!

## What is Otter?

Otter is a peer-to-peer chat application that:
- **Runs without servers** - Connect directly with other users
- **Encrypts everything** - End-to-end encrypted messaging
- **Protects privacy** - No tracking, no data collection
- **Works anywhere** - Decentralized peer discovery

## Quick Start

### Windows

1. **Extract the files** to a folder
2. **Double-click `run_otter.bat`** to start
3. **Share your Peer ID** with others to connect

That's it! Otter will:
- Auto-generate your identity on first run
- Create a data directory in `%USERPROFILE%\.otter`
- Start listening for peer connections
- Display your Peer ID and fingerprint

### Linux / macOS

1. **Extract the files** to a folder
2. **Run `./run_otter.sh`** in terminal
3. **Share your Peer ID** with others to connect

Or run directly:
```bash
./otter
```

## Using Otter

Once Otter starts, you'll see your:
- **Peer ID** - Share this with people you want to connect with
- **Fingerprint** - First 8 bytes of your public key (for verification)
- **Data Directory** - Where your identity and data are stored

### Commands

Type these commands in Otter:

- `/peers` - List connected peers
- `/send` - Send an encrypted message
- `/call` - Start a voice call
- `/hangup` - End the current call
- `/help` - Show help
- `/quit` - Exit Otter

### First Connection

Otter uses **local network discovery** (mDNS) and **DHT** to find peers automatically. When another Otter user is on your network or reachable via DHT, you'll automatically discover each other!

**To connect with specific users:**
1. Share your Peer ID with them (it looks like: `12D3KooW...`)
2. Once discovered, you can use `/send` to message them

## Command Line Options

For advanced usage:

```bash
# Use a custom nickname
otter --nickname "Alice"

# Listen on a specific port
otter --port 9000

# Use a custom data directory
otter --data-dir /path/to/data

# Combine options
otter --nickname "Bob" --port 9001
```

### Legacy Commands

Otter also supports explicit subcommands:

```bash
# Generate a new identity
otter init

# Start with existing identity
otter start -i identity.json -p 0

# Show identity info
otter info -i identity.json
```

## Troubleshooting

### Can't connect to peers?

- **Check firewall**: Make sure Otter can accept incoming connections
- **Network issues**: Try using `--port` to specify a port and forward it
- **Local network**: Users on the same LAN should auto-discover via mDNS

### Identity lost?

Your identity is stored in:
- **Windows**: `C:\Users\YourName\.otter\identity.json`
- **Linux/macOS**: `~/.otter/identity.json`

**Important**: Backup this file! It contains your private keys.

### Fresh start?

Delete (or backup) your `.otter` directory and restart. A new identity will be generated.

## Security Notes

- **Private Keys**: Your identity file contains private keys. Keep it safe!
- **Backup**: Save your `identity.json` file to restore your identity later
- **Public Keys**: Your Peer ID is derived from your public key - it's safe to share
- **Fingerprint**: Share your fingerprint with others to verify your identity

## Features

### Current

✅ Text messaging (encrypted)  
✅ Peer-to-peer networking  
✅ Auto peer discovery (mDNS + DHT)  
✅ Identity management  
✅ Voice call infrastructure

### Coming Soon

🚧 Group chats  
🚧 File transfer  
🚧 Voice/video calls  
🚧 Multi-device support  

## Getting Help

- **GitHub**: https://github.com/MhaWay/Otter
- **Documentation**: See `README.md` and `USAGE.md` in the repository
- **Issues**: Report bugs on GitHub Issues

## Building from Source

If you want to build Otter yourself:

```bash
# Clone the repository
git clone https://github.com/MhaWay/Otter.git
cd Otter

# Build release
cargo build --release -p otter-cli

# Or use Make
make release
```

## License

Otter is open source software licensed under MIT or Apache-2.0.

---

**Happy chatting! 🦦**

Remember: Otter is designed for privacy and decentralization. Your conversations are yours alone.
