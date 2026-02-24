#!/bin/bash
# Quick test build for Android - development only

set -e

echo "🦦 Otter Mobile - Quick Android Test Build"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FLUTTER_APP="$WORKSPACE_ROOT/flutter_app"

# Build Rust for Android (arm64 only for speed)
echo "🔨 Building Rust FFI for arm64-v8a..."
cargo ndk \
    -t arm64-v8a \
    -o "$FLUTTER_APP/android/app/src/main/jniLibs" \
    build -p otter-mobile --lib --release

echo "✅ FFI library built!"
ls -lh "$FLUTTER_APP/android/app/src/main/jniLibs/arm64-v8a/"

echo ""
echo "📱 Now run from flutter_app directory:"
echo "   cd flutter_app && flutter run"
