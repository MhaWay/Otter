# Enum Tag Deserialization Error - Debug Guide

## Problem Description

Users report deserialization errors when receiving encrypted messages:

```
WARN Failed to deserialize message: tag for enum is not valid, found 7
WARN Failed to deserialize message: tag for enum is not valid, found 128
```

## Understanding the Error

### Message Enum Variants

The `Message` enum has 5 variants, indexed 0-4:

```rust
pub enum Message {
    Text = 0,        // Plain text
    Identity = 1,    // Public key exchange  
    Encrypted = 2,   // Encrypted message (THIS ONE FAILS)
    Status = 3,      // Status update
    Typing = 4,      // Typing indicator
}
```

### What the Error Means

Bincode serialization stores enum variants as their index. When deserializing:
- Tag 0-4: Valid (matches enum variants)
- Tag 7, 128, etc.: **INVALID** - data is corrupted or not a Message enum

## Current Status

**What Works ✅:**
- Identity messages (variant 1) - These work perfectly
- Network transmission via gossipsub
- Message encryption/decryption

**What Fails ❌:**
- Encrypted text messages (variant 2) - Invalid tag error
- Messages appear sent but never received
- Deserialization fails on receiver side

## Debugging Steps

### 1. Collect Debug Logs

Run both peers with debug logging:

```bash
# Terminal 1 (Alice)
RUST_LOG=debug ./target/release/otter --nickname Alice 2>&1 | tee alice-debug.log

# Terminal 2 (Bob) 
RUST_LOG=debug ./target/release/otter --nickname Bob --port 9001 2>&1 | tee bob-debug.log
```

### 2. Send a Test Message

In one terminal, use `/send` to send a message. This will trigger the error.

### 3. Examine the Logs

Look for these debug lines:

**On sender side:**
```
DEBUG Serialized to X bytes
DEBUG First 32 bytes as hex: 020000...
DEBUG Enum tag (first byte): 2
```

**On receiver side:**
```
DEBUG Received X bytes from ...
DEBUG First 32 bytes as hex: 070000... ← DIFFERENT!
DEBUG Enum tag (first byte): 7 ← WRONG!
WARN Failed to deserialize message: tag for enum is not valid, found 7
```

### 4. Compare Hex Dumps

- **If hex dumps match**: Problem is in bincode deserialization
- **If hex dumps differ**: Problem is in network transmission
- **If tag differs**: Corruption or transformation occurring

## Common Causes

### 1. Bincode Version Mismatch

If bincode version changed between builds, serialization format may differ.

**Check:**
```bash
cargo tree -p otter-messaging | grep bincode
```

**Solution:**
Ensure consistent bincode version across all builds.

### 2. Double Serialization

If message is being serialized twice somewhere in the chain.

**Check:**
Look for nested `to_bytes()` calls or unnecessary wrapping.

**Solution:**
Remove redundant serialization layers.

### 3. Gossipsub Corruption

Network layer modifying bytes during transmission.

**Check:**
Compare sender hex dump with receiver hex dump.

**Solution:**
Verify gossipsub configuration, check for middleware.

### 4. Struct Layout Changes

If `EncryptedMessage` or `Message` struct changed.

**Check:**
Ensure both peers built from same codebase.

**Solution:**
Rebuild both peers from same source.

## Proposed Fixes

### Fix 1: Explicit Bincode Configuration

Use fixed-int encoding to ensure consistent format:

```rust
pub fn to_bytes(&self) -> Result<Vec<u8>, MessagingError> {
    let config = bincode::DefaultOptions::new()
        .with_little_endian()
        .with_fixint_encoding();
    
    config.serialize(self)
        .map_err(|e| MessagingError::SerializationError(e.to_string()))
}

pub fn from_bytes(bytes: &[u8]) -> Result<Self, MessagingError> {
    let config = bincode::DefaultOptions::new()
        .with_little_endian()
        .with_fixint_encoding();
        
    config.deserialize(bytes)
        .map_err(|e| MessagingError::SerializationError(e.to_string()))
}
```

### Fix 2: Switch to JSON

More portable, easier to debug:

```rust
// Already implemented!
pub fn to_json(&self) -> Result<String, MessagingError>
pub fn from_json(json: &str) -> Result<Self, MessagingError>
```

Change CLI to use JSON instead of bincode for encrypted messages.

### Fix 3: Message Envelope

Add versioning and integrity:

```rust
#[derive(Serialize, Deserialize)]
struct MessageEnvelope {
    version: u8,
    message: Message,
    checksum: [u8; 32],
}
```

### Fix 4: Direct EncryptedMessage

Send `EncryptedMessage` directly without Message enum wrapper:

```rust
// Instead of: Message::Encrypted { ... }
// Just send: EncryptedMessage
```

## Testing

After applying a fix:

```bash
# Rebuild
cargo build --release -p otter-cli

# Test with two terminals
./target/release/otter --nickname Alice
./target/release/otter --nickname Bob --port 9001

# Send messages both ways
# Verify no deserialization errors
# Confirm messages display correctly
```

## Next Steps

1. **Collect hex dumps** from debug logs
2. **Identify mismatch point** (serialization vs transmission)
3. **Apply appropriate fix** based on findings
4. **Test thoroughly** with multiple message exchanges

## Additional Resources

- See `DESERIALIZATION_ERROR_DEBUG.md` for more details
- Check `MESSAGE_SENDING_FIX.md` for related fixes
- Bincode docs: https://docs.rs/bincode/

## Status

- ✅ Debug logging implemented
- ✅ Hex dump capability added
- ⏳ Waiting for user hex dump data
- ⏳ Root cause to be determined
- ⏳ Fix to be applied once diagnosed
