# Preserve Otter Rust FFI
-keepclasseswithmembernames class * {
    native <methods>;
}

# Keep Flutter classes
-keep class io.flutter.** { *; }
-keep class io.flutter.plugins.** { *; }

# Keep Google Sign-In
-keep class com.google.android.gms.** { *; }

# Preserve Google API client
-keep class com.google.api.** { *; }

# Allow obfuscation but keep library exports
-keepclassmembers class com.otter.chat.MainActivity {
    native <methods>;
}

# Suppress warnings
-dontwarn com.google.**
-dontwarn sun.misc.Unsafe
-dontwarn com.sun.jna.**
