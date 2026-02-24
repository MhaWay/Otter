#!/bin/bash

# 🦦 Otter - Deploy Automatico su Android
# 
# Questo script:
# 1. Rileva dispositivi Android connessi
# 2. Builda la libreria Rust FFI
# 3. Builda l'APK Flutter
# 4. Installa su tutti i dispositivi connessi

set -e  # Exit on error

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Banner
echo -e "${CYAN}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "   🦦 Otter - Deploy Automatico Android"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${NC}"

# Check environment
if [ -z "$ANDROID_HOME" ]; then
    echo -e "${RED}❌ ANDROID_HOME non impostato${NC}"
    echo "Esegui: export ANDROID_HOME=\$HOME/Android/Sdk"
    exit 1
fi

if [ -z "$ANDROID_NDK_HOME" ]; then
    echo -e "${RED}❌ ANDROID_NDK_HOME non impostato${NC}"
    echo "Esegui: export ANDROID_NDK_HOME=\$ANDROID_HOME/ndk/26.1.10909125"
    exit 1
fi

# Check for adb
if ! command -v adb &> /dev/null; then
    echo -e "${RED}❌ adb non trovato${NC}"
    echo "Assicurati che \$ANDROID_HOME/platform-tools sia nel PATH"
    exit 1
fi

# Detect connected devices
echo -e "${BLUE}📱 Rilevamento dispositivi Android...${NC}"
DEVICES=$(adb devices | tail -n +2 | grep -v "^$" | grep "device$" | cut -f1)
DEVICE_COUNT=$(echo "$DEVICES" | grep -c . || echo 0)

if [ "$DEVICE_COUNT" -eq 0 ]; then
    echo -e "${YELLOW}⚠️  Nessun dispositivo Android connesso${NC}"
    echo ""
    echo "Connetti un dispositivo via USB e:"
    echo "  1. Abilita 'Opzioni sviluppatore'"
    echo "  2. Abilita 'Debug USB'"
    echo "  3. Riprova: adb devices"
    echo ""
    exit 1
fi

echo -e "${GREEN}✓ Trovati $DEVICE_COUNT dispositivo/i:${NC}"
echo "$DEVICES" | while read -r device; do
    MODEL=$(adb -s "$device" shell getprop ro.product.model 2>/dev/null | tr -d '\r')
    ANDROID_VERSION=$(adb -s "$device" shell getprop ro.build.version.release 2>/dev/null | tr -d '\r')
    echo "  • $device - $MODEL (Android $ANDROID_VERSION)"
done
echo ""

# Ask for build type
echo -e "${BLUE}🔧 Tipo di build:${NC}"
echo "  1) Debug (più veloce, con log)"
echo "  2) Release (ottimizzato, senza log)"
echo "  3) Profile (performance profiling)"
read -p "Scelta [1-3, default=1]: " BUILD_TYPE
BUILD_TYPE=${BUILD_TYPE:-1}

case $BUILD_TYPE in
    1)
        BUILD_MODE="debug"
        FLUTTER_BUILD_CMD="flutter build apk --debug"
        ;;
    2)
        BUILD_MODE="release"
        FLUTTER_BUILD_CMD="flutter build apk --release"
        ;;
    3)
        BUILD_MODE="profile"
        FLUTTER_BUILD_CMD="flutter build apk --profile"
        ;;
    *)
        echo -e "${RED}❌ Scelta non valida${NC}"
        exit 1
        ;;
esac

echo -e "${GREEN}✓ Build mode: $BUILD_MODE${NC}"
echo ""

# Step 1: Build Rust FFI
echo -e "${BLUE}🔨 Step 1/3: Building Rust FFI library...${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ "$BUILD_MODE" == "release" ] || [ "$BUILD_MODE" == "profile" ]; then
    CARGO_PROFILE="--release"
else
    CARGO_PROFILE=""
fi

cd "$(dirname "$0")"

cargo ndk \
    -t arm64-v8a \
    -o flutter_app/android/app/src/main/jniLibs \
    build -p otter-mobile --lib \
    $CARGO_PROFILE

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Rust FFI build completata${NC}"
    ls -lh flutter_app/android/app/src/main/jniLibs/arm64-v8a/libotter_mobile.so
else
    echo -e "${RED}❌ Rust FFI build fallita${NC}"
    exit 1
fi
echo ""

# Step 2: Build Flutter APK
echo -e "${BLUE}🔨 Step 2/3: Building Flutter APK...${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

cd flutter_app

# Get dependencies
flutter pub get

# Build APK
$FLUTTER_BUILD_CMD

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Flutter APK build completata${NC}"
    if [ "$BUILD_MODE" == "debug" ]; then
        APK_PATH="build/app/outputs/flutter-apk/app-debug.apk"
    else
        APK_PATH="build/app/outputs/flutter-apk/app-$BUILD_MODE.apk"
    fi
    
    if [ -f "$APK_PATH" ]; then
        APK_SIZE=$(du -h "$APK_PATH" | cut -f1)
        echo "APK: $APK_PATH ($APK_SIZE)"
    else
        echo -e "${RED}❌ APK non trovato in $APK_PATH${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ Flutter APK build fallita${NC}"
    exit 1
fi
echo ""

# Step 3: Install on all devices
echo -e "${BLUE}📲 Step 3/3: Installing APK su dispositivi...${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

INSTALLED_COUNT=0
FAILED_COUNT=0

echo "$DEVICES" | while read -r device; do
    echo -ne "${YELLOW}→ Installing su $device... ${NC}"
    
    if adb -s "$device" install -r "$APK_PATH" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Installato${NC}"
        INSTALLED_COUNT=$((INSTALLED_COUNT + 1))
    else
        echo -e "${RED}✗ Fallito${NC}"
        FAILED_COUNT=$((FAILED_COUNT + 1))
    fi
done

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Deploy completato!${NC}"
echo ""
echo "Dispositivi installati: $DEVICE_COUNT"
echo ""

# Launch options
echo -e "${BLUE}🚀 Vuoi avviare l'app?${NC}"
echo "  1) Sì, su tutti i dispositivi"
echo "  2) Sì, solo sul primo dispositivo"
echo "  3) No, avvierò manualmente"
read -p "Scelta [1-3, default=3]: " LAUNCH_CHOICE
LAUNCH_CHOICE=${LAUNCH_CHOICE:-3}

PACKAGE_NAME="com.example.otter_mobile"  # Change if different

case $LAUNCH_CHOICE in
    1)
        echo ""
        echo -e "${BLUE}🚀 Avvio app su tutti i dispositivi...${NC}"
        echo "$DEVICES" | while read -r device; do
            echo "  → Avvio su $device"
            adb -s "$device" shell am start -n "$PACKAGE_NAME/.MainActivity" > /dev/null 2>&1
        done
        ;;
    2)
        FIRST_DEVICE=$(echo "$DEVICES" | head -n1)
        echo ""
        echo -e "${BLUE}🚀 Avvio app su $FIRST_DEVICE...${NC}"
        adb -s "$FIRST_DEVICE" shell am start -n "$PACKAGE_NAME/.MainActivity"
        ;;
    3)
        echo ""
        echo -e "${CYAN}ℹ️  Avvia manualmente l'app sui dispositivi${NC}"
        ;;
esac

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}   Deploy completato con successo! 🎉${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Tail logs
if [ "$LAUNCH_CHOICE" != "3" ]; then
    echo -e "${BLUE}📋 Vuoi vedere i log dell'app?${NC}"
    echo "  1) Sì, tail logcat in tempo reale"
    echo "  2) No"
    read -p "Scelta [1-2, default=2]: " LOG_CHOICE
    LOG_CHOICE=${LOG_CHOICE:-2}
    
    if [ "$LOG_CHOICE" == "1" ]; then
        FIRST_DEVICE=$(echo "$DEVICES" | head -n1)
        echo ""
        echo -e "${CYAN}📋 Logcat da $FIRST_DEVICE (Ctrl+C per uscire):${NC}"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        adb -s "$FIRST_DEVICE" logcat | grep -E "flutter|otter|NetworkService|PeerDiscovered"
    fi
fi

echo ""
echo "Happy testing! 🦦"
