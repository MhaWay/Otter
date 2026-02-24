# iOS Build Configuration

This directory contains iOS-specific build configuration for Otter mobile app with Rust FFI integration.

## Prerequisites

1. **Xcode** 14.3 or newer
2. **macOS** 12.0 or newer
3. **Rust toolchain** with iOS targets
4. **CocoaPods** (for dependency management)

## Setup

### 1. Install iOS Rust Targets

```bash
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
```

### 2. Get Dependencies

```bash
cd flutter_app/ios
pod install --repo-update
```

This:
- Installs Flutter framework
- Compiles otter-mobile Rust library (via Podfile)
- Creates .a static library for linking

### 3. Verify Setup

```bash
ls -la flutter_app/ios/Frameworks/
# Should see: libotter_mobile.a
```

## Building

### Debug Build (Simulator)

```bash
flutter run
# or
xcode_project_path=$(find . -name "Runner.xcworkspace")
open "$xcode_project_path"
```

In Xcode:
- Select "Any iOS Simulator" scheme
- Build (Cmd+B) or Run (Cmd+R)

### Release Build (Device)

```bash
flutter build ios --release
```

Then upload via:
- Xcode > Product > Archive
- Or command: `flutter build ios --release --obfuscate`

### Device Deployment

```bash
# List connected devices
xcode-select --install  # Install Xcode command line tools first
ios-deploy --detect

# Install to device
flutter run -d <device_id>
# or
flutter install
```

## Troubleshooting

### Issue: "pod install" fails

**Solution**: 
```bash
cd flutter_app/ios
rm -rf Pods Podfile.lock
pod install --repo-update
```

### Issue: "Module map file not found"

**Cause**: Rust FFI compilation failed

**Solution**:
```bash
cd /path/to/Otter  # workspace root
rustup target add aarch64-apple-ios
cargo build -p otter-mobile --target aarch64-apple-ios --release
```

Verify static library created:
```bash
ls -la target/aarch64-apple-ios/release/libotter_mobile.a
```

### Issue: Linker error "Undefined symbol for architecture arm64"

**Cause**: libotter_mobile.a not linked properly

**Solution**:
1. In Xcode, select Runner target
2. Build Phases → Link Binary With Libraries
3. Add `libotter_mobile.a` from Frameworks folder
4. Ensure Frameworks folder in Build Settings:
   ```
   Other Linker Flags: -Lflutter_app/ios/Frameworks
   ```

### Issue: "Command 'rustup' not found"

**Solution**: Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### Issue: Build succeeds but app crashes on startup

**Cause**: FFI library not loaded properly

**Solution**:
1. Verify `GeneratedPluginRegistrant.m` exists
2. Check Info.plist for app initialization
3. Use Xcode debugger to check native library loading

## File Structure

```
ios/
├── Runner/
│   ├── GeneratedPluginRegistrant.m
│   ├── Info.plist
│   ├── Assets.xcassets/
│   └── Base.lproj/
├── Frameworks/                       # Generated FFI libraries
│   ├── libotter_mobile.a
│   └── libotter_mobile_*.a          # Per-architecture variants
├── Podfile                           # iOS dependencies + FFI build
└── README.md
```

## Architecture Notes

- **Deployment Target**: iOS 12.0+
- **Supported Architectures**:
  - `aarch64-apple-ios` (device: iPhone/iPad)
  - `aarch64-apple-ios-sim` (simulator: Apple Silicon)
  - `x86_64-apple-ios` (simulator: Intel)
- **FFI Binding**: Swift/Objective-C ↔ Rust C ABI
- **Static Library**: libotter_mobile.a compiled per-architecture

## Rust FFI Integration

### Updating Rust Bindings

When `otter-mobile` FFI exports change:

1. Regenerate Swift/Objective-C bridging headers:
```bash
cd /path/to/Otter
cargo build -p otter-mobile --lib
```

2. Rebuild iOS app:
```bash
cd flutter_app
flutter clean
flutter pub get
cd ios
rm -rf Pods Podfile.lock
pod install --repo-update
flutter run
```

## Testing on Device

```bash
# List connected devices
instruments -s devices

# Build and install to first device
flutter run -d "<Device Name>"

# View device console
xcode-select --install
deviceconsole
```

## Deployment to App Store

### 1. Create App ID & Certificate

Via [Apple Developer Console](https://developer.apple.com/account/):
- App IDs
- Signing Certificates
- Provisioning Profiles

### 2. Configure Xcode Project

```bash
open Runner.xcworkspace
# In Xcode UI:
# Target > Signing & Capabilities
#   ✓ Automatically manage signing
#   ✓ Select your Apple ID
```

### 3. Build for Distribution

```bash
flutter build ios --release --obfuscate --split-debug-info=.
```

### 4. Archive & Upload

```bash
open ios/Runner.xcworkspace
# Xcode > Product > Archive
# Cmd+Shift+K to analyze
# Use "Distribute App" button
```

Alt. command-line:
```bash
xcodebuild -workspace ios/Runner.xcworkspace \
  -scheme Runner \
  -configuration Release \
  -derivedDataPath ios/build \
  -archivePath ios/build/Runner.xcarchive \
  archive

xcodebuild -exportArchive \
  -archivePath ios/build/Runner.xcarchive \
  -exportOptionsPlist ios/ExportOptions.plist \
  -exportPath ios/build/DistributionBuilds
```

## Swift/Objective-C FFI Bindings

Example calling Rust functions from Swift:

```swift
import Foundation

// Load Rust library
private let lib = dlopen("libotter_mobile", RTLD_NOW)

// Define external C functions
@_silgen_name("otter_mobile_generate_identity")
func otter_mobile_generate_identity() -> UnsafeMutablePointer<CChar>

@_silgen_name("otter_mobile_free_string")
func otter_mobile_free_string(_ ptr: UnsafeMutablePointer<CChar>)

@_silgen_name("otter_mobile_get_version")
func otter_mobile_get_version() -> UnsafeMutablePointer<CChar>

// Usage
let idPtr = otter_mobile_generate_identity()
if let idStr = String(cString: idPtr, encoding: .utf8) {
    print("Peer ID: \(idStr)")
    otter_mobile_free_string(idPtr)
}
```

## See Also

- [Flutter iOS Build Documentation](https://flutter.dev/docs/deployment/ios)
- [Xcode Build System Reference](https://developer.apple.com/xcode/)
- [Rust FFI Guide](https://docs.rust-embedded.org/book/c-language-bindings.html)
- [Apple App Store Guidelines](https://developer.apple.com/app-store/review/guidelines/)
