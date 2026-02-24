#!/bin/bash

# 🦦 Otter - Lista Dispositivi Android
# Mostra tutti i dispositivi Android connessi via ADB

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}   📱 Dispositivi Android Connessi${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Check for adb
if ! command -v adb &> /dev/null; then
    echo -e "${YELLOW}❌ adb non trovato${NC}"
    echo "Configura: export PATH=\$PATH:\$ANDROID_HOME/platform-tools"
    exit 1
fi

# Get devices
DEVICES=$(adb devices | tail -n +2 | grep -v "^$")

if [ -z "$DEVICES" ]; then
    echo -e "${YELLOW}⚠️  Nessun dispositivo connesso${NC}"
    echo ""
    echo "Connetti un dispositivo via USB/WiFi e abilita Debug USB"
    echo ""
    exit 0
fi

DEVICE_COUNT=$(echo "$DEVICES" | grep -c "device$" || echo 0)
UNAUTHORIZED_COUNT=$(echo "$DEVICES" | grep -c "unauthorized" || echo 0)

if [ "$DEVICE_COUNT" -gt 0 ]; then
    echo -e "${GREEN}✓ Dispositivi autorizzati: $DEVICE_COUNT${NC}"
    echo ""
    
    echo "$DEVICES" | grep "device$" | while read -r line; do
        DEVICE_ID=$(echo "$line" | awk '{print $1}')
        
        # Get device info
        MODEL=$(adb -s "$DEVICE_ID" shell getprop ro.product.model 2>/dev/null | tr -d '\r')
        MANUFACTURER=$(adb -s "$DEVICE_ID" shell getprop ro.product.manufacturer 2>/dev/null | tr -d '\r')
        ANDROID_VERSION=$(adb -s "$DEVICE_ID" shell getprop ro.build.version.release 2>/dev/null | tr -d '\r')
        SDK_VERSION=$(adb -s "$DEVICE_ID" shell getprop ro.build.version.sdk 2>/dev/null | tr -d '\r')
        ABI=$(adb -s "$DEVICE_ID" shell getprop ro.product.cpu.abi 2>/dev/null | tr -d '\r')
        
        # Check connection type
        if [[ "$DEVICE_ID" == *":"* ]]; then
            CONNECTION="WiFi"
        else
            CONNECTION="USB"
        fi
        
        echo -e "${CYAN}╔══════════════════════════════════════════${NC}"
        echo -e "${CYAN}║${NC} Device ID:    $DEVICE_ID"
        echo -e "${CYAN}║${NC} Manufacturer: $MANUFACTURER"
        echo -e "${CYAN}║${NC} Model:        $MODEL"
        echo -e "${CYAN}║${NC} Android:      $ANDROID_VERSION (SDK $SDK_VERSION)"
        echo -e "${CYAN}║${NC} CPU ABI:      $ABI"
        echo -e "${CYAN}║${NC} Connection:   $CONNECTION"
        echo -e "${CYAN}╚══════════════════════════════════════════${NC}"
        echo ""
    done
fi

if [ "$UNAUTHORIZED_COUNT" -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Dispositivi non autorizzati: $UNAUTHORIZED_COUNT${NC}"
    echo ""
    echo "$DEVICES" | grep "unauthorized" | while read -r line; do
        DEVICE_ID=$(echo "$line" | awk '{print $1}')
        echo "  • $DEVICE_ID - Controlla popup 'Allow USB debugging' sul dispositivo"
    done
    echo ""
fi

# ADB commands helper
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}   Comandi ADB Utili${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "# Installa APK su un device specifico:"
echo "  adb -s <device_id> install -r app.apk"
echo ""
echo "# Avvia app:"
echo "  adb -s <device_id> shell am start -n com.example.otter_mobile/.MainActivity"
echo ""
echo "# Logcat filtrato:"
echo "  adb -s <device_id> logcat | grep -E 'flutter|otter'"
echo ""
echo "# Screenshot:"
echo "  adb -s <device_id> exec-out screencap -p > screenshot.png"
echo ""
echo "# Connetti via WiFi (dopo prima connessione USB):"
echo "  adb tcpip 5555"
echo "  adb connect <device_ip>:5555"
echo ""
