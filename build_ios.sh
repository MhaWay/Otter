#!/bin/bash
# Build script for iOS app with Rust FFI compilation

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$SCRIPT_DIR"
FLUTTER_APP="$WORKSPACE_ROOT/flutter_app"

echo "🦦 Otter iOS Build Script"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check prerequisites
check_prerequisites() {
    echo "📋 Checking prerequisites..."
    
    # Check macOS
    if [ "$(uname)" != "Darwin" ]; then
        echo "❌ iOS build only works on macOS"
        exit 1
    fi
    echo "✅ macOS detected"
    
    # Check Xcode
    if ! command -v xcodebuild &> /dev/null; then
        echo "❌ Xcode not found. Install from App Store"
        exit 1
    fi
    echo "✅ Xcode: $(xcodebuild -version | head -n1)"
    
    # Check Flutter
    if ! command -v flutter &> /dev/null; then
        echo "❌ Flutter not found. Install from https://flutter.dev/docs/get-started"
        exit 1
    fi
    echo "✅ Flutter: $(flutter --version | head -n1)"
    
    # Check Rust
    if ! command -v rustc &> /dev/null; then
        echo "❌ Rust not found. Install from https://rustup.rs"
        exit 1
    fi
    echo "✅ Rust: $(rustc --version)"
    
    # Check CocoaPods
    if ! command -v pod &> /dev/null; then
        echo "❌ CocoaPods not found. Installing..."
        sudo gem install cocoapods
    fi
    echo "✅ CocoaPods: $(pod --version)"
}

# Install iOS Rust targets
install_rust_targets() {
    echo ""
    echo "📦 Installing Rust iOS targets..."
    
    targets=(
        "aarch64-apple-ios"
        "aarch64-apple-ios-sim"
        "x86_64-apple-ios"
    )
    
    for target in "${targets[@]}"; do
        if ! rustup target list | grep -q "^$target (installed)"; then
            echo "  Installing $target..."
            rustup target add "$target"
        fi
    done
    echo "✅ All iOS targets installed"
}

# Build Rust FFI library
build_rust_ffi() {
    echo ""
    echo "🔨 Building Rust FFI library..."
    
    cd "$WORKSPACE_ROOT"
    
    # Create frameworks directory
    FRAMEWORKS_DIR="$FLUTTER_APP/ios/Frameworks"
    mkdir -p "$FRAMEWORKS_DIR"
    
    # Determine architecture based on build destination
    if [ "$BUILD_TYPE" = "simulator" ]; then
        targets=("aarch64-apple-ios-sim" "x86_64-apple-ios")
    elif [ "$BUILD_TYPE" = "device" ]; then
        targets=("aarch64-apple-ios")
    else
        # Both
        targets=("aarch64-apple-ios" "aarch64-apple-ios-sim" "x86_64-apple-ios")
    fi
    
    declare -a built_files
    
    for target in "${targets[@]}"; do
        echo ""
        echo "  📦 Building for $target..."
        
        cargo build -p otter-mobile \
            --target "$target" \
            --lib \
            --release
        
        src="$WORKSPACE_ROOT/target/$target/release/libotter_mobile.a"
        
        if [ -f "$src" ]; then
            size=$(ls -lh "$src" | awk '{print $5}')
            built_files+=("$src")
            echo "  ✅ Built: $src ($size)"
        else
            echo "  ❌ Failed to build for $target"
            exit 1
        fi
    done
    
    # Create universal binary if both device and simulator architectures built
    if [ ${#built_files[@]} -gt 1 ]; then
        echo ""
        echo "  🔗 Creating universal binary..."
        
        universal_output="$FRAMEWORKS_DIR/libotter_mobile.a"
        lipo "${built_files[@]}" -create -output "$universal_output"
        
        if [ -f "$universal_output" ]; then
            size=$(ls -lh "$universal_output" | awk '{print $5}')
            echo "  ✅ Universal binary: $universal_output ($size)"
        else
            echo "  ⚠️  Could not create universal binary, using first target"
            cp "${built_files[0]}" "$universal_output"
        fi
    else
        cp "${built_files[0]}" "$FRAMEWORKS_DIR/libotter_mobile.a"
        echo "✅ Copied library to $FRAMEWORKS_DIR"
    fi
}

# Install CocoaPods dependencies
install_pods() {
    echo ""
    echo "📦 Installing CocoaPods dependencies..."
    
    cd "$FLUTTER_APP/ios"
    
    # Clean Pod cache if requested
    if [ "$CLEAN_PODS" = "yes" ]; then
        echo "  🗑️  Cleaning Pod cache..."
        rm -rf Pods Podfile.lock
    fi
    
    pod install --repo-update
    echo "✅ Pods installed"
}

# Build Flutter app
build_flutter() {
    echo ""
    echo "🏗️  Building Flutter App..."
    
    cd "$FLUTTER_APP"
    
    # Clean previous build
    flutter clean
    flutter pub get
    
    if [ "$BUILD_TYPE" = "device" ] || [ "$BUILD_TYPE" = "release" ]; then
        echo "📦 Building release for iOS device..."
        flutter build ios --release
        
        echo ""
        echo "✅ Release build complete"
        echo ""
        echo "Next steps:"
        echo "  1. Open Xcode: open ios/Runner.xcworkspace"
        echo "  2. Select target device in Xcode"
        echo "  3. Product > Archive"
        echo "  4. Upload to App Store Connect"
    else
        echo "📦 Building for iOS simulator..."
        flutter build ios
        
        echo ""
        echo "✅ Simulator build complete"
        echo ""
        echo "To run on simulator:"
        echo "  flutter run"
        echo ""
        echo "Or open in Xcode:"
        echo "  open ios/Runner.xcworkspace"
    fi
}

# Main flow
main() {
    BUILD_TYPE="${1:-simulator}"
    CLEAN_PODS="${2:-no}"
    
    case "$BUILD_TYPE" in
        simulator|device|release|all)
            ;;
        *)
            echo "Usage: $0 [simulator|device|release|all] [clean-pods]"
            echo ""
            echo "Examples:"
            echo "  $0 simulator        # Build for simulator (default)"
            echo "  $0 device           # Build for iOS device"
            echo "  $0 release          # Build release for App Store"
            echo "  $0 simulator clean  # Clean pods + rebuild"
            exit 1
            ;;
    esac
    
    check_prerequisites
    install_rust_targets
    build_rust_ffi
    install_pods
    build_flutter
    
    echo ""
    echo "✨ Build complete!"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

main "$@"
