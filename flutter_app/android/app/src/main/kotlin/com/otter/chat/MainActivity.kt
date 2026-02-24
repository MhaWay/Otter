package com.otter.chat

import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity: FlutterActivity() {
    private val CHANNEL = "com.otter.chat/native"
    
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "loadNativeLibrary" -> {
                        try {
                            // Load the Rust FFI shared library
                            System.loadLibrary("otter_mobile")
                            result(mapOf(
                                "success" to true,
                                "message" to "otter_mobile library loaded"
                            ))
                        } catch (e: Exception) {
                            result(mapOf(
                                "success" to false,
                                "error" to e.message
                            ))
                        }
                    }
                    "getNativeVersion" -> {
                        try {
                            val version = nativeGetVersion()
                            result(mapOf(
                                "version" to version,
                                "success" to true
                            ))
                        } catch (e: Exception) {
                            result(mapOf(
                                "success" to false,
                                "error" to e.message
                            ))
                        }
                    }
                    else -> result(null)
                }
            }
    }
    
    // Declare native functions from Rust FFI
    external fun nativeGetVersion(): String
    
    companion object {
        init {
            // Load native libraries in correct order
            try {
                System.loadLibrary("otter_mobile")
            } catch (e: UnsatisfiedLinkError) {
                // Library will be loaded via MethodChannel if not found
                System.err.println("Warning: Could not load otter_mobile library at startup: ${e.message}")
            }
        }
    }
}
