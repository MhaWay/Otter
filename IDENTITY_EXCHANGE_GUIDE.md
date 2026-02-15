# Quick Guide: Identity Exchange in Otter

## What You Need to Know

Identity exchange is **automatic** - it happens as soon as two peers connect!

## Visual Flow

```
┌─────────────┐                                      ┌─────────────┐
│   Peer A    │                                      │   Peer B    │
│  (Alice)    │                                      │   (Bob)     │
└──────┬──────┘                                      └──────┬──────┘
       │                                                    │
       │  1. Discover via mDNS/DHT                         │
       │◄──────────────────────────────────────────────────┤
       │                                                    │
       │  2. Establish P2P Connection                      │
       │◄─────────────────┬────────────────────────────────┤
       │                  │                                 │
       │  3. Auto-send Identity Message                    │
       ├─────────────────►│                                 │
       │                  │  Message::Identity(Alice)       │
       │                  └────────────────────────────────►│
       │                                                    │
       │  4. Receive Bob's Identity                        │
       │◄───────────────────────────────────────────────────┤
       │  Message::Identity(Bob)                           │
       │                                                    │
       │  5. Register each other's public keys             │
       ├─ register_peer(Bob) ─┐                           │
       │                       │  ┌─ register_peer(Alice) ─┤
       │                       ▼  ▼                        │
       │  6. Create Crypto Sessions (X25519 key exchange)  │
       ├───────────────────────────────────────────────────┤
       │                                                    │
       │  ✓ Ready for Encrypted Communication              │
       │                                                    │
       │  7. Send Encrypted Message                        │
       ├──────────────────►│                               │
       │  ChaCha20-Poly1305│                               │
       │                   └──────────────────────────────►│
       │                                                    │
       │  8. Decrypt and Read                              │
       │                                       "Hello Bob!" │
       │                                                    │
       │  9. Reply with Encrypted Message                  │
       │◄──────────────────────────────────────────────────┤
       │  "Hi Alice!"                                       │
       │                                                    │
└─────┴────────────────────────────────────────────────────┴─────┘
```

## Step-by-Step: What Happens Automatically

### Step 1: Peer Discovery
- **mDNS**: Finds peers on local network
- **Kademlia DHT**: Discovers peers globally
- **Manual**: Connect via peer ID

### Step 2: P2P Connection
- libp2p establishes encrypted transport
- Both peers receive `PeerConnected` event

### Step 3: Identity Announcement
- **Each peer automatically**:
  - Creates `Message::Identity` with their public keys
  - Sends via `NetworkCommand::SendMessage`
  - Broadcasts to connected peer

### Step 4: Identity Reception
- Peer receives `MessageReceived` event
- Deserializes identity message
- Extracts public keys

### Step 5: Peer Registration
- `MessageHandler::register_peer(public_identity)` called
- Stores peer's public keys
- Peer ID → Public Keys mapping created

### Step 6: Crypto Session
- **X25519 Key Exchange** performed
- Shared secret derived
- Session ready for encryption/decryption

### Step 7-9: Encrypted Messaging
- Messages encrypted with ChaCha20-Poly1305
- Only recipient can decrypt
- Authenticity verified via signatures

## What You See in Console

```bash
$ ./otter

╔══════════════════════════════════════════════════════════════╗
║          🦦 Otter - Decentralized Private Chat              ║
╚══════════════════════════════════════════════════════════════╝

🆔 Peer ID:     ABC123...
🔑 Fingerprint: 2945f80a
📁 Data Dir:    ~/.otter

🚀 Starting Otter peer...

✓ Network started successfully
✓ Listening for peers on the network...

✓ Discovered peer: XYZ789...

✓ Connected: XYZ789...
  ✓ Identity sent                    ← AUTOMATIC!

✓ Identity verified for peer: XYZ789...  ← AUTOMATIC!

💬 Ready for encrypted chat!
```

## What Each Line Means

| Console Output | What It Means |
|---------------|---------------|
| `✓ Discovered peer: <id>` | Found a peer via mDNS or DHT |
| `✓ Connected: <id>` | P2P connection established |
| `✓ Identity sent` | Your public keys sent to peer |
| `✓ Identity verified for peer: <id>` | Received and verified peer's public keys |

## After Identity Exchange

Once you see "Identity verified", you can:

✅ **Send encrypted messages**: `/send`  
✅ **Start voice calls**: `/call`  
✅ **View peer info**: `/peers`  

## Security Notes

### What's Protected
- ✅ Messages encrypted end-to-end
- ✅ Public keys authenticated
- ✅ Peer ID cryptographically bound to keys

### What You Should Do
1. **Verify Fingerprints**: Compare fingerprints with peer out-of-band
   ```
   Your fingerprint: 2945f80a
   Peer fingerprint: Check with peer via phone/in-person
   ```

2. **Trust On First Use (TOFU)**: Accept first identity, be suspicious of changes

3. **Key Pinning**: Otter remembers peer identities across sessions

### What's NOT Protected
- ❌ Network metadata (who you're talking to)
- ❌ Connection timing
- ❌ Peer discovery (mDNS broadcasts locally)

## Troubleshooting

### "Peer not found" error
**Problem**: Identity exchange didn't complete  
**Solution**: Wait for "Identity verified" message, or reconnect

### No encrypted messages received
**Problem**: Crypto session not established  
**Check**: Did both peers show "Identity verified"?

### Different Peer ID than expected
**Problem**: Peer regenerated identity or MITM  
**Solution**: Verify fingerprint out-of-band before proceeding

## Manual Identity Announcement

Identity is sent automatically, but you can also manually announce:

```rust
// Future feature: /announce command
// For now, reconnect triggers new exchange
```

## For Developers

### Sending Identity Programmatically

```rust
use otter_messaging::Message;
use otter_network::NetworkCommand;

// Create identity message
let identity_msg = Message::identity(handler.public_identity());
let data = identity_msg.to_bytes()?;

// Send to specific peer
command_tx.send(NetworkCommand::SendMessage {
    to: peer_id,
    data,
}).await?;
```

### Receiving Identity

```rust
match message {
    Message::Identity { public_identity, .. } => {
        // Register peer
        handler.register_peer(public_identity)?;
        println!("✓ Identity registered");
    }
    // ... other message types
}
```

### Testing Identity Exchange

```bash
# Terminal 1
./otter --nickname Alice --port 9000

# Terminal 2
./otter --nickname Bob --port 9001

# Both should show:
# ✓ Identity sent
# ✓ Identity verified

# Try encrypted messaging:
# Alice> /send
# Select Bob
# Message: Hello!
```

## FAQ

**Q: When does identity exchange happen?**  
A: Automatically, immediately after peer connection.

**Q: Can I disable automatic exchange?**  
A: No, it's required for encrypted communication.

**Q: What if identity exchange fails?**  
A: Reconnect or restart Otter. Check logs for errors.

**Q: How do I verify a peer's identity?**  
A: Compare fingerprints out-of-band (phone, in-person).

**Q: Can peers impersonate each other?**  
A: No, Peer IDs are derived from public keys cryptographically.

**Q: What happens if a peer changes keys?**  
A: Their Peer ID changes. Otter should warn about this.

## Learn More

- **IDENTITY_EXCHANGE.md**: Full technical documentation
- **ARCHITECTURE.md**: System design overview
- **SECURITY_ENHANCEMENTS.md**: Security features

## Summary

🎉 **Identity exchange is automatic** - just connect and it works!

1. Peers connect
2. Identities exchanged automatically
3. Crypto sessions established
4. ✓ Ready for encrypted chat

No manual configuration needed!
