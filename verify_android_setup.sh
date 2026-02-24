#!/bin/bash

# 🦦 Otter - Verifica Completa Ambiente Android
# Esegue tutti i controlli necessari e mostra summary finale

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

echo -e "${CYAN}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "   🦦 Otter - Verifica End-to-End"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${NC}"

# Reload environment
source ~/.bashrc 2>/dev/null || true

# Test 1: Flutter
echo -ne "${BLUE}[1/12]${NC} Flutter.................. "
if command -v flutter &> /dev/null; then
    VERSION=$(flutter --version 2>&1 | head -n1 | grep -oP 'Flutter \K[0-9.]+' || echo "unknown")
    echo -e "${GREEN}✓ v$VERSION${NC}"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "${RED}✗ Not found${NC}"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# Test 2: Rust
echo -ne "${BLUE}[2/12]${NC} Rust..................... "
if command -v rustc &> /dev/null; then
    VERSION=$(rustc --version | grep -oP 'rustc \K[0-9.]+')
    echo -e "${GREEN}✓ v$VERSION${NC}"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "${RED}✗ Not found${NC}"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# Test 3: cargo-ndk 
echo -ne "${BLUE}[3/12]${NC} cargo-ndk................ "
if command -v cargo-ndk &> /dev/null; then
    VERSION=$(cargo ndk --version 2>&1 | head -n1 | grep -oP 'cargo-ndk \K[0-9.]+' || echo "installed")
    echo -e "${GREEN}✓ v$VERSION${NC}"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "${RED}✗ Not found${NC}"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# Test 4: Android SDK
echo -ne "${BLUE}[4/12]${NC} ANDROID_HOME............. "
if [ -n "$ANDROID_HOME" ] && [ -d "$ANDROID_HOME" ]; then
    echo -e "${GREEN}✓ $ANDROID_HOME${NC}"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "${RED}✗ Not set${NC}"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# Test 5: Android NDK
echo -ne "${BLUE}[5/12]${NC} ANDROID_NDK_HOME......... "
if [ -n "$ANDROID_NDK_HOME" ] && [ -d "$ANDROID_NDK_HOME" ]; then
NDK_VERSION=$(basename "$ANDROID_NDK_HOME")
    echo -e "${GREEN}✓ v$NDK_VERSION${NC}"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "${RED}✗ Not set${NC}"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# Test 6: ADB
echo -ne "${BLUE}[6/12]${NC} adb (platform-tools)..... "
if command -v adb &> /dev/null; then
    VERSION=$(adb --version 2>&1 | head -n1 | grep -oP 'version \K[0-9.]+')
    echo -e "${GREEN}✓ v$VERSION${NC}"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "${RED}✗ Not found${NC}"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# Test 7: Rust target aarch64
echo -ne "${BLUE}[7/12]${NC} Target aarch64-android... "
if rustup target list 2>/dev/null | grep -q "aarch64-linux-android (installed)"; then
    echo -e "${GREEN}✓ Installed${NC}"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "${YELLOW}⚠ Not installed${NC}"
    WARN_COUNT=$((WARN_COUNT + 1))
fi

# Test 8: Rust target armv7
echo -ne "${BLUE}[8/12]${NC} Target armv7-android..... "
if rustup target list 2>/dev/null | grep -q "armv7-linux-androideabi (installed)"; then
    echo -e "${GREEN}✓ Installed${NC}"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "${YELLOW}⚠ Not installed${NC}"
    WARN_COUNT=$((WARN_COUNT + 1))
fi

# Test 9: FFI Library exists
echo -ne "${BLUE}[9/12]${NC} libotter_mobile.so....... "
FFI_LIB="flutter_app/android/app/src/main/jniLibs/arm64-v8a/libotter_mobile.so"
if [ -f "$FFI_LIB" ]; then
    SIZE=$(du -h "$FFI_LIB" | cut -f1)
    echo -e "${GREEN}✓ $SIZE${NC}"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "${YELLOW}⚠ Not built (run ./quick_build_android.sh)${NC}"
    WARN_COUNT=$((WARN_COUNT + 1))
fi

# Test 10: Flutter dependencies
echo -ne "${BLUE}[10/12]${NC} Flutter dependencies..... "
if [ -d "flutter_app/.dart_tool" ]; then
    echo -e "${GREEN}✓ Installed${NC}"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "${YELLOW}⚠ Run: cd flutter_app && flutter pub get${NC}"
    WARN_COUNT=$((WARN_COUNT + 1))
fi

# Test 11: Scripts executable
echo -ne "${BLUE}[11/12]${NC} Deploy scripts........... "
SCRIPTS_OK=true
for script in deploy_android.sh list_android_devices.sh install_apk.sh quick_build_android.sh check_android_setup.sh; do
    if [ ! -x "$script" ]; then
        SCRIPTS_OK=false
    fi
done
if [ "$SCRIPTS_OK" = true ]; then
    echo -e "${GREEN}✓ All executable${NC}"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "${YELLOW}⚠ Some not executable (run: chmod +x *.sh)${NC}"
    WARN_COUNT=$((WARN_COUNT + 1))
fi

# Test 12: Android devices
echo -ne "${BLUE}[12/12]${NC} Connected devices........ "
if command -v adb &> /dev/null; then
    DEVICE_COUNT=$(adb devices 2>/dev/null | tail -n +2 | grep -c "device$" 2>/dev/null || echo "0")
    if [ "$DEVICE_COUNT" -gt 0 ] 2>/dev/null; then
        echo -e "${GREEN}✓ $DEVICE_COUNT device(s)${NC}"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo -e "${YELLOW}⚠ None (connect device via USB)${NC}"
        WARN_COUNT=$((WARN_COUNT + 1))
    fi
else
    echo -e "${YELLOW}⚠ adb not available${NC}"
    WARN_COUNT=$((WARN_COUNT + 1))
fi

# Summary
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}   Summary${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e " ${GREEN}✓ Passed:${NC}  $PASS_COUNT"
echo -e " ${YELLOW}⚠ Warnings:${NC} $WARN_COUNT"
echo -e " ${RED}✗ Failed:${NC}  $FAIL_COUNT"
echo ""

# Status determination
if [ "$FAIL_COUNT" -eq 0 ] && [ "$WARN_COUNT" -eq 0 ]; then
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}   ✓ PERFETTO! Ambiente completamente configurato${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "${MAGENTA}🚀 Ready to deploy!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Connect Android device via USB"
    echo "  2. ./deploy_android.sh"
    echo ""
    exit 0
elif [ "$FAIL_COUNT" -eq 0 ]; then
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}   ⚠ OK con avvisi minori${NC}"
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "${YELLOW}Warnings can be ignored if you plan to:${NC}"
    echo "  - Use existing FFI library (already built)"
    echo "  - Install dependencies later"
    echo "  - Connect devices later"
    echo ""
    echo -e "${MAGENTA}🚀 Ready to deploy with warnings!${NC}"
    echo ""
    exit 0
else
    echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${RED}   ✗ ERRORI rilevati${NC}"
    echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo "Fix errors above, then re-run:"
    echo "  ./verify_android_setup.sh"
    echo ""
    echo "For detailed setup instructions:"
    echo "  cat ANDROID_SETUP.md"
    echo ""
    exit 1
fi
