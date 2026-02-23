# Sprint 1: Bootstrap Integration into Node Startup

**Objective:** Integrate `BootstrapSources` into the node startup sequence so bootstrap happens **automatically** when the node starts.

**Status:** 📋 Planning

---

## Current State

### ✅ What's Ready
- `BootstrapSources` module: Fully implemented and tested
- Real network validation: PASSED (2.3sec to first peer)
- DNS resolution: Working (4 bootstrap peers discovered)
- Cache persistence: Verified

### ❌ What's Missing
- Bootstrap not integrated into actual node startup
- Currently only works in `bootstrap_test.rs` example
- Nodes don't auto-discover peers on startup

---

## Integration Points

### File: `crates/otter-cli/src/main.rs`

**Functions to modify:**
1. `run_simple_mode()` - CLI auto-startup
2. `start_peer()` - Named peer startup

**Current flow:**
```rust
let (event_tx, mut event_rx, command_tx, command_rx) = create_network_channels();
let mut network = Network::new(event_tx, command_rx)?;
network.listen(&listen_addr)?;
let network_handle = tokio::spawn(async move { network.run().await });
```

**New flow:**
```rust
let (event_tx, mut event_rx, command_tx, command_rx) = create_network_channels();
let mut network = Network::new(event_tx, command_rx)?;
network.listen(&listen_addr)?;

// 🆕 BOOTSTRAP INTEGRATION HERE
let mut bootstrap = BootstrapSources::new(cache_path);
bootstrap.initialize().await?;
let bootstrap_peers = bootstrap.bootstrap().await;
for addr in bootstrap_peers {
    network.dial(&addr)?;  // Need to add method
}

let network_handle = tokio::spawn(async move { network.run().await });
```

---

## Implementation Tasks

### Task 1: Add public methods to `Network`

```rust
impl Network {
    /// Dial a peer by multiaddr (used by bootstrap)
    pub fn dial(&mut self, addr: &Multiaddr) -> Result<(), NetworkError> {
        self.swarm.dial(addr.clone())
            .map_err(|e| NetworkError::TransportError(e.to_string()))
    }
    
    /// Get reference to Kademlia for manual peer addition
    pub fn kademlia(&mut self) -> &mut kad::Behaviour<kad::store::MemoryStore> {
        &mut self.swarm.behaviour_mut().kad
    }
    
    /// Add peer address to DHT routing table (used by bootstrap gossip)
    pub fn add_dht_peer(&mut self, peer_id: &PeerId, addr: &Multiaddr) {
        self.swarm
            .behaviour_mut()
            .kad
            .add_address(peer_id, addr.clone());
    }
}
```

**File:** `crates/otter-network/src/lib.rs`  
**Complexity:** 🟢 Low (3 simple methods)

---

### Task 2: Create bootstrap startup function

**File:** `crates/otter-cli/src/main.rs`

```rust
async fn bootstrap_network(
    network: &mut Network,
    bootstrap: &mut BootstrapSources,
) -> Result<()> {
    println!("🌍 Starting bootstrap discovery...");
    
    // Tier 2 + Tier 1: Get all bootstrap peers
    let peers = bootstrap.bootstrap().await;
    println!("✓ Discovered {} bootstrap peers", peers.len());
    
    // Try to connect to first few peers
    for (i, addr) in peers.iter().take(5).enumerate() {
        debug!("Dialing bootstrap peer {}/{}: {}", i + 1, peers.len().min(5), addr);
        
        // Extract peer_id for DHT tracking
        if let Ok(peer_id) = extract_peer_id_from_multiaddr(addr) {
            network.add_dht_peer(&peer_id, addr)?;
        }
        
        // Attempt connection
        if let Err(e) = network.dial(addr) {
            warn!("Failed to dial bootstrap peer {}: {}", addr, e);
        }
    }
    
    println!("✓ Bootstrap initialization complete");
    Ok(())
}

fn extract_peer_id_from_multiaddr(addr: &Multiaddr) -> Result<PeerId> {
    // Parse /p2p/... suffix from multiaddr
    addr.iter()
        .find_map(|proto| match proto {
            libp2p::multiaddr::Protocol::P2p(peer_id) => Some(Ok(peer_id)),
            _ => None,
        })
        .unwrap_or_else(|| Err(anyhow!("No P2P protocol in multiaddr")))
}
```

**Complexity:** 🟠 Medium (coordination logic)

---

### Task 3: Integrate into `run_simple_mode()`

**Location:** Line ~270 in `main.rs`

```rust
// After network.listen()
network.listen(&listen_addr)?;

// 🆕 Bootstrap peers
let cache_path = data_dir.join("peer_cache.json");
let mut bootstrap = BootstrapSources::new(cache_path);
if let Err(e) = bootstrap.initialize().await {
    warn!("Bootstrap initialization warning: {}", e);
}
bootstrap_network(&mut network, &mut bootstrap).await?;

// Then spawn network task
let network_handle = tokio::spawn(async move { network.run().await });
```

**Impact:** Changes ~5 lines  
**Risk:** 🟢 Low (additive only)

---

### Task 4: Integrate into `start_peer()`

**Location:** Line ~450 in `main.rs`

Same pattern as Task 3 - add bootstrap after `network.listen()`.

**Impact:** Changes ~5 lines  
**Risk:** 🟢 Low (additive only)

---

## Expected Behavior After Integration

### Before (Current)
```
$ otter-cli --cli
✓ Network started
✓ Listening for peers on the network...
→ No peers connect (relies on mDNS only, won't work over internet)
```

### After (Sprint 1)
```
$ otter-cli --cli
🌍 Starting bootstrap discovery...
✅ Resolved 4 addresses from libp2p bootstrap
✓ Dialing bootstrap peer 1/4: /dnsaddr/ny5.bootstrap.libp2p.io/...
✓ Dialing bootstrap peer 2/4: /dnsaddr/sg1.bootstrap.libp2p.io/...
✓ Bootstrap initialization complete
🎉 First peer connected in 2.3s
✓ Connected: QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa
✓ Network started
✓ Listening for peers on the network...
→ Already discovering more peers via gossip
```

---

## Testing Plan

### Unit Test
```bash
cargo test -p otter-network --lib bootstrap
→ Should still pass (no changes to module)
```

### Integration Test
```bash
cargo run --bin otter-cli -- --cli
→ Should show bootstrap peers in logs
→ Should connect to at least 1 peer automatically
→ Cache should be populated after first run
```

### Persistence Test
```bash
# First run
$ cargo run --bin otter-cli -- --cli
→ Connects and populates cache

# Second run (no network)
$ cargo run --bin otter-cli -- --cli
→ Should load from cache
→ Can connect without DNS (Tier 3)
```

---

## Dependencies

**Before this task:**
- ✅ `BootstrapSources` fully functional
- ✅ Real network validation passed
- ✅ DNS resolver enabled in `Cargo.toml`

**After this task:**
- ✅ Node auto-discovers peers on startup
- ✅ Gossip discovery (Tier 3) can begin
- ✅ Ready for Sprint 2 (NAT traversal)

---

## Estimated Effort

| Task | Complexity | Time [hours] |
|------|-----------|-------------|
| Add Network methods | 🟢 Low | 0.5 |
| Create bootstrap function | 🟠 Medium | 1.5 |
| Integrate into run_simple_mode | 🟢 Low | 0.5 |
| Integrate into start_peer | 🟢 Low | 0.5 |
| Testing + debugging | 🟠 Medium | 1.5 |
| **Total** | | **4.5 hours** |

---

## Success Criteria

✅ Node discovers bootstrap peers on startup  
✅ First peer connected within 15 seconds  
✅ Cache populated and persisted  
✅ Reputation tracking works  
✅ Logs show clear bootstrap sequence  
✅ No breaking changes to existing tests  
✅ Commit includes integration example

---

## Post-Sprint 1 (Sprint 2+)

Once bootstrap is integrated:

1. **Gossip Discovery (Tier 3)**
   - Request peer lists from connected nodes
   - Feeds decentralized discovery

2. **NAT Traversal**
   - AutoNAT detection
   - DCUTR hole punching
   - Relay support

3. **Background Tasks**
   - Periodic DHT refresh
   - Connection health monitoring

---

## Files to Modify

- ✏️ `crates/otter-network/src/lib.rs` (+20 lines)
- ✏️ `crates/otter-cli/src/main.rs` (+50 lines)
- ➕ Import `BootstrapSources` in CLI

**Total new lines:** ~70  
**Churn:** Minimal (mostly additive)

---

**Ready to implement? 🚀**
