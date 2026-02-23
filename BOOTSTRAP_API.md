# 🚀 Bootstrap API - Documentazione Completa

## 📋 Panoramica

Il modulo `bootstrap` implementa un'architettura a **3 tier** per la scoperta peer:

1. **Tier 1:** DNS multiaddr (runtime da fonti pubbliche)
2. **Tier 2:** Peer cache locale (persistente con TTL 72h)
3. **Tier 3:** Peer gossip (decentralizzato via peer exchange)

---

## 🔌 API Pubblica

### Struttura Principale: `BootstrapSources`

```rust
pub struct BootstrapSources {
    cache: PeerCache,
    cache_path: PathBuf,
    dns_cache: HashMap<String, (Vec<Multiaddr>, SystemTime)>,
    relay_peers: HashMap<PeerId, Vec<Multiaddr>>,
}
```

---

## 🎯 Funzioni Essenziali

### 1. Inizializzazione

```rust
/// Crea un nuovo BootstrapSources
pub fn new(cache_path: PathBuf) -> Self

/// Inizializza dalla cache su disco
pub async fn initialize(&mut self) -> Result<(), BootstrapError>
```

**Esempio d'uso:**
```rust
use std::path::PathBuf;

let cache_path = PathBuf::from("~/.otter/peer_cache.json");
let mut bootstrap = BootstrapSources::new(cache_path);
bootstrap.initialize().await?;
```

---

### 2. Bootstrap Completo (All-in-One)

```rust
/// Esegue la sequenza completa di bootstrap (tutti i 3 tier)
/// Ritorna tutti i multiaddr scoperti ordinati per priorità
pub async fn bootstrap(&mut self) -> Vec<Multiaddr>
```

**Workflow interno:**
```
1. Carica Tier 2 (cache locale) → peer con alta reputation
2. Se < 5 peer → Tier 1 (DNS bootstrap)
3. Tier 3 (gossip) avviene dopo connessione nel network layer
```

**Esempio d'uso:**
```rust
let mut bootstrap = BootstrapSources::new(cache_path);
bootstrap.initialize().await?;

let peers = bootstrap.bootstrap().await;
println!("Found {} bootstrap peers", peers.len());

for addr in peers {
    swarm.dial(addr)?;
}
```

---

### 3. Tier 1: DNS Bootstrap

```rust
/// Risolve DNS TXT records da domini pubblici
/// Usa DNS-over-HTTPS per privacy
pub async fn resolve_dns_bootstrap(&mut self) 
    -> Result<Vec<Multiaddr>, BootstrapError>
```

**Domini interrogati:**
- `_dnsaddr.bootstrap.libp2p.io`
- `_dnsaddr.discovery.ipfs.io`
- `dnsaddr.bootstrap.libp2p.io`

**Caching:** 24h TTL per ridurre lookup DNS

**Esempio d'uso:**
```rust
let dns_peers = bootstrap.resolve_dns_bootstrap().await?;
println!("DNS discovered: {} peers", dns_peers.len());
```

---

### 4. Tier 2: Cache Locale

```rust
/// Carica peer dalla cache persistente (~/.otter/peer_cache.json)
/// Ritorna peer ordinati per reputation score
pub async fn load_cached_peers(&mut self) 
    -> Result<Vec<Multiaddr>, BootstrapError>
```

**Formato cache JSON:**
```json
{
  "version": 1,
  "last_updated": 1708700000,
  "peers": [
    {
      "peer_id": "12D3KooW...",
      "addresses": ["/ip4/1.2.3.4/tcp/4001"],
      "last_seen": 1708699000,
      "successful_dials": 15,
      "failed_dials": 2,
      "is_relay": true,
      "latency_ms": 120
    }
  ]
}
```

**Esempio d'uso:**
```rust
let cached = bootstrap.load_cached_peers().await?;
println!("Cached peers: {}", cached.len());
```

---

### 5. Tier 3: Peer Gossip

```rust
/// Richiede peer conosciuti da un peer connesso
/// (Gossip-based discovery decentralizzato)
pub async fn gossip_discovery(&self, peer_id: &PeerId) 
    -> Vec<Multiaddr>
```

**Note:** Implementazione avviene nel network layer tramite protocollo custom

---

### 6. Reputation System

```rust
/// Registra connessione riuscita (aumenta reputation)
pub async fn record_successful_dial(
    &mut self, 
    peer_id: &PeerId, 
    addr: &Multiaddr,
    latency_ms: u32
)

/// Registra connessione fallita (diminuisce reputation)
pub async fn record_failed_dial(
    &mut self, 
    peer_id: &PeerId
)
```

**Reputation Score Formula:**
```
score = success_rate + relay_bonus + latency_penalty

Dove:
- success_rate: successful_dials / (successful + failed)
- relay_bonus: +0.1 se is_relay = true
- latency_penalty: -0.1 se latency > 1000ms, -0.05 se > 500ms
```

**Esempio d'uso:**
```rust
// Dopo connessione riuscita
let start = Instant::now();
swarm.dial(addr.clone())?;
// ... connessione stabilita ...
let latency = start.elapsed().as_millis() as u32;

bootstrap.record_successful_dial(&peer_id, &addr, latency).await;

// Dopo fallimento
if connection_failed {
    bootstrap.record_failed_dial(&peer_id).await;
}
```

---

### 7. Relay Support (NAT Traversal)

```rust
/// Marca un peer come relay-capable
pub async fn mark_as_relay(
    &mut self, 
    peer_id: &PeerId, 
    addrs: Vec<Multiaddr>
)

/// Ottieni tutti i relay conosciuti
pub fn get_relay_peers(&self) -> Vec<(PeerId, Vec<Multiaddr>)>

/// Ottieni relay dalla cache (persistenti)
pub fn get_cached_relays(&self) -> Vec<Multiaddr>
```

**Esempio d'uso:**
```rust
// Dopo Identify protocol
SwarmEvent::Behaviour(OtterEvent::Identify(identify::Event::Received {
    peer_id,
    info,
})) => {
    // Verifica se supporta relay
    if info.protocols.iter().any(|p| p.as_ref().contains("/relay")) {
        bootstrap.mark_as_relay(&peer_id, info.listen_addrs).await;
    }
}

// Usa relay per connessione
let relays = bootstrap.get_cached_relays();
for relay_addr in relays {
    swarm.dial(relay_addr)?;
}
```

---

### 8. Statistiche Cache

```rust
/// Ottieni statistiche della cache
pub fn cache_stats(&self) -> CacheStats

pub struct CacheStats {
    pub total_peers: usize,
    pub relay_peers: usize,
    pub average_reputation: f64,
    pub last_updated: i64,
}
```

**Esempio d'uso:**
```rust
let stats = bootstrap.cache_stats();
println!("Cache stats:");
println!("  Total peers: {}", stats.total_peers);
println!("  Relay peers: {}", stats.relay_peers);
println!("  Avg reputation: {:.2}", stats.average_reputation);
```

---

## 🔄 Sequenza di Avvio Completa

```rust
use otter_network::bootstrap::BootstrapSources;
use std::path::PathBuf;

async fn startup_network() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Inizializza bootstrap
    let cache_path = PathBuf::from("~/.otter/peer_cache.json");
    let mut bootstrap = BootstrapSources::new(cache_path);
    bootstrap.initialize().await?;
    
    // 2. Esegui bootstrap completo
    let peers = bootstrap.bootstrap().await;
    
    if peers.is_empty() {
        return Err("No bootstrap peers found!".into());
    }
    
    println!("🚀 Bootstrap discovered {} peers", peers.len());
    
    // 3. Connetti ai peer (in parallelo)
    for addr in peers.iter().take(10) {  // Top 10
        swarm.dial(addr.clone())?;
    }
    
    // 4. Attendi prima connessione
    tokio::time::timeout(
        Duration::from_secs(10),
        wait_for_connection(&mut swarm)
    ).await??;
    
    // 5. Post-connessione: DHT bootstrap
    swarm.behaviour_mut().kad.bootstrap()?;
    
    // 6. Gossip discovery (richiedi peer ai nodi connessi)
    for peer_id in swarm.connected_peers() {
        let gossip_peers = bootstrap.gossip_discovery(&peer_id).await;
        for addr in gossip_peers {
            swarm.dial(addr)?;
        }
    }
    
    // 7. AutoNAT check (nel network layer)
    // 8. Background tasks (periodic refresh)
    
    Ok(())
}
```

---

## 📊 Metriche di Successo

Per validare che il bootstrap funzioni correttamente:

```rust
let stats = bootstrap.cache_stats();

// ✅ Obiettivo 1: Cache popolata entro 24h
assert!(stats.total_peers >= 20, "Insufficient cache size");

// ✅ Obiettivo 2: Reputation media alta
assert!(stats.average_reputation > 0.7, "Low peer quality");

// ✅ Obiettivo 3: Relay disponibili
assert!(stats.relay_peers >= 3, "Need more relay nodes");
```

---

## 🐛 Gestione Errori

```rust
pub enum BootstrapError {
    DnsResolutionFailed(String),
    NoBootstrapPeers,
    InvalidMultiaddr(String),
    ParseError(String),
    IoError(std::io::Error),
    SerializationError(String),
}
```

**Best Practices:**

```rust
match bootstrap.resolve_dns_bootstrap().await {
    Ok(peers) => {
        println!("✅ DNS bootstrap: {} peers", peers.len());
    }
    Err(BootstrapError::DnsResolutionFailed(e)) => {
        warn!("⚠️ DNS failed: {}, falling back to cache", e);
        // Fallback to cached peers
        let cached = bootstrap.load_cached_peers().await?;
        // ...
    }
    Err(e) => {
        error!("❌ Bootstrap critical error: {}", e);
        return Err(e.into());
    }
}
```

---

## 🔧 Configurazione

### Domini DNS Bootstrap

Modifica i domini in `src/bootstrap.rs`:

```rust
pub const DNS_BOOTSTRAP_DOMAINS: &[&str] = &[
    "_dnsaddr.bootstrap.libp2p.io",     // Ufficiale libp2p
    "_dnsaddr.discovery.ipfs.io",       // IPFS
    "_dnsaddr.custom.mydomain.com",     // Custom (opzionale)
];
```

### TTL Cache

```rust
// In PeerCache::cleanup_stale()
cache.cleanup_stale(72);  // 72h TTL

// In DNS cache
Duration::from_secs(86400)  // 24h TTL
```

---

## 🧪 Testing

```bash
# Test completi
cargo test -p otter-network --lib bootstrap

# Test specifici
cargo test -p otter-network test_peer_reputation_score
cargo test -p otter-network test_cache_cleanup_stale
cargo test -p otter-network test_cache_save_load
```

**Test disponibili:**
- ✅ `test_bootstrap_sources_creation`
- ✅ `test_peer_reputation_score`
- ✅ `test_cache_cleanup_stale`
- ✅ `test_cache_save_load`

---

## 📈 Performance

| Operazione | Tempo Atteso | Note |
|-----------|--------------|------|
| `load_cached_peers()` | < 50ms | Lettura file locale |
| `resolve_dns_bootstrap()` | 1-3s | DNS-over-HTTPS query |
| `bootstrap()` completo | 2-5s | Parallel operations |
| `record_successful_dial()` | < 10ms | Async save |

---

## 🔒 Privacy & Sicurezza

### DNS-over-HTTPS

Tutte le query DNS usano HTTPS (Cloudflare DNS):
```
https://cloudflare-dns.com/dns-query?name=...&type=TXT
```

**Vantaggi:**
- ✅ Crittografato end-to-end
- ✅ No ISP snooping
- ✅ Fallback automatico

### Peer Reputation

Il reputation score è **locale** (non condiviso):
- Nessun server centrale
- Privacy-preserving
- Anti-Sybil locale

---

## 🎯 Best Practices

### 1. Inizializza All'Avvio

```rust
// ✅ DO: Initialize early
let mut bootstrap = BootstrapSources::new(cache_path);
bootstrap.initialize().await?;

// ❌ DON'T: Skip initialization
let bootstrap = BootstrapSources::new(cache_path);
// Missing initialize() → cache vuota!
```

### 2. Registra Sempre Dial Results

```rust
// ✅ DO: Track success/failure
match swarm.dial(addr.clone()) {
    Ok(_) => bootstrap.record_successful_dial(&peer_id, &addr, latency).await,
    Err(_) => bootstrap.record_failed_dial(&peer_id).await,
}

// ❌ DON'T: Ignore results
swarm.dial(addr)?;  // No tracking!
```

### 3. Usa Bootstrap Completo

```rust
// ✅ DO: Use all-in-one bootstrap
let peers = bootstrap.bootstrap().await;

// ❌ DON'T: Manual tier management
let dns = bootstrap.resolve_dns_bootstrap().await?;
let cached = bootstrap.load_cached_peers().await?;
// Missing merge logic, TTL handling, etc.
```

---

## 📚 Esempi Completi

### Esempio 1: Bootstrap Minimo

```rust
use otter_network::bootstrap::BootstrapSources;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bootstrap = BootstrapSources::default();
    bootstrap.initialize().await?;
    
    let peers = bootstrap.bootstrap().await;
    
    for addr in peers {
        println!("Discovered: {}", addr);
    }
    
    Ok(())
}
```

### Esempio 2: Con Reputation Tracking

```rust
use otter_network::bootstrap::BootstrapSources;
use std::time::Instant;

async fn connect_with_tracking() {
    let mut bootstrap = BootstrapSources::default();
    bootstrap.initialize().await.unwrap();
    
    let peers = bootstrap.bootstrap().await;
    
    for addr in peers {
        let start = Instant::now();
        
        match swarm.dial(addr.clone()) {
            Ok(_) => {
                let latency = start.elapsed().as_millis() as u32;
                
                // Extract peer_id from addr (simplified)
                if let Some(peer_id) = extract_peer_id(&addr) {
                    bootstrap.record_successful_dial(
                        &peer_id, 
                        &addr, 
                        latency
                    ).await;
                }
            }
            Err(e) => {
                eprintln!("Failed to dial {}: {}", addr, e);
            }
        }
    }
}
```

### Esempio 3: Monitoring Loop

```rust
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(300)).await;  // 5 min
        
        let stats = bootstrap.cache_stats();
        
        println!("📊 Cache Stats:");
        println!("  Peers: {}", stats.total_peers);
        println!("  Relays: {}", stats.relay_peers);
        println!("  Reputation: {:.2}", stats.average_reputation);
        
        // Alert se cache troppo piccola
        if stats.total_peers < 10 {
            warn!("⚠️ Low peer count, triggering bootstrap refresh");
            bootstrap.bootstrap().await;
        }
    }
});
```

---

## 🔗 Risorse

- [Codice sorgente](crates/otter-network/src/bootstrap.rs)
- [Analisi completa DHT](ANALISI_PROGETTO_DHT.md)
- [Architettura implementazione](ARCHITETTURA_IMPLEMENTAZIONE.md)
- [libp2p Bootstrap Spec](https://github.com/libp2p/specs)

---

**Versione:** 1.0  
**Data:** Febbraio 23, 2026  
**Status:** ✅ Implementazione completa con test
