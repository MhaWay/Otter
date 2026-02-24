import 'dart:ffi' as ffi;
import 'dart:io';
import 'dart:convert';
import 'package:ffi/ffi.dart';

/// FFI Bridge to Rust otter-mobile library
class NativeBridge {
  static NativeBridge? _instance;
  late ffi.DynamicLibrary _lib;
  
  // FFI function signatures
  late final ffi.Pointer<ffi.Char> Function() _generateIdentity;
  late final ffi.Pointer<ffi.Char> Function() _getVersion;
  late final void Function(ffi.Pointer<ffi.Char>) _freeString;
  late final ffi.Pointer<ffi.Char> Function(ffi.Pointer<ffi.NativeFunction<ffi.Void Function(ffi.Pointer<ffi.Char>)>>) _registerCallback;
  late final ffi.Pointer<ffi.Char> Function(ffi.Pointer<ffi.Char>) _startNetwork;
  late final ffi.Pointer<ffi.Char> Function() _getPeers;
  late final ffi.Pointer<ffi.Char> Function(ffi.Pointer<ffi.Char>, ffi.Pointer<ffi.Char>) _sendMessage;
  late final ffi.Pointer<ffi.Char> Function() _stopNetwork;
  
  // Event callback
  void Function(Map<String, dynamic>)? onEvent;
  
  NativeBridge._() {
    _loadLibrary();
  }
  
  static NativeBridge get instance {
    _instance ??= NativeBridge._();
    return _instance!;
  }
  
  void _loadLibrary() {
    if (Platform.isAndroid) {
      _lib = ffi.DynamicLibrary.open('libotter_mobile.so');
    } else if (Platform.isIOS) {
      _lib = ffi.DynamicLibrary.process();
    } else {
      throw UnsupportedError('Platform not supported');
    }
    
    // Bind functions
    _generateIdentity = _lib
        .lookup<ffi.NativeFunction<ffi.Pointer<ffi.Char> Function()>>('otter_mobile_generate_identity')
        .asFunction();
        
    _getVersion = _lib
        .lookup<ffi.NativeFunction<ffi.Pointer<ffi.Char> Function()>>('otter_mobile_get_version')
        .asFunction();
        
    _freeString = _lib
        .lookup<ffi.NativeFunction<ffi.Void Function(ffi.Pointer<ffi.Char>)>>('otter_mobile_free_string')
        .asFunction();
        
    _registerCallback = _lib
        .lookup<ffi.NativeFunction<ffi.Pointer<ffi.Char> Function(ffi.Pointer<ffi.NativeFunction<ffi.Void Function(ffi.Pointer<ffi.Char>)>>)>>('otter_mobile_register_callback')
        .asFunction();
        
    _startNetwork = _lib
        .lookup<ffi.NativeFunction<ffi.Pointer<ffi.Char> Function(ffi.Pointer<ffi.Char>)>>('otter_mobile_start_network')
        .asFunction();
        
    _getPeers = _lib
        .lookup<ffi.NativeFunction<ffi.Pointer<ffi.Char> Function()>>('otter_mobile_get_peers')
        .asFunction();
        
    _sendMessage = _lib
        .lookup<ffi.NativeFunction<ffi.Pointer<ffi.Char> Function(ffi.Pointer<ffi.Char>, ffi.Pointer<ffi.Char>)>>('otter_mobile_send_message')
        .asFunction();
        
    _stopNetwork = _lib
        .lookup<ffi.NativeFunction<ffi.Pointer<ffi.Char> Function()>>('otter_mobile_stop_network')
        .asFunction();
  }
  
  String _getStringFromPointer(ffi.Pointer<ffi.Char> ptr) {
    if (ptr == ffi.nullptr) return '{}';
    final str = ptr.cast<Utf8>().toDartString();
    _freeString(ptr);
    return str;
  }
  
  Map<String, dynamic> _getJsonFromPointer(ffi.Pointer<ffi.Char> ptr) {
    final str = _getStringFromPointer(ptr);
    try {
      return json.decode(str) as Map<String, dynamic>;
    } catch (e) {
      return {'success': false, 'error': 'JSON parse error: $e'};
    }
  }
  
  /// Generate new identity
  Map<String, dynamic> generateIdentity() {
    final ptr = _generateIdentity();
    return _getJsonFromPointer(ptr);
  }
  
  /// Get version info
  Map<String, dynamic> getVersion() {
    final ptr = _getVersion();
    return _getJsonFromPointer(ptr);
  }
  
  /// Register event callback
  void registerEventCallback(void Function(Map<String, dynamic>) callback) {
    onEvent = callback;
    
    // Create native callback
    final nativeCallback = ffi.Pointer.fromFunction<ffi.Void Function(ffi.Pointer<ffi.Char>)>(
      _eventCallbackHandler,
    );
    
    final ptr = _registerCallback(nativeCallback);
    final result = _getJsonFromPointer(ptr);
    
    if (result['success'] != true) {
      print('Failed to register callback: ${result['error']}');
    }
  }
  
  static void _eventCallbackHandler(ffi.Pointer<ffi.Char> eventPtr) {
    if (eventPtr == ffi.nullptr) return;
    
    final eventStr = eventPtr.cast<Utf8>().toDartString();
    try {
      final event = json.decode(eventStr) as Map<String, dynamic>;
      NativeBridge.instance.onEvent?.call(event);
    } catch (e) {
      print('Error parsing event: $e');
    }
  }
  
  /// Start network with identity
  Map<String, dynamic> startNetwork(Map<String, dynamic> identity) {
    final identityJson = json.encode(identity);
    final identityPtr = identityJson.toNativeUtf8();
    
    final resultPtr = _startNetwork(identityPtr.cast());
    malloc.free(identityPtr);
    
    return _getJsonFromPointer(resultPtr);
  }
  
  /// Get connected peers
  Map<String, dynamic> getPeers() {
    final ptr = _getPeers();
    return _getJsonFromPointer(ptr);
  }
  
  /// Send message to topic
  Map<String, dynamic> sendMessage(String topic, String message) {
    final topicPtr = topic.toNativeUtf8();
    final messagePtr = message.toNativeUtf8();
    
    final resultPtr = _sendMessage(topicPtr.cast(), messagePtr.cast());
    
    malloc.free(topicPtr);
    malloc.free(messagePtr);
    
    return _getJsonFromPointer(resultPtr);
  }
  
  /// Stop network
  Map<String, dynamic> stopNetwork() {
    final ptr = _stopNetwork();
    return _getJsonFromPointer(ptr);
  }
}
