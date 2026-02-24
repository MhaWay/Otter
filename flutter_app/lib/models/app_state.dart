import 'package:flutter/foundation.dart';

enum Screen {
  home,
  loading,
  mainApp,
}

class AppState extends ChangeNotifier {
  Screen _currentScreen = Screen.loading;
  String? _peerId;
  String? _nickname;
  bool _isNetworkReady = false;
  List<String> _loadingLogs = [];

  Screen get currentScreen => _currentScreen;
  String? get peerId => _peerId;
  String? get nickname => _nickname;
  bool get isNetworkReady => _isNetworkReady;
  List<String> get loadingLogs => _loadingLogs;

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
}
