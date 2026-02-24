import 'package:flutter/foundation.dart';

enum Screen {
  home,
  loading,
  mainApp,
}

class Peer {
  final String peerId;
  final String nickname;
  final DateTime? connectedAt;
  
  Peer({
    required this.peerId,
    required this.nickname,
    this.connectedAt,
  });
  
  String get shortId => peerId.length > 12 
      ? '${peerId.substring(0, 8)}...${peerId.substring(peerId.length - 4)}' 
      : peerId;
}

class AppState extends ChangeNotifier {
  Screen _currentScreen = Screen.loading;
  String? _peerId;
  String? _nickname;
  bool _isNetworkReady = false;
  List<String> _loadingLogs = [];
  List<Peer> _connectedPeers = [];

  Screen get currentScreen => _currentScreen;
  String? get peerId => _peerId;
  String? get nickname => _nickname;
  bool get isNetworkReady => _isNetworkReady;
  List<String> get loadingLogs => _loadingLogs;
  List<Peer> get connectedPeers => _connectedPeers;
  
  int get peerCount => _connectedPeers.length;

  void setScreen(Screen screen) {
    _currentScreen = screen;
    notifyListeners();
  }

  void setPeerId(String id) {
    _peerId = id;
    notifyListeners();
  }

  void setNickname(String name) {
    _nickname = name;
    notifyListeners();
  }

  void setNetworkReady(bool ready) {
    _isNetworkReady = ready;
    notifyListeners();
  }

  void addLoadingLog(String log) {
    _loadingLogs.add(log);
    notifyListeners();
  }

  void clearLoadingLogs() {
    _loadingLogs.clear();
    notifyListeners();
  }
  
  void updatePeers(List<Peer> peers) {
    _connectedPeers = peers;
    notifyListeners();
  }
  
  void addPeer(Peer peer) {
    if (!_connectedPeers.any((p) => p.peerId == peer.peerId)) {
      _connectedPeers.add(peer);
      notifyListeners();
    }
  }
  
  void removePeer(String peerId) {
    _connectedPeers.removeWhere((p) => p.peerId == peerId);
    notifyListeners();
  }
}
