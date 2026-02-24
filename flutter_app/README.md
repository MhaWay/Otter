# Otter Mobile (Flutter + Rust FFI)

Flutter mobile app for Otter privacy-focused P2P chat platform.

## Architecture

- **Flutter UI**: Cross-platform mobile interface (iOS/Android)
- **Rust FFI Backend**: Direct native calls to compiled `otter-mobile` Rust crate
- **Core P2P**: Shares same network library as desktop (libp2p, DHT, Gossipsub)

## Setup

### Prerequisites
- Flutter SDK (3.0+)
- Rust (1.70+)
- Android SDK / iOS Setup

### Running

```bash
# In flutter_app directory
flutter pub get
flutter run
```

## Development

### Screens Implemented
- [ ] **HomeScreen**: Login/Register
- [ ] **LoadingScreen**: Network initialization with 14s timeout
- [x] **MainAppScreen**: Chat interface (skeleton)

### Features TODO
- Connect to Rust FFI layer
- Network initialization
- Identity management
- Contact list
- Message sending/receiving
- Google Sign-In integration
- Google Drive backup

## Building for Android

```bash
# Build release APK
flutter build apk --release

# Build split APKs
flutter build apk --release --split-per-abi
```

## Building for iOS

```bash
# Build release
flutter build ios --release
```

## FFI Integration

The app will communicate with `otter-mobile` Rust crate via FFI:

```dart
// Example (future implementation)
import 'dart:ffi' as ffi;

typedef GenerateIdentityFunc = ffi.Pointer<ffi.Char> Function();
final generateIdentity = otterLib.lookupFunction<GenerateIdentityFunc, GenerateIdentityFunc>('otter_mobile_generate_identity');
```

See `../../crates/otter-mobile` for FFI export definitions.
