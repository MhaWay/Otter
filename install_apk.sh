#!/bin/bash

# 🦦 Otter - Installa APK su Dispositivi Android
# Installa l'APK già buildato su tutti i dispositivi connessi

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}   🦦 Otter - Installa APK${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Check for adb
if ! command -v adb &> /dev/null; then
    echo -e "${RED}❌ adb non trovato${NC}"
    echo "Configura: export PATH=\$PATH:\$ANDROID_HOME/platform-tools"
    exit 1
fi

# Detect connected devices
DEVICES=$(adb devices | tail -n +2 | grep -v "^$" | grep "device$" | cut -f1)
DEVICE_COUNT=$(echo "$DEVICES" | grep -c . || echo 0)

if [ "$DEVICE_COUNT" -eq 0 ]; then
    echo -e "${YELLOW}⚠️  Nessun dispositivo connesso${NC}"
    echo ""
    echo "Usa: ./list_android_devices.sh per vedere i dispositivi"
    exit 1
fi

echo -e "${GREEN}✓ Trovati $DEVICE_COUNT dispositivo/i${NC}"
echo ""

# Find APK
cd "$(dirname "$0")/flutter_app"

APK_DEBUG="build/app/outputs/flutter-apk/app-debug.apk"
APK_RELEASE="build/app/outputs/flutter-apk/app-release.apk"
APK_PROFILE="build/app/outputs/flutter-apk/app-profile.apk"

if [ -f "$APK_RELEASE" ]; then
    APK_PATH="$APK_RELEASE"
    APK_TYPE="Release"
elif [ -f "$APK_DEBUG" ]; then
    APK_PATH="$APK_DEBUG"
    APK_TYPE="Debug"
elif [ -f "$APK_PROFILE" ]; then
    APK_PATH="$APK_PROFILE"
    APK_TYPE="Profile"
else
    echo -e "${RED}❌ Nessun APK trovato${NC}"
    echo ""
    echo "Builda prima l'APK con:"
    echo "  ./deploy_android.sh  (completo)"
    echo "oppure:"
    echo "  cd flutter_app && flutter build apk --release"
    exit 1
fi

APK_SIZE=$(du -h "$APK_PATH" | cut -f1)
echo -e "${GREEN}✓ APK trovato:${NC} $APK_TYPE ($APK_SIZE)"
echo ""

# Install on all devices
echo -e "${BLUE}📲 Installazione su dispositivi...${NC}"
echo ""

INSTALLED=0
FAILED=0

echo "$DEVICES" | while read -r device; do
    MODEL=$(adb -s "$device" shell getprop ro.product.model 2>/dev/null | tr -d '\r')
    echo -ne "${YELLOW}→ $device ($MODEL)... ${NC}"
    
    if adb -s "$device" install -r "$APK_PATH" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Installato${NC}"
        INSTALLED=$((INSTALLED + 1))
    else
        echo -e "${RED}✗ Fallito${NC}"
        FAILED=$((FAILED + 1))
    fi
done

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Installazione completata!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Launch option
echo -e "${BLUE}Vuoi avviare l'app?${NC}"
echo "  1) Sì, su tutti i dispositivi"
echo "  2) No, avvierò manualmente"
read -p "Scelta [1-2, default=2]: " LAUNCH
LAUNCH=${LAUNCH:-2}

if [ "$LAUNCH" == "1" ]; then
    PACKAGE_NAME="com.example.otter_mobile"
    echo ""
    echo -e "${BLUE}🚀 Avvio app...${NC}"
    echo "$DEVICES" | while read -r device; do
        adb -s "$device" shell am start -n "$PACKAGE_NAME/.MainActivity" > /dev/null 2>&1
        echo "  ✓ Avviato su $device"
    done
fi

echo ""
echo "Happy testing! 🦦"
