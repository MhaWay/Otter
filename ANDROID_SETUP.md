# 🦦 Otter Mobile - Setup Completo Android

## Requisiti Sistema

- **OS:** Linux (Ubuntu/Debian), macOS, Windows + WSL2
- **RAM:** Minimo 8 GB (consigliato 16 GB)
- **Spazio:** ~15 GB per Android SDK + NDK + Flutter

---

## 1. Installazione Flutter

### Linux (Ubuntu/Debian)

```bash
# Opzione A: Snap (più semplice)
sudo snap install flutter --classic

# Verifica installazione
flutter --version

# Opzione B: Manuale
cd ~
wget https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_3.24.0-stable.tar.xz
tar xf flutter_linux_3.24.0-stable.tar.xz
export PATH="$PATH:`pwd`/flutter/bin"
echo 'export PATH="$PATH:$HOME/flutter/bin"' >> ~/.bashrc

# Pre-download dependencies
flutter precache
```

### macOS

```bash
# Via Homebrew
brew install --cask flutter

# Oppure download manuale
cd ~
curl -O https://storage.googleapis.com/flutter_infra_release/releases/stable/macos/flutter_macos_3.24.0-stable.zip
unzip flutter_macos_3.24.0-stable.zip
export PATH="$PATH:`pwd`/flutter/bin"
echo 'export PATH="$PATH:$HOME/flutter/bin"' >> ~/.zshrc
```

### Verifica

```bash
flutter doctor

# Output atteso (alcuni ✗ sono OK se fai solo Android):
# [✓] Flutter (Channel stable, 3.24.0, on Linux, locale en_US.UTF-8)
# [✓] Android toolchain - develop for Android devices (Android SDK version 34.0.0)
# [✗] Chrome - develop for the web (Not installed)  ← OK da ignorare
# [✗] Android Studio (not installed)  ← Installeremo dopo
# [✓] Connected device (1 available)
# [✓] Network resources
```

---

## 2. Installazione Android SDK + NDK

### Opzione A: Android Studio (Consigliato per UI)

```bash
# Linux
sudo snap install android-studio --classic

# Oppure download manuale:
# https://developer.android.com/studio

# macOS
brew install --cask android-studio

# Avvia Android Studio
android-studio

# Nel wizard iniziale:
# 1. Next → Next → Standard installation
# 2. Accetta licenze
# 3. Finish (scarica SDK automaticamente)

# Installa NDK:
# Android Studio → More Actions → SDK Manager
# SDK Tools tab:
#   ✓ Android SDK Build-Tools
#   ✓ Android SDK Command-line Tools
#   ✓ NDK (Side by side)  ← IMPORTANTE
#   ✓ Android Emulator (opzionale)
# Apply → OK
```

### Opzione B: Solo Command Line (più leggero)

```bash
# Linux
cd ~
mkdir -p Android/Sdk
cd Android/Sdk

# Download command-line tools
wget https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip
unzip commandlinetools-linux-11076708_latest.zip
mkdir -p cmdline-tools/latest
mv cmdline-tools/* cmdline-tools/latest/ 2>/dev/null || true

# Aggiungi al PATH
export ANDROID_HOME=$HOME/Android/Sdk
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin
export PATH=$PATH:$ANDROID_HOME/platform-tools
echo 'export ANDROID_HOME=$HOME/Android/Sdk' >> ~/.bashrc
echo 'export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin' >> ~/.bashrc
echo 'export PATH=$PATH:$ANDROID_HOME/platform-tools' >> ~/.bashrc

# Installa SDK e NDK
sdkmanager "platform-tools" "platforms;android-34" "build-tools;34.0.0"
sdkmanager "ndk;26.1.10909125"  # NDK LTS

# Accetta licenze
sdkmanager --licenses

# Imposta NDK_HOME
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/26.1.10909125
echo 'export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/26.1.10909125' >> ~/.bashrc

# macOS: sostituisci 'linux' con 'mac' nei link di download
```

### Verifica

```bash
# Verifica variabili ambiente
echo $ANDROID_HOME        # ~/Android/Sdk
echo $ANDROID_NDK_HOME    # ~/Android/Sdk/ndk/26.1.10909125

# Verifica tools
adb --version             # Android Debug Bridge
sdkmanager --list | grep -E "ndk|build-tools"

# Verifica Flutter vede Android
flutter doctor
# [✓] Android toolchain ← Dovrebbe essere verde ora
```

---

## 3. Configurazione Flutter per Android

```bash
# Configura Android SDK path (se non auto-detected)
flutter config --android-sdk $ANDROID_HOME

# Accetta licenze Android
flutter doctor --android-licenses
# Premere 'y' su tutte le licenze

# Verifica finale
flutter doctor -v

# Output atteso:
# [✓] Flutter (Channel stable, 3.24.0)
# [✓] Android toolchain - develop for Android devices (Android SDK version 34.0.0)
#     • Android SDK at /home/user/Android/Sdk
#     • Platform android-34, build-tools 34.0.0
#     • Java binary at: /usr/bin/java
#     • Java version OpenJDK Runtime Environment
#     • All Android licenses accepted.
# [✓] Connected device (1 available)
```

---

## 4. Installazione Rust + Cargo Tools

### Rust (se non già installato)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verifica
rustc --version
cargo --version
```

### Rust Targets Android

```bash
# Installa target per ARM64 (più comuni Android moderni)
rustup target add aarch64-linux-android

# (Opzionale) Altri target per compatibilità
rustup target add armv7-linux-androideabi   # ARM 32-bit
rustup target add x86_64-linux-android      # Emulatore x86
rustup target add i686-linux-android        # Emulatore x86 32-bit

# Verifica
rustup target list | grep android
# aarch64-linux-android (installed)
# armv7-linux-androideabi (installed)
# ...
```

### cargo-ndk

```bash
cargo install cargo-ndk

# Verifica
cargo ndk --version
```

---

## 5. Setup Progetto Otter

```bash
cd /path/to/Otter

# Install Flutter dependencies
cd flutter_app
flutter pub get

# Build Rust FFI per Android
cd ..
./quick_build_android.sh

# Se tutto OK, dovresti vedere:
# ✅ libotter_mobile.so → flutter_app/android/app/src/main/jniLibs/arm64-v8a/
```

---

## 6. Connessione Dispositivo Android

### Via USB

```bash
# Sul dispositivo Android:
# Settings → About phone → Tap "Build number" 7 volte
# → "You are now a developer!"
# Settings → Developer options → USB debugging ON

# Connetti dispositivo via USB al computer

# Verifica connessione
adb devices
# List of devices attached
# ABC123456789    device  ← OK!

# Se mostra "unauthorized":
# - Guarda dispositivo → popup "Allow USB debugging?" → Allow
# - Riprova: adb devices
```

### Via WiFi (Android 11+)

```bash
# Una volta abilitato USB debugging via cavo:

# Sul computer (con dispositivo connesso USB):
adb tcpip 5555
adb shell ip addr show wlan0 | grep "inet "
# Nota l'IP, es: 192.168.1.100

# Scollega USB

# Connetti via WiFi:
adb connect 192.168.1.100:5555

# Verifica
adb devices
# 192.168.1.100:5555    device
```

---

## 7. Build e Run

### Development Build (con hot reload)

```bash
cd /path/to/Otter/flutter_app

# Run su dispositivo connesso
flutter run

# Se più dispositivi connessi:
flutter devices
flutter run -d ABC123456789  # usa device ID

# Hot reload: premere 'r' nel terminal
# Hot restart: premere 'R'
# Quit: premere 'q'
```

### Release Build (ottimizzato)

```bash
# Build APK
flutter build apk --release

# Output:
# ✓ Built build/app/outputs/flutter-apk/app-release.apk (XX.X MB)

# Installa APK su dispositivo
adb install build/app/outputs/flutter-apk/app-release.apk

# Oppure build App Bundle (per Google Play)
flutter build appbundle --release
# Output: build/app/outputs/bundle/release/app-release.aab
```

### Debug Build (con console log)

```bash
flutter run --debug

# In parallelo, monitora log nativi:
adb logcat | grep -E "flutter|otter|NetworkService"
```

---

## 8. Troubleshooting Comuni

### ❌ "Could not find any NDK"

```bash
# Verifica NDK installato:
ls $ANDROID_HOME/ndk
# Dovrebbe listare: 26.1.10909125 (o altra versione)

# Imposta ANDROID_NDK_HOME:
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/26.1.10909125
echo 'export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/26.1.10909125' >> ~/.bashrc
source ~/.bashrc

# Riprova build
./quick_build_android.sh
```

### ❌ "Gradle build failed" / "Could not resolve dependencies"

```bash
cd flutter_app/android

# Clear cache
./gradlew clean
rm -rf ~/.gradle/caches

# Riprova
cd ..
flutter clean
flutter pub get
flutter run
```

### ❌ "adb: device unauthorized"

```bash
# Sul dispositivo: Allow USB debugging popup

# Se non appare popup:
adb kill-server
adb start-server
adb devices

# Revoca autorizzazioni precedenti:
# Dispositivo → Developer options → Revoke USB debugging authorizations
# Scollega e ricollega USB
```

### ❌ "libotter_mobile.so not found"

```bash
# Verifica file esiste
ls -lh flutter_app/android/app/src/main/jniLibs/arm64-v8a/libotter_mobile.so

# Se manca:
./quick_build_android.sh

# Se quick_build_android.sh fallisce:
cargo ndk \
  --android-platform 21 \
  --output flutter_app/android/app/src/main/jniLibs \
  build -p otter-mobile --lib \
  --target aarch64-linux-android \
  --release
```

### ❌ Flutter run timeout / "Waiting for connection"

```bash
# Controlla dispositivo connesso
adb devices

# Uccidi server ADB e riparte
adb kill-server
adb start-server

# Su alcuni Linux serve udev rules:
sudo usermod -aG plugdev $USER
wget -S -O - https://raw.githubusercontent.com/cm-b2g/B2G/1230463/tools/51-android.rules | sudo tee /etc/udev/rules.d/51-android.rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# Log out e log in (o riavvia)
```

---

## 9. Verifica Setup Completo

Esegui questo script per verificare tutto:

```bash
#!/bin/bash
echo "🦦 Otter Android Setup Checker"
echo "================================"

# Flutter
echo -n "Flutter: "
if command -v flutter &> /dev/null; then
    flutter --version | head -n1
else
    echo "❌ NOT FOUND"
fi

# Rust
echo -n "Rust: "
if command -v rustc &> /dev/null; then
    rustc --version
else
    echo "❌ NOT FOUND"
fi

# cargo-ndk
echo -n "cargo-ndk: "
if command -v cargo-ndk &> /dev/null; then
    cargo ndk --version 2>&1 | head -n1
else
    echo "❌ NOT FOUND"
fi

# Android SDK
echo -n "Android SDK: "
if [ -n "$ANDROID_HOME" ]; then
    echo "✓ $ANDROID_HOME"
else
    echo "❌ ANDROID_HOME not set"
fi

# Android NDK
echo -n "Android NDK: "
if [ -n "$ANDROID_NDK_HOME" ]; then
    echo "✓ $ANDROID_NDK_HOME"
else
    echo "❌ ANDROID_NDK_HOME not set"
fi

# ADB
echo -n "ADB: "
if command -v adb &> /dev/null; then
    adb --version | head -n1
else
    echo "❌ NOT FOUND"
fi

# Rust targets
echo "Rust Android targets:"
rustup target list | grep android | grep installed || echo "  ❌ None installed"

# Devices
echo "Connected devices:"
if command -v adb &> /dev/null; then
    adb devices | tail -n +2 | grep -v "^$" || echo "  ⚠️ No devices"
else
    echo "  ❌ adb not found"
fi

echo ""
echo "Run: flutter doctor  for detailed Flutter status"
```

Salva come `check_setup.sh`, rendi eseguibile: `chmod +x check_setup.sh`, ed esegui: `./check_setup.sh`

**Output atteso OK:**
```
🦦 Otter Android Setup Checker
================================
Flutter: Flutter 3.24.0 • channel stable
Rust: rustc 1.81.0
cargo-ndk: cargo-ndk 4.1.2
Android SDK: ✓ /home/user/Android/Sdk
Android NDK: ✓ /home/user/Android/Sdk/ndk/26.1.10909125
ADB: Android Debug Bridge version 1.0.41
Rust Android targets:
  aarch64-linux-android (installed)
  armv7-linux-androideabi (installed)
Connected devices:
  ABC123456789    device
```

---

## 10. Prossimi Passi

Una volta completato il setup:

1. **Build FFI:** `./quick_build_android.sh`
2. **Test app:** `cd flutter_app && flutter run`
3. **Segui ANDROID_TESTING.md** per scenari test P2P

---

## Riferimenti

- **Flutter Install:** https://docs.flutter.dev/get-started/install
- **Android Studio:** https://developer.android.com/studio
- **Android NDK:** https://developer.android.com/ndk/downloads
- **cargo-ndk:** https://github.com/bbqsrc/cargo-ndk
- **ADB troubleshooting:** https://developer.android.com/studio/command-line/adb

---

**Setup completato? 🚀 Inizia con i test!**
