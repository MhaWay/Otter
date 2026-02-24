# 🦦 Otter - Deploy Android - Quick Reference

## ✅ Ambiente Già Configurato

L'ambiente Android è stato completamente configurato con:

- ✅ **Flutter** installato via snap
- ✅ **Android SDK** in `$HOME/Android/Sdk`
- ✅ **Android NDK 26.1.10909125** installato
- ✅ **platform-tools** (adb) installato
- ✅ **Rust targets** Android installati (aarch64, armv7)
- ✅ **cargo-ndk** installato
- ✅ **Variabili ambiente** configurate in `.bashrc`

---

## 🚀 Deploy Rapido (TL;DR)

### Metodo 1: Deploy Completo Automatico

```bash
# Build Rust FFI + Flutter APK + Installa su tutti i dispositivi
./deploy_android.sh
```

Lo script:
1. Rileva dispositivi connessi
2. Chiede tipo build (debug/release/profile)
3. Builda Rust FFI per Android
4. Builda Flutter APK
5. Installa su tutti i dispositivi
6. Opzionalmente avvia l'app e mostra logcat

### Metodo 2: Build + Installa Separati

```bash
# 1. Build solo Rust FFI
./quick_build_android.sh

# 2. Build + installa Flutter
cd flutter_app
flutter run --release

# Oppure build APK e installa manualmente
flutter build apk --release
./install_apk.sh
```

---

## 📋 Script Disponibili

### `./check_android_setup.sh`
Verifica che tutto l'ambiente sia configurato correttamente.

Output:
- ✅ Flutter version
- ✅ Rust version  
- ✅ Android SDK/NDK paths
- ✅ ADB disponibile
- ✅ Rust Android targets installati
- ✅ Dispositivi connessi

### `./list_android_devices.sh`
Mostra tutti i dispositivi Android connessi con dettagli:
- Device ID
- Manufacturer
- Model
- Android version
- CPU ABI
- Connection type (USB/WiFi)

### `./deploy_android.sh`
Deploy automatico completo (build + install + launch).

Fasi:
1. Rileva dispositivi Android connessi
2. Chiede tipo di build (debug/release/profile)
3. Build Rust FFI library (`libotter_mobile.so`)
4. Build Flutter APK
5. Installa APK su tutti i dispositivi
6. Opzionalmente avvia app
7. Opzionalmente mostra logcat filtered

### `./install_apk.sh`
Installa APK già buildato su tutti i dispositivi connessi.

Utile per installare velocemente dopo un build Flutter manuale.

### `./quick_build_android.sh`
Build veloce della sola libreria Rust FFI per Android.

Output: `flutter_app/android/app/src/main/jniLibs/arm64-v8a/libotter_mobile.so`

---

## 🔧 Workflow Consigliato

### Prima Volta

```bash
# 1. Verifica setup
./check_android_setup.sh

# 2. Connetti dispositivo Android via USB
#    - Abilita "Opzioni sviluppatore"
#    - Abilita "Debug USB"
#    - Connetti cavo USB

# 3. Verifica connessione
./list_android_devices.sh

# 4. Deploy completo
./deploy_android.sh
# Scegli: 1) Debug (più veloce)
# Scegli: 1) Sì, avvia su tutti i dispositivi
# Scegli: 1) Sì, tail logcat
```

### Testing Iterativo

Durante lo sviluppo con modifiche frequenti:

**Modifiche solo Dart/Flutter:**
```bash
cd flutter_app
flutter run --hot
# Poi usa 'r' per hot reload, 'R' per hot restart
```

**Modifiche Rust FFI:**
```bash
./quick_build_android.sh
cd flutter_app
flutter run
```

**Build Release per testing finale:**
```bash
./deploy_android.sh
# Scegli: 2) Release
```

---

## 📱 Testing P2P Multi-Device

### Setup: 2 Dispositivi

```bash
# 1. Connetti Device A via USB
# 2. Install:
./deploy_android.sh

# 3. Verifica Device A installato
./list_android_devices.sh

# 4. Scollega Device A, connetti Device B
# 5. Install su Device B:
./install_apk.sh

# 6. Entrambi i device sulla stessa WiFi
# 7. Avvia app su entrambi
# 8. Attendi 10-20 secondi → dovrebbero scoprirsi via mDNS/DHT
```

### Monitoring

**Logcat live da un device:**
```bash
# Trova device ID
./list_android_devices.sh

# Tail logcat filtrato
adb -s <device_id> logcat | grep -E "flutter|otter|NetworkService|PeerDiscovered"
```

**Eventi attesi:**
```
flutter: NetworkService - network_started
flutter: Peer ID: 12D3KooW...
flutter: NetworkService - network_ready, peer_count: 0
flutter: NetworkService - peer_connected: { peer_id: 12D3KooW... }
```

---

## 🐛 Troubleshooting

### ❌ "No devices connected"

```bash
# Verifica connessione
adb devices

# Se lista vuota:
# 1. Controlla cavo USB
# 2. Su dispositivo: controlla popup "Allow USB debugging"
# 3. Riprova: adb kill-server && adb start-server
```

### ❌ "device unauthorized"

```bash
# Sul dispositivo:
# Settings → Developer options → Revoke USB debugging authorizations
# Scollega/ricollega USB
# Accetta popup "Allow USB debugging"
```

### ❌ "Could not find Flutter"

```bash
# Flutter installato via snap, potrebbe servire reload PATH
source ~/.bashrc

# Verifica
flutter --version
```

### ❌ "ANDROID_NDK_HOME not set"

```bash
# Reload bashrc
source ~/.bashrc

# Verifica
echo $ANDROID_NDK_HOME
# Dovrebbe mostrare: /home/mhaway/Android/Sdk/ndk/26.1.10909125

# Se vuoto, export manualmente:
export ANDROID_HOME=$HOME/Android/Sdk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/26.1.10909125
export PATH=$PATH:$ANDROID_HOME/platform-tools
```

### ❌ "libotter_mobile.so not found"

```bash
# Build non riuscito o file in posizione sbagliata
# Rebuild:
./quick_build_android.sh

# Verifica output:
ls -lh flutter_app/android/app/src/main/jniLibs/arm64-v8a/libotter_mobile.so
```

### ❌ "App crashes on launch"

```bash
# Monitor logcat per stack trace
adb logcat | grep -E "FATAL|AndroidRuntime"

# Cause comuni:
# - libotter_mobile.so non presente → rebuild FFI
# - Permessi mancanti in AndroidManifest.xml
# - Versione Android troppo vecchia (richiede min SDK 21)
```

---

## 📊 Dopo il Deploy

### Test Checklist

- [ ] App si avvia senza crash
- [ ] LoadingScreen mostra log inizializzazione
- [ ] Dopo 10-15 sec passa a MainApp
- [ ] HomeTab mostra "Network status"
- [ ] PeersTab mostra lista peer (vuota se isolato)
- [ ] ChatTab funzionante
- [ ] SettingsTab mostra versione e peer ID

### Multi-Device Test

- [ ] 2 device sulla stessa WiFi
- [ ] Entrambi mostrano "1 peer" in HomeTab dopo 10-20 sec
- [ ] PeersTab mostra peer dell'altro device
- [ ] Device A: tap "Invia Test Message"
- [ ] Device B: ChatTab riceve messaggio entro 3 secondi

---

## 🔗 Collegamenti Utili

- **Setup Dettagliato**: [ANDROID_SETUP.md](ANDROID_SETUP.md)
- **Scenari Test**: [ANDROID_TESTING.md](ANDROID_TESTING.md)
- **Architettura FFI**: [crates/otter-mobile/src/lib.rs](crates/otter-mobile/src/lib.rs)
- **NetworkService Dart**: [flutter_app/lib/services/network_service.dart](flutter_app/lib/services/network_service.dart)

---

**Ready to test! 🦦🚀**
