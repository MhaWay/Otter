# Otter Usage Examples

This document provides practical examples of using the Otter decentralized chat platform.

## Basic Usage

### 1. Generate Your Identity

First, create your cryptographic identity:

```bash
otter init
```

This creates an `identity.json` file containing your Ed25519 signing keys and X25519 encryption keys.

Example output:
```
✓ Identity generated successfully!
  Peer ID: 2q9ncWzzw9fxaH3c2bGmQy4KGxgRLa9FuNKi2kK1mXsH
  Saved to: identity.json

To start chatting, run:
  otter start -i identity.json
```

**Important**: Keep your `identity.json` file secure! It contains your private keys.

### 2. View Your Identity

To see your identity information:

```bash
otter info
```

Example output:
```
Identity Information
====================
Peer ID: 2q9ncWzzw9fxaH3c2bGmQy4KGxgRLa9FuNKi2kK1mXsH
Public Key: ac091d8a643df934a149cf02211042ebebaf23ba4a1a0c68f984dd7f48b259db
File: identity.json
```

### 3. Start Your Peer

Start the chat peer and begin listening for connections:

```bash
otter start
```

By default, it listens on a random port. To specify a port:

```bash
otter start -p 9000
```

Example output:
```
🦦 Otter Chat - Decentralized & Private
========================================
Peer ID: 2q9ncWzzw9fxaH3c2bGmQy4KGxgRLa9FuNKi2kK1mXsH

✓ Network started
✓ Listening for peers...

Commands:
  /peers  - List connected peers
  /send   - Send a message to a peer
  /help   - Show this help
  /quit   - Exit

otter> 
```

## Two-Peer Chat Example

Here's how to set up a chat between two peers:

### Terminal 1 - Alice

```bash
# Create Alice's identity
otter init -o alice_identity.json

# Start Alice's peer
otter start -i alice_identity.json -p 9000
```

### Terminal 2 - Bob

```bash
# Create Bob's identity  
otter init -o bob_identity.json

# Start Bob's peer
otter start -i bob_identity.json -p 9001
```

### Peer Discovery

On the same local network, peers will automatically discover each other via mDNS:

```
✓ Discovered peer: 12D3KooWXYZ...
✓ Connected: 12D3KooWXYZ...
✓ Identity verified for peer: 2q9ncWzzw9fxaH3c2bGmQy4KGxgRLa9FuNKi2kK1mXsH
```

## Interactive Commands

Once your peer is running, you can use these commands:

### List Connected Peers

```
otter> /peers
```

Example output:
```
Connected Peers:
  1. 2q9ncWzzw9fxaH3c2bGmQy4KGxgRLa9FuNKi2kK1mXsH
  2. 3rAmdXyyz8gybI4d3cHnRz5LHyhSMb0GvOLj3lL2nYtI
```

### Send an Encrypted Message

```
otter> /send
```

Follow the prompts:
1. Select the peer to message
2. Type your message
3. Message is automatically encrypted and sent

Example:
```
Select a peer:
> 1. 2q9ncWzzw9fxaH3c2bGmQy4KGxgRLa9FuNKi2kK1mXsH
  2. 3rAmdXyyz8gybI4d3cHnRz5LHyhSMb0GvOLj3lL2nYtI

Message: Hello, this is a secret message!

✓ Message encrypted and sent!
```

### Receiving Messages

When you receive a message, it appears automatically:

```
🔐 Encrypted message from 2q9ncWzzw9fxaH3c2bGmQy4KGxgRLa9FuNKi2kK1mXsH: Hello, this is a secret message!
```

### Get Help

```
otter> /help
```

### Exit

```
otter> /quit
```

## Multiple Identities

You can create multiple identities for different purposes:

```bash
# Work identity
otter init -o work_identity.json

# Personal identity
otter init -o personal_identity.json

# Start with specific identity
otter start -i work_identity.json
```

## Network Configuration

### Custom Port

```bash
otter start -p 8080
```

### Different Identity File

```bash
otter start -i my_custom_identity.json
```

### Both Custom Port and Identity

```bash
otter start -i alice_identity.json -p 9000
```

## Security Best Practices

1. **Protect Your Identity File**: Your `identity.json` contains your private keys
   ```bash
   chmod 600 identity.json
   ```

2. **Back Up Your Identity**: Without it, you lose your identity
   ```bash
   cp identity.json identity.json.backup
   ```

3. **One Identity Per Device**: Don't share identity files between devices
   - Use different identities on each device
   - Or implement proper key synchronization

4. **Verify Peer Identities**: When connecting to peers, verify their Peer IDs through a secondary channel

## Advanced Scenarios

### Running on a Server

To run Otter on a server, you might want to:

```bash
# Listen on all interfaces
otter start -i server_identity.json -p 9000
```

### Testing Locally

For testing, run multiple peers locally:

```bash
# Terminal 1
otter start -i peer1.json -p 9001

# Terminal 2  
otter start -i peer2.json -p 9002

# Terminal 3
otter start -i peer3.json -p 9003
```

### Scripting

You can script identity generation:

```bash
#!/bin/bash
for i in {1..5}; do
    otter init -o "peer${i}_identity.json"
done
```

## Troubleshooting

### "Identity file already exists"

```bash
# Use a different filename
otter init -o new_identity.json

# Or remove the existing file
rm identity.json
otter init
```

### "Failed to read identity file"

```bash
# Check if file exists
ls -l identity.json

# Check file permissions
chmod 600 identity.json

# Verify file is valid JSON
cat identity.json | jq .
```

### "No connected peers"

- Ensure both peers are on the same local network
- Check firewall settings
- Verify both peers are running
- Try specifying port explicitly

### mDNS Errors

If you see mDNS errors like "Operation not permitted":
- This is usually due to lack of multicast permissions
- On Linux, you might need `CAP_NET_RAW` capability
- These errors don't prevent the chat from working, but may affect peer discovery

## Integration Examples

### Using in Scripts

```bash
#!/bin/bash

# Check if identity exists
if [ ! -f identity.json ]; then
    echo "Creating identity..."
    otter init
fi

# Start peer
otter start -p 9000
```

### Systemd Service (Linux)

Create `/etc/systemd/system/otter.service`:

```ini
[Unit]
Description=Otter Decentralized Chat
After=network.target

[Service]
Type=simple
User=otter
WorkingDirectory=/home/otter
ExecStart=/usr/local/bin/otter start -i /home/otter/identity.json -p 9000
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

Then:
```bash
sudo systemctl daemon-reload
sudo systemctl enable otter
sudo systemctl start otter
```

## What's Next?

- Explore the [Architecture](ARCHITECTURE.md) documentation
- Check out the [README](README.md) for build instructions
- Review the code in the `crates/` directory
- Contribute improvements via pull requests!
