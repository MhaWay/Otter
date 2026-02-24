#!/bin/bash

echo "🦦 Otter Android Setup Checker"
echo "================================"
echo ""

EXIT_CODE=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Flutter
echo -n "Flutter: "
if command -v flutter &> /dev/null; then
    VERSION=$(flutter --version 2>/dev/null | head -n1)
    echo -e "${GREEN}✓${NC} $VERSION"
else
    echo -e "${RED}❌ NOT FOUND${NC}"
    echo "   Install: https://docs.flutter.dev/get-started/install"
    EXIT_CODE=1
fi

# Rust
echo -n "Rust: "
if command -v rustc &> /dev/null; then
    VERSION=$(rustc --version)
    echo -e "${GREEN}✓${NC} $VERSION"
else
    echo -e "${RED}❌ NOT FOUND${NC}"
    echo "   Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    EXIT_CODE=1
fi

# cargo-ndk
echo -n "cargo-ndk: "
if command -v cargo-ndk &> /dev/null; then
    VERSION=$(cargo ndk --version 2>&1 | head -n1)
    echo -e "${GREEN}✓${NC} $VERSION"
else
    echo -e "${RED}❌ NOT FOUND${NC}"
    echo "   Install: cargo install cargo-ndk"
    EXIT_CODE=1
fi

# Android SDK
echo -n "Android SDK: "
if [ -n "$ANDROID_HOME" ] && [ -d "$ANDROID_HOME" ]; then
    echo -e "${GREEN}✓${NC} $ANDROID_HOME"
else
    echo -e "${RED}❌ ANDROID_HOME not set or directory missing${NC}"
    echo "   Install Android Studio or command-line tools"
    echo "   Set: export ANDROID_HOME=/path/to/Android/Sdk"
    EXIT_CODE=1
fi

# Android NDK
echo -n "Android NDK: "
if [ -n "$ANDROID_NDK_HOME" ] && [ -d "$ANDROID_NDK_HOME" ]; then
    echo -e "${GREEN}✓${NC} $ANDROID_NDK_HOME"
elif [ -n "$ANDROID_HOME" ] && [ -d "$ANDROID_HOME/ndk" ]; then
    NDK_VERSION=$(ls "$ANDROID_HOME/ndk" 2>/dev/null | head -n1)
    if [ -n "$NDK_VERSION" ]; then
        echo -e "${YELLOW}⚠${NC} Found but ANDROID_NDK_HOME not set"
        echo "   Set: export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/$NDK_VERSION"
        EXIT_CODE=1
    else
        echo -e "${RED}❌ NOT FOUND${NC}"
        echo "   Install: sdkmanager 'ndk;26.1.10909125'"
        EXIT_CODE=1
    fi
else
    echo -e "${RED}❌ NOT FOUND${NC}"
    echo "   Install NDK via Android Studio → SDK Manager → SDK Tools → NDK"
    EXIT_CODE=1
fi

# ADB
echo -n "ADB: "
if command -v adb &> /dev/null; then
    VERSION=$(adb --version 2>&1 | head -n1)
    echo -e "${GREEN}✓${NC} $VERSION"
else
    echo -e "${RED}❌ NOT FOUND${NC}"
    echo "   Should be installed with Android SDK platform-tools"
    EXIT_CODE=1
fi

# Rust targets
echo ""
echo "Rust Android targets:"
TARGETS_FOUND=0
for TARGET in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android; do
    if rustup target list 2>/dev/null | grep -q "^$TARGET (installed)"; then
        echo -e "  ${GREEN}✓${NC} $TARGET"
        TARGETS_FOUND=$((TARGETS_FOUND + 1))
    else
        echo -e "  ${YELLOW}○${NC} $TARGET (not installed)"
    fi
done

if [ $TARGETS_FOUND -eq 0 ]; then
    echo -e "  ${RED}❌ No Android targets installed${NC}"
    echo "     Install: rustup target add aarch64-linux-android"
    EXIT_CODE=1
fi

# Devices
echo ""
echo "Connected Android devices:"
if command -v adb &> /dev/null; then
    DEVICES=$(adb devices 2>/dev/null | tail -n +2 | grep -v "^$" | grep "device$")
    if [ -n "$DEVICES" ]; then
        echo "$DEVICES" | while read -r line; do
            echo -e "  ${GREEN}✓${NC} $line"
        done
    else
        echo -e "  ${YELLOW}⚠${NC} No devices connected"
        echo "     Connect device via USB and enable USB debugging"
    fi
else
    echo -e "  ${RED}❌ adb not available${NC}"
fi

# Flutter doctor
echo ""
echo "Flutter Doctor Summary:"
if command -v flutter &> /dev/null; then
    flutter doctor 2>&1 | grep -E "^\[" | while read -r line; do
        if echo "$line" | grep -q "✓"; then
            echo -e "${GREEN}$line${NC}"
        elif echo "$line" | grep -q "✗"; then
            echo -e "${RED}$line${NC}"
        else
            echo -e "${YELLOW}$line${NC}"
        fi
    done
else
    echo -e "${RED}❌ Flutter not available${NC}"
fi

# Final status
echo ""
echo "================================"
if [ $EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}✓ Setup looks good! Ready to build.${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. ./quick_build_android.sh"
    echo "  2. cd flutter_app && flutter run"
    echo ""
else
    echo -e "${RED}❌ Some issues detected. Review errors above.${NC}"
    echo ""
    echo "See ANDROID_SETUP.md for detailed installation instructions."
    echo ""
fi

exit $EXIT_CODE
