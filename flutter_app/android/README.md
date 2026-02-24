# Android Build Configuration

This directory contains Android-specific build configuration for Otter mobile app with Rust FFI integration.

## Prerequisites

1. **Android Studio** (2023.1 or newer)
2. **Android SDK**: API 34 (Android 14)
3. **Android NDK**: Version 25.2.9519653
4. **Rust toolchain** (see root README)

## Setup

### 1. Android SDK & NDK Installation

Via Android Studio:
```
Settings → Languages & Frameworks → Android SDK
  ✓ Android 14 (API 34)
  ✓ Build Tools 34.x.x
  ✓ Android NDK 25.2.9519653
  ✓ Android SDK Tools
```

Or via `sdkmanager`:
```bash
sdkmanager "platforms;android-34"
sdkmanager "build-tools;34.0.0"
sdkmanager "ndk;25.2.9519653"
```

### 2. Configure local.properties

Copy `local.properties.example` to `local.properties` and update:
```properties
sdk.dir=/path/to/Android/Sdk
ndkVersion=25.2.9519653
```

### 3. Install Rust Android Targets

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk
```

## Building

### Debug Build (APK)

```bash
flutter build apk --debug
```

This will:
1. Invoke gradle build
2. Gradle calls `buildRustLib` task
3. `cargo ndk` compiles Rust for Android targets
4. FFI `.so` libraries placed in `app/src/main/jniLibs/`
5. APK assembled with embedded native libraries

### Release Build (APK - optimized)

```bash
flutter build apk --release
```

### Split APKs by ABI (smaller downloads)

```bash
flutter build apk --release --split-per-abi
```

Produces:
- `app-arm64-v8a-release.apk` (~35MB)
- `app-armeabi-v7a-release.apk` (~30MB)
- `app-x86_64-release.apk` (~35MB)

## Troubleshooting

### Issue: "Cargo workspace not found"
**Solution**: Ensure full Rust project is in `../../` from flutter_app

### Issue: "NDK not found"
**Solution**: 
```bash
# Set android.ndkPath in local.properties
android.ndkPath=/path/to/Android/Sdk/ndk/25.2.9519653
```

### Issue: "Could not load library 'otter_mobile'"
**Cause**: FFI library failed to compile or not copied to jniLibs/
**Solution**:
```bash
flutter clean
flutter pub get
flutter build apk --verbose  # Show detailed output
```

### Issue: "Rust target not found"
**Solution**:
```bash
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
cargo ndk --version  # Verify cargo-ndk installed
```

### Issue: NDK toolchain path error
**Solution**: Update `app/build.gradle` paths for your NDK:
```gradle
def ndkToolchain = "${ndkDir}/toolchains/llvm/prebuilt/linux-x86_64"
// or for macOS:
def ndkToolchain = "${ndkDir}/toolchains/llvm/prebuilt/darwin-x86_64"
```

## File Structure

```
android/
├── app/
│   ├── src/
│   │   └── main/
│   │       ├── AndroidManifest.xml
│   │       ├── kotlin/com/otter/chat/
│   │       │   └── MainActivity.kt          # Native binding
│   │       ├── jniLibs/                    # Generated FFI libraries
│   │       │   ├── arm64-v8a/libotter_mobile.so
│   │       │   ├── armeabi-v7a/libotter_mobile.so
│   │       │   └── x86_64/libotter_mobile.so
│   │       └── res/                        # Android resources
│   ├── build.gradle                        # FFI compilation task
│   └── proguard-rules.pro
├── build.gradle                            # Top-level gradle config
├── gradle.properties                       # Gradle settings
├── settings.gradle                         # Subproject config
└── local.properties.example
```

## Testing on Device

```bash
# List connected devices
adb devices

# Install and run on first device
flutter run

# Install specific device
flutter run -d emulator-5554

# Live reload during development
# Press 'r' in terminal to reload Dart code
```

## Deployment

### Google Play Store

1. Create signed keystore:
   ```bash
   keytool -genkey -v -keystore ~/upload-keystore.jks \
     -keyalg RSA -keysize 4096 -validity 10950 \
     -alias upload
   ```

2. Create `~/.android/key.properties`:
   ```properties
   storePassword=<password>
   keyPassword=<password>
   keyAlias=upload
   storeFile=/path/to/upload-keystore.jks
   ```

3. Build signed release:
   ```bash
   flutter build appbundle --release
   ```

4. Upload to Play Console

See full guide: https://flutter.dev/docs/deployment/android

## Architecture Notes

- **libp2p native**: Via Rust + cargo ndk compilation
- **FFI boundary**: MainActivity.kt exposes native functions to Dart
- **ABI targets**: arm64-v8a (primary), armeabi-v7a, x86_64 (emulator)
- **Minimum SDK**: 21 (Android 5.0)
- **Target SDK**: 34 (Android 14)

## See Also

- [Flutter Android Build Documentation](https://flutter.dev/docs/deployment/android)
- [Android NDK Documentation](https://developer.android.com/ndk)
- [Rust FFI Integration](https://rust-lang.github.io/rfcs/2435-project-safe-ffi.html)
