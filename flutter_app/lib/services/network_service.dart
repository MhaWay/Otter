import 'package:flutter/foundation.dart';
import 'dart:async';
import 'native_bridge.dart';

class NetworkService extends ChangeNotifier {
  static NetworkService? _instance;
  
  final NativeBridge _bridge = NativeBridge.instance;
  
  bool _isInitialized = false;
  bool _isConnected = false;
  String? _peerId;
  List<NetworkPeer> _peers = [];
  List<NetworkMessage> _messages = [];
  Timer? _peerPollTimer;
  
  bool get isInitialized => _isInitialized;
  bool get isConnected => _isConnected;
  String? get peerId => _peerId;
  List<NetworkPeer> get peers => _peers;
  List<NetworkMessage> get messages => _messages;
  
  NetworkService._() {
    _setupEventListener();
  }
  
  static NetworkService get instance {
    _instance ??= NetworkService._();
    return _instance!;
  }
  
  void _setupEventListener() {
    _bridge.registerEventCallback((event) {
      _handleNetworkEvent(event);
    });
  }
  
  void _handleNetworkEvent(Map<String, dynamic> event) {
    final eventType = event['event_type'] as String?;
    final data = event['data'] as Map<String, dynamic>?;
    
    if (eventType == null || data == null) return;
    
    print('📡 Network Event: $eventType - $data');
    
    switch (eventType) {
      case 'network_started':
        _peerId = data['peer_id'] as String?;
        _isInitialized = true;
        notifyListeners();
        break;
        
      case 'network_ready':
        _isConnected = true;
        final peerCount = data['peer_count'] as int? ?? 0;
        print('✅ Network ready with $peerCount peers');
        notifyListeners();
        break;
        
      case 'peer_connected':
        final peerId = data['peer_id'] as String?;
        if (peerId != null) {
          print('🔗 Peer connected: $peerId');
          _refreshPeers();
        }
        break;
        
      case 'peer_disconnected':
        final peerId = data['peer_id'] as String?;
        if (peerId != null) {
          print('🔌 Peer disconnected: $peerId');
          _peers.removeWhere((p) => p.peerId == peerId);
          notifyListeners();
        }
        break;
        
      case 'message':
        final from = data['from'] as String?;
        final topic = data['topic'] as String?;
        final messageData = data['data'] as String?;
        
        if (from != null && topic != null && messageData != null) {
          print('💬 Message from $from on $topic');
          _messages.add(NetworkMessage(
            from: from,
            topic: topic,
            data: messageData,
            timestamp: DateTime.now(),
          ));
          notifyListeners();
        }
        break;
        
      case 'bootstrap_complete':
        print('🚀 Bootstrap complete');
        break;
        
      case 'network_stopped':
        _isInitialized = false;
        _isConnected = false;
        _peers.clear();
        notifyListeners();
        break;
    }
  }
  
  /// Initialize network with identity
  Future<bool> initialize({Map<String, dynamic>? identity}) async {
    try {
      final identityData = identity ?? {};
      final result = _bridge.startNetwork(identityData);
      
      if (result['success'] == true) {
        _peerId = result['peer_id'] as String?;
        _isInitialized = true;
        
        // Start polling for peers
        _startPeerPolling();
        
        notifyListeners();
        return true;
      } else {
        print('❌ Network init failed: ${result['error']}');
        return false;
      }
    } catch (e) {
      print('❌ Network init exception: $e');
      return false;
    }
  }
  
  void _startPeerPolling() {
    _peerPollTimer?.cancel();
    _peerPollTimer = Timer.periodic(const Duration(seconds: 2), (_) {
      _refreshPeers();
    });
  }
  
  void _refreshPeers() {
    try {
      final result = _bridge.getPeers();
      if (result['success'] == true) {
        final peersData = result['peers'] as List<dynamic>?;
        if (peersData != null) {
          _peers = peersData.map((p) => NetworkPeer.fromJson(p as Map<String, dynamic>)).toList();
          notifyListeners();
        }
      }
    } catch (e) {
      print('Error refreshing peers: $e');
    }
  }
  
  /// Send message to topic
  Future<bool> sendMessage(String topic, String message) async {
    try {
      final result = _bridge.sendMessage(topic, message);
      return result['success'] == true;
    } catch (e) {
      print('Error sending message: $e');
      return false;
    }
  }
  
  /// Stop network
  Future<void> stop() async {
    _peerPollTimer?.cancel();
    _bridge.stopNetwork();
    _isInitialized = false;
    _isConnected = false;
    _peers.clear();
    _messages.clear();
    notifyListeners();
  }
  
  @override
  void dispose() {
    _peerPollTimer?.cancel();
    super.dispose();
  }
}

class NetworkPeer {
  final String peerId;
  final String nickname;
  final DateTime? connectedAt;
  
  NetworkPeer({
    required this.peerId,
    required this.nickname,
    this.connectedAt,
  });
  
  factory NetworkPeer.fromJson(Map<String, dynamic> json) {
    return NetworkPeer(
      peerId: json['peer_id'] as String,
      nickname: json['nickname'] as String? ?? 'Unknown',
      connectedAt: json['connected_at'] != null
          ? DateTime.tryParse(json['connected_at'] as String)
          : null,
    );
  }
  
  String get shortId => peerId.length > 12 ? '${peerId.substring(0, 8)}...${peerId.substring(peerId.length - 4)}' : peerId;
}

class NetworkMessage {
  final String from;
  final String topic;
  final String data;
  final DateTime timestamp;
  
  NetworkMessage({
    required this.from,
    required this.topic,
    required this.data,
    required this.timestamp,
  });
}
