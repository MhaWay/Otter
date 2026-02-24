# 🦦 Otter - Guida Test Android P2P

## ✅ Implementazioni Completate

### FFI Layer (Rust → Dart)
- ✅ `otter_mobile_start_network()` - Inizializzazione completa libp2p
- ✅ `otter_mobile_get_peers()` - Query peer connessi via command channel
- ✅ `otter_mobile_send_message()` - Broadcast messaggi su gossipsub
- ✅ `otter_mobile_stop_network()` - Shutdown network
- ✅ Sistema di eventi asincrono (Rust → FFI callback → Dart)

### Dart Services
- ✅ `NativeBridge` - Binding FFI a tutte le funzioni native
- ✅ `NetworkService` - Gestione stato con Provider/ChangeNotifier
- ✅ Tracking automatico peer (polling 2 secondi)
- ✅ Gestione eventi: `network_ready`, `peer_connected`, `message`, ecc.

### UI Flutter
- ✅ **LoadingScreen**: Workflow inizializzazione rete con log real-time
- ✅ **HomeTab**: Status rete, peer count, invio test message
- ✅ **PeersTab**: Lista live peer connessi con nickname e shortID
- ✅ **ChatTab**: Messaggi ricevuti con timestamp
- ✅ **SettingsTab**: Info rete, versione, disconnect

---

## 🔧 Setup Ambiente Android

### 1. Installa Android NDK

**Opzione A - Via Android Studio:**
```bash
# Apri Android Studio
# Tools → SDK Manager → SDK Tools
# Seleziona: NDK (Side by side)
# Apply per installare
```

**Opzione B - Da linea di comando:**
```bash
# Installa sdkmanager se non presente
sudo apt install android-sdk

# Installa NDK (ultima versione LTS)
sdkmanager --install "ndk;26.1.10909125"

# Imposta variabile ambiente
export ANDROID_NDK_HOME=$HOME/Android/Sdk/ndk/26.1.10909125
echo 'export ANDROID_NDK_HOME=$HOME/Android/Sdk/ndk/26.1.10909125' >> ~/.bashrc
```

**Opzione C - Download manuale:**
```bash
# Scarica da: https://developer.android.com/ndk/downloads
# Estrai in ~/Android/Sdk/ndk/ o altra directory
export ANDROID_NDK_HOME=/path/to/ndk
```

### 2. Installa Rust Target Android

```bash
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
rustup target add i686-linux-android
```

### 3. Verifica Flutter

```bash
cd flutter_app
flutter doctor

# Se manca Android SDK:
flutter config --android-sdk /path/to/Android/Sdk

# Accetta licenze
flutter doctor --android-licenses
```

---

## 🚀 Build e Deploy

### Build Rust FFI per Android

```bash
# Dalla root di Otter:
./quick_build_android.sh

# Output atteso:
# ✅ libotter_mobile.so → flutter_app/android/app/src/main/jniLibs/arm64-v8a/
```

### Opzioni Build Avanzate

**Build multi-arch (arm64 + arm7):**
```bash
cargo ndk \
  --android-platform 21 \
  --output flutter_app/android/app/src/main/jniLibs \
  build -p otter-mobile --lib \
  --target aarch64-linux-android \
  --target armv7-linux-androideabi \
  --release
```

**Build per emulatore x86:**
```bash
cargo ndk \
  --android-platform 21 \
  --output flutter_app/android/app/src/main/jniLibs \
  build -p otter-mobile --lib \
  --target x86_64-linux-android \
  --release
```

### Deploy su Dispositivo

```bash
# 1. Abilita USB debugging sul dispositivo Android
#    Settings → About phone → Tap "Build number" 7 volte
#    Settings → Developer options → USB debugging ON

# 2. Connetti dispositivo via USB

# 3. Verifica connessione
adb devices
# Dovrebbe mostrare: <device_id>   device

cd flutter_app

# 4. Installa dipendenze
flutter pub get

# 5. Run su dispositivo
flutter run --release

# Oppure build APK:
flutter build apk --release
# APK: flutter_app/build/app/outputs/flutter-apk/app-release.apk
```

---

## 🧪 Scenari di Test P2P

### Test 1: Avvio Isolato (Single Device)

**Setup:** 1 dispositivo, nessuna rete WiFi

**Procedura:**
1. Installa app: `flutter run`
2. App avvia → LoadingScreen con log
3. Aspettare 14 secondi (timeout)
4. App passa a MainApp

**Verifica:**
- ✅ Log mostra: "Inizializzazione Otter...", "Avvio rete P2P..."
- ✅ MainApp mostra: "0 peers", Network status: "In attesa ⏳"
- ✅ Nessun crash o errore

**Log Rust da controllare:**
```bash
adb logcat | grep otter
# Aspettati: Network started, DHT bootstrap in progress, timeout
```

---

### Test 2: Peer Discovery mDNS (Local Network)

**Setup:** 2+ dispositivi Android sulla **stessa rete WiFi locale**

**Procedura:**
1. Installa app su Device A e B
2. Avvia Device A → Attendi LoadingScreen
3. Avvia Device B → Attendi LoadingScreen
4. Dopo 2-5 secondi, entrambi dovrebbero mostrare MainApp

**Verifica:**
- ✅ HomeTab mostra: "1+ peers" su entrambi i device
- ✅ PeersTab mostra: Peer dell'altro dispositivo con ID e nickname
- ✅ Network status: "Connesso ✅"
- ✅ Log: "🎉 Rete pronta! Peers: 1"

**Dettagli tecnici:**
- mDNS trasmette su multicast 224.0.0.251:5353
- Discovery time: 1-10 secondi tipicamente
- Se firewall/router blocca multicast → fallback a DHT bootstrap

**Troubleshooting:**
```bash
# Su entrambi i device:
adb logcat | grep -E "(mDNS|Discovered peer|PeerDiscovered)"

# Aspettati su Device A:
# mDNS: Discovered peer <peer_id_B> at /ip4/192.168.x.x/tcp/...
# PeerDiscovered { peer_id: <peer_id_B>, addresses: [...] }
```

---

### Test 3: DHT Bootstrap (Internet)

**Setup:** 2+ dispositivi su **reti diverse** (4G + WiFi, oppure WiFi casa 1 + WiFi casa 2)

**Procedura:**
1. Device A su rete WiFi casa
2. Device B su rete mobile 4G/5G
3. Avvia entrambi contemporaneamente
4. Attendi 10-30 secondi

**Verifica:**
- ✅ Device A e B si trovano via DHT bootstrap peers
- ✅ Connessione stabilita (potrebbe richiedere relay se dietro NAT simmetrico)
- ✅ Peers mostrati in PeersTab
- ✅ Log: "Bootstrap complete", "Peer connected via DHT"

**Bootstrap Peers (hardcoded in otter-network):**
- Usano DNS discovery o hardcoded multiaddr
- Fallback a DHT query iterativo

**Troubleshooting:**
```bash
adb logcat | grep -E "(Bootstrap|DHT|Kademlia|AutoNAT)"

# Se dopo 60 secondi nessun peer:
# - Verifica connessione Internet
# - Bootstrap peers potrebbero essere offline (usa bootstrap locale)
# - NAT simmetrico richiede relay (non ancora implementato)
```

---

### Test 4: Invio e Ricezione Messaggi

**Setup:** 2 dispositivi connessi (da Test 2 o Test 3)

**Procedura:**
1. Device A: HomeTab → Tap "Invia Test Message"
2. Device B: Naviga a **ChatTab**

**Verifica:**
- ✅ Device A: Log conferma "Message sent"
- ✅ Device B: ChatTab mostra nuovo messaggio entro 1-3 secondi
- ✅ Messaggio contiene: sender ID, timestamp, data

**Struttura messaggio:**
```dart
// NetworkMessage in network_service.dart
{
  "from": "12D3KooW...",  // PeerId mittente
  "topic": "otter-global",
  "data": "Test message from Otter Mobile!",
  "timestamp": "2026-02-24T15:30:00.000Z"
}
```

**Troubleshooting:**
```bash
# Device A (sender):
adb logcat | grep "SendMessage\|gossipsub publish"

# Device B (receiver):
adb logcat | grep "MessageReceived\|message event"

# Se messaggio non arriva:
# 1. Verifica subscription a topic "otter-chat" in Network::new()
# 2. Gossipsub heartbeat ogni 10 secondi - attendi 15s
# 3. Controlla peer_count > 0 su entrambi i device
```

---

### Test 5: Stress Test Multi-Peer (3+ devices)

**Setup:** 3-5 dispositivi sulla stessa WiFi

**Procedura:**
1. Avvia tutti i device simultaneamente
2. Attendi formazione mesh (10-20 secondi)
3. Da diversi device: invia messaggi
4. Monitora stabilità connessioni per 5+ minuti

**Verifica:**
- ✅ Tutti i device vedono N-1 peer (es. 3 device → 2 peer ciascuno)
- ✅ Messaggi propagano a TUTTI i device entro 3 secondi
- ✅ Nessun disconnect spontaneo (idle_timeout 10 minuti)
- ✅ peer_count rimane stabile

**Metriche da monitorare:**
```bash
# Ogni 30 secondi su tutti i device:
adb shell "dumpsys meminfo com.example.otter_mobile | grep TOTAL"
adb shell "top -n 1 | grep otter"

# Aspettati:
# - RAM: 80-150 MB stabile
# - CPU: <5% quando idle, <15% durante discovery
```

---

## 🐛 Debug e Logging

### Livelli Log Rust

**File:** `crates/otter-network/src/lib.rs`

```rust
// Per verbose logging durante test:
tracing_subscriber::fmt()
    .with_env_filter("otter=debug,libp2p=debug")
    .init();

// Production:
tracing_subscriber::fmt()
    .with_env_filter("otter=info,libp2p=warn")
    .init();
```

### Log Flutter/Dart

```bash
# Full logcat:
adb logcat

# Solo app:
adb logcat | grep -E "flutter|otter"

# Solo network events:
adb logcat | grep "NetworkService"

# Salva log su file:
adb logcat > test_android_$(date +%Y%m%d_%H%M%S).log
```

### Debug UI in tempo reale

**Aggiungi in HomeTab:**
```dart
// flutter_app/lib/screens/main_app_screen.dart
Text('Last event: ${networkService.lastEventType}'),
Text('Event count: ${networkService.eventCount}'),
```

---

## 📊 Metriche Attese

### Discovery Time
- **mDNS (same WiFi):** 1-10 secondi
- **DHT bootstrap:** 10-60 secondi
- **Relay fallback:** 30-120 secondi (se NAT simmetrico)

### Message Latency
- **Same WiFi:** 50-500 ms
- **Cross-internet:** 100-2000 ms (dipende da ping)
- **Via relay:** +500-1500 ms overhead

### Resource Usage (per device)
- **RAM:** 80-150 MB (stabile dopo 5 min)
- **CPU:** < 5% idle, < 20% durante discovery
- **Network:** 1-5 KB/s idle (heartbeat), 10-50 KB/s durante chat
- **Battery:** 2-5% per ora in background (stima)

---

## 🔍 Checklist Pre-Test

Prima di ogni sessione di test:

- [ ] Android NDK installato: `echo $ANDROID_NDK_HOME`
- [ ] Rust target installato: `rustup target list | grep aarch64-linux-android`
- [ ] cargo-ndk installato: `cargo ndk --version`
- [ ] Flutter valido: `flutter doctor` (0 errori gravi)
- [ ] Device connesso: `adb devices`
- [ ] USB debugging abilitato
- [ ] WiFi attivo (per mDNS test)
- [ ] Build recente: `./quick_build_android.sh` eseguito < 24h

---

## 🎯 Prossimi Passi Dopo Test

### Priorità ALTA (dopo test riusciti):
1. **Persistenza Identity**: Salvare/caricare keypair da storage (evita nuovo peer_id ad ogni avvio)
2. **UI Miglioramenti**: Avatar, nickname utente, chat input field
3. **Notifiche Push**: Per messaggi in background
4. **Reconnection Logic**: Auto-reconnect dopo perdita rete

### Priorità MEDIA:
5. **E2E Encryption**: Crittografia messaggi (già prevista in otter-crypto)
6. **Peer Trust System**: Rating/blocking peer malevoli
7. **File Sharing**: Condivisione file via chunks su gossipsub
8. **Voice Chat**: Integrazione otter-voice con WebRTC

### Priorità BASSA:
9. **iOS Support**: Porting FFI per iOS (90% riutilizzabile)
10. **Background Service**: Rete attiva con app in background Android
11. **Metrics Dashboard**: Telemetria P2P in Settings

---

## 📝 Report Bug/Issue

Quando trovi un problema durante i test, documenta:

```markdown
### Bug: [Titolo breve]

**Device:** Samsung Galaxy S21 / Android 13
**Build:** `git rev-parse --short HEAD` output
**Network:** WiFi / 4G / Airplane mode
**Peers:** 0 / 1 / 2+

**Passi:**
1. ...
2. ...

**Atteso:**
XXX dovrebbe succedere

**Reale:**
YYY è successo invece

**Log:**
```
<paste logcat output>
```

**Screenshot:** (allega se UI)
```

Invia issue a: https://github.com/MhaWay/Otter/issues

---

## ✅ Test Completati Template

```markdown
# Test Session - 2026-02-24

## Test 1: Avvio Isolato
- ✅ Passa | ❌ Fallisce | ⚠️ Parziale
- Note: ...

## Test 2: mDNS Discovery
- ✅ / ❌ / ⚠️
- Time to connect: XX secondi
- Note: ...

## Test 3: DHT Bootstrap
- ✅ / ❌ / ⚠️
- Note: ...

## Test 4: Messaging
- ✅ / ❌ / ⚠️
- Latency: XX ms
- Note: ...

## Test 5: Multi-Peer
- ✅ / ❌ / ⚠️
- Devices: X
- Stability: X min senza disconnessioni
- Note: ...
```

---

**Buon testing! 🦦🚀**
