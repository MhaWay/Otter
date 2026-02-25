# 🏠 Otter GUI Home Tab - Network Status Dashboard

## VISUAL MOCKUP - How It Looks

```
╔════════════════════════════════════════════════════════════════════════════╗
║                        🏠 Otter Network Status                              ║
╠════════════════════════════════════════════════════════════════════════════╣
║                                                                              ║
║  ┌──────────────────────────────────────────────────────────────────────┐  ║
║  │                           🟢 ONLINE                                    │  ║
║  │                 Rete connessa e pronta                                 │  ║
║  │                                                                        │  ║
║  └──────────────────────────────────────────────────────────────────────┘  ║
║                                                                              ║
║  ┌──────────────────────────────────────────────────────────────────────┐  ║
║  │    📡 Bootstrap: Connected | DHT Peers: 4                            │  ║
║  └──────────────────────────────────────────────────────────────────────┘  ║
║                                                                              ║
║  ┌──────────────────────────────────────────────────────────────────────┐  ║
║  │  🔊 Listening Addresses                                              │  ║
║  │     • /ip4/127.0.0.1/tcp/30333                                       │  ║
║  │     • /ip4/192.168.1.100/tcp/30333                                   │  ║
║  │     • /ip6/::1/tcp/30333                                             │  ║
║  └──────────────────────────────────────────────────────────────────────┘  ║
║                                                                              ║
║  ┌──────────────────────────────────────────────────────────────────────┐  ║
║  │  📊 Network Health Metrics                                           │  ║
║  │                                                                        │  ║
║  │     👥 Connected Peers         12                                     │  ║
║  │     ⏱️  Avg Latency            245 ms                                 │  ║
║  │     ❌ Error Rate              2.1%                                   │  ║
║  │     🗂️  DHT Size               48 peers                              │  ║
║  │     🕐 Updated: 14:35:22                                             │  ║
║  │                                                                        │  ║
║  └──────────────────────────────────────────────────────────────────────┘  ║
║                                                                              ║
║  ┌──────────────────────────────────────────────────────────────────────┐  ║
║  │  👥 Discovered Peers (5)                                             │  ║
║  │                                                                        │  ║
║  │     🟢 Alice (12D3KooW...)                                            │  ║
║  │     🟢 Bob (12D3KooW...)                                              │  ║
║  │     🟢 Carol (12D3KooW...)                                            │  ║
║  │     🔴 David (12D3KooW...)                                            │  ║
║  │     🟢 Eve (12D3KooW...)                                              │  ║
║  │                                                                        │  ║
║  └──────────────────────────────────────────────────────────────────────┘  ║
║                                                                              ║
║  ┌──────────────────────────────────────────────────────────────────────┐  ║
║  │  📝 Network Log                                  [scrollable area]    │  ║
║  │                                                                        │  ║
║  │     ✓ Rete pronta (mesh peers: 3)                                    │  ║
║  │     ✓ In ascolto su /ip4/127.0.0.1/tcp/30333                         │  ║
║  │     🔍 Ricerca peer (connessi: 2)                                    │  ║
║  │     ✓ Peer online: 12D3KooWBvvVGDJXhQpG...                          │  ║
║  │     ✓ Peer online: 12D3KooWGKYjWB5JKCJCr...                         │  ║
║  │     📂 Caricate 3 peer precedenti                                    │  ║
║  │     ✓ Peer online: 12D3KooWRZKYXkpqXtN7c...                         │  ║
║  │     ✓ Peer online: 12D3KooWA8EXV3KjBxEU...                          │  ║
║  │                                                                        │  ║
║  └──────────────────────────────────────────────────────────────────────┘  ║
║                                                                              ║
╚════════════════════════════════════════════════════════════════════════════╝
```

## COLOR SCHEME

| Element | Color | RGB | Usage |
|---------|-------|-----|-------|
| Title/Highlights | Bright Blue | `#2072E8` | Primary information |
| ONLINE Status | Bright Green | `#00CC33` | Network ready |
| CONNECTING Status | Yellow | `#FFCC00` | In progress |
| DEGRADED Status | Orange | `#FF9900` | Warning state |
| ERROR Status | Red | `#FF0000` | Error state |
| Peer Online | Green | `#00CC33` | Peer connected |
| Peer Offline | Gray | `#B0B0B0` | Peer disconnected |
| Healthy Metric | Green | `#00CC33` | Good health |
| Warning Metric | Orange | `#FF9900` | Poor health |
| Info Text | Light Blue | `#6699FF` | Secondary info |
| Normal Text | Light Gray | `#CCCCCC` | Default text |

---

## METRIC EXAMPLES - Different Network States

### ✅ HEALTHY NETWORK
```
Status: 🟢 ONLINE
Bootstrap: Connected | DHT Peers: 8
Listening: 3 addresses
Connected Peers: 15
Avg Latency: 120 ms
Error Rate: 1.2% (GREEN)
DHT Size: 64 peers
```

### ⚠️ DEGRADED NETWORK
```
Status: 🟠 DEGRADED
Bootstrap: Connected | DHT Peers: 2
Listening: 2 addresses
Connected Peers: 3
Avg Latency: 450 ms
Error Rate: 8.5% (ORANGE)
DHT Size: 12 peers
```

### 🔴 CONNECTION ERROR
```
Status: 🔴 ERROR
Bootstrap: Disconnected
DHT Peers: 0
Listening: None
Connected Peers: 0
Avg Latency: N/A
Error Rate: 100% (RED)
DHT Size: 0 peers
```

---

## INTERACTIVE FEATURES

### Not Yet Implemented (Future)
- [ ] Click on peer name to see full peer ID and details
- [ ] Refresh button to manually trigger peer discovery
- [ ] Copy button for peer IDs
- [ ] Export network log to file
- [ ] Network health history graph
- [ ] Ping latency to specific peer

### Currently Working
- ✅ Auto-update as network events arrive
- ✅ Scrollable network log
- ✅ Real-time status indicator
- ✅ Color-coded health metrics
- ✅ Connected peer list with online status

---

## DATA SOURCES

All dashboard data comes from **active network events**:

| Section | Data Source | Update Trigger |
|---------|-------------|-----------------|
| Status | `network_state` field | NetworkEvent::NetworkReady/Degraded |
| Bootstrap | `bootstrap_connected_count` | NetworkEvent::NetworkReady |
| Addresses | `listening_addresses` vec | NetworkEvent::ListeningOn |
| Metrics | `HealthReport` struct | NetworkEvent::HealthReport (periodic) |
| Peers | `discovered_peers` vec | NetworkEvent::PeerOnline/PeerOffline |
| Log | `loading_logs` vec | All N NetworkEvents |

---

## PERFORMANCE CONSIDERATIONS

- **Memory**: Stores last 8 log entries (minimal footprint)
- **Display**: Shows max 10 peers (scrollable for more)
- **Updates**: Real-time as events arrive (no polling)
- **Rendering**: Only affected sections re-render
- **Scrollable**: Log section is scrollable for long histories

---

## USER WORKFLOWS

### Workflow 1: Initial Network Verification
1. Launch Otter app
2. **Home tab shows**: "🟡 CONNECTING"
3. Wait for status change to "🟢 ONLINE"
4. Verify DHT Peers > 0
5. Verify Connected Peers > 0
6. ✅ Network is ready

### Workflow 2: Peer Discovery Verification
1. Watch "👥 Discovered Peers" section
2. See peers appear as 🟢 (online)
3. Watch Network Log for "Peer online" events
4. Check Avg Latency for healthy values (<300ms)
5. ✅ Peers are discoverable

### Workflow 3: Troubleshooting
1. Status is 🟠 DEGRADED?
   - Check connected peers count (should be > 0)
   - Check DHT size (should be > 0)
   - Check error rate (aim for < 5%)
2. Check listening addresses (at least 1)
3. Check network log for errors
4. Network Log shows bootstrap issues?
   - Likely firewall blocking
   - Check internet connectivity

---

## STATISTICAL DATA DISPLAYED

### Event Counters (in log)
- Bootstrap connections: Shows connect/ready events
- Peer discoveries: Shows peer online events
- Network state changes: Shows ready/degraded events

### Computed Metrics
- **Peer Count**: Total connected peers
- **Error Rate**: Failed messages / total attempts
- **Avg Latency**: Average response time in milliseconds
- **DHT Size**: Entries in Kademlia routing table

### Timestamps
- Each log entry has network event timestamp
- Health report shows when metrics were last updated
- Last seen status on discovered peers

---

## ACCESSIBILITY

- **Large Text**: 28-32px titles, 12-16px content
- **High Contrast**: White/blue text on dark background
- **Clear Icons**: Emoji status indicators (🟢🟡🟠🔴)
- **Organized Layout**: Logical sections with clear headers
- **Responsive**: Mobile-friendly if deployed on tablet

---

## LOCALIZATION READINESS

Text strings for translation:
- "🏠 Otter Network Status" → Title
- "ONLINE" / "CONNECTING" / "DEGRADED" / "ERROR" → Status
- "Rete connessa e pronta" → Status description
- "Bootstrap: Connected" → Bootstrap status
- "Listening Addresses" → Section header
- "Network Health Metrics" → Section header
- "Connected Peers" → Peer count label
- "Avg Latency" → Latency label
- "Error Rate" → Error rate label
- "DHT Size" → DHT size label
- "Discovered Peers" → Peers section header
- "Network Log" → Log section header

---

## SUMMARY

**What Users See**:
- Clear, at-a-glance network status
- Real-time metrics of network health
- List of connected peers with online status
- Recent network events for debugging

**What Developers See**:
- Proof that network stack is working
- Bootstrap peer connectivity
- DHT routing table population
- Peer discovery mechanism health
- Network event flow and timing

**What Happens Next**:
1. Build APK with enhanced GUI
2. Deploy to Android device
3. User sees dashboard showing:
   - Network connecting
   - Bootstrap peers found
   - Peers discovered
   - Health metrics populated
   - Status changes in real-time
4. User gains confidence network is working correctly
5. User can proceed to messaging features

---

**Status**: ✅ **IMPLEMENTED & COMPILED**  
**Ready for**: Android APK build and device testing
