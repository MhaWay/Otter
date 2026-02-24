import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/app_state.dart';
import '../services/network_service.dart';

class LoadingScreen extends StatefulWidget {
  const LoadingScreen({Key? key}) : super(key: key);

  @override
  State<LoadingScreen> createState() => _LoadingScreenState();
}

class _LoadingScreenState extends State<LoadingScreen>
    with SingleTickerProviderStateMixin {
  late AnimationController _spinnerController;
  bool _networkStarted = false;

  @override
  void initState() {
    super.initState();
    _spinnerController = AnimationController(
      duration: const Duration(milliseconds: 1000),
      vsync: this,
    )..repeat();

    // Start network initialization
    _initializeNetwork();
  }
  
  Future<void> _initializeNetwork() async {
    if (_networkStarted) return;
    _networkStarted = true;
    
    final appState = context.read<AppState>();
    final networkService = NetworkService.instance;
    
    appState.addLoadingLog('🦦 Inizializzazione Otter...');
    appState.addLoadingLog('🔐 Generazione identità P2P...');
    
    // Initialize network
    final success = await networkService.initialize();
    
    if (success) {
      appState.setPeerId(networkService.peerId ?? 'Unknown');
      appState.addLoadingLog('✅ Identità: ${networkService.peerId?.substring(0, 16)}...');
      appState.addLoadingLog('🌐 Avvio rete P2P...');
      appState.addLoadingLog('🔍 Ricerca peers DHT...');
      appState.addLoadingLog('📡 Bootstrap...');
      
      // Listen to network changes
      networkService.addListener(() {
        if (networkService.isConnected && mounted) {
          appState.setNetworkReady(true);
          appState.addLoadingLog('🎉 Rete pronta! Peers: ${networkService.peers.length}');
          
          Future.delayed(const Duration(seconds: 1), () {
            if (mounted) {
              appState.setScreen(Screen.mainApp);
            }
          });
        }
      });
      
      // 14 second timeout
      Future.delayed(const Duration(seconds: 14), () {
        if (mounted && appState.currentScreen == Screen.loading) {
          appState.addLoadingLog('⏱️ Timeout - continuando...');
          appState.addLoadingLog('ℹ️ Rete isolata o no peers');
          appState.setScreen(Screen.mainApp);
        }
      });
    } else {
      appState.addLoadingLog('❌ Errore rete');
      Future.delayed(const Duration(seconds: 2), () {
        if (mounted) {
          appState.setScreen(Screen.home);
        }
      });
    }
  }

  @override
  void dispose() {
    _spinnerController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: SingleChildScrollView(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Text(
                '🌐 Connessione alla rete...',
                style: TextStyle(fontSize: 28, fontWeight: FontWeight.bold),
              ),
              const SizedBox(height: 20),
              const Text(
                'Connessione alla rete P2P...',
                style: TextStyle(fontSize: 16, color: Colors.grey),
              ),
              const SizedBox(height: 30),
              // Spinner
              RotationTransition(
                turns: _spinnerController,
                child: Container(
                  width: 100,
                  height: 100,
                  decoration: BoxDecoration(
                    border: Border.all(
                      color: const Color(0xFF6699FF),
                      width: 4,
                    ),
                    borderRadius: BorderRadius.circular(50),
                  ),
                  child: const Center(
                    child: Text(
                      '🦦',
                      style: TextStyle(fontSize: 48),
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 30),
              // Logs
              Consumer<AppState>(
                builder: (context, appState, _) {
                  return Container(
                    width: 300,
                    height: 200,
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: Colors.black12,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(
                        color: Colors.grey.withOpacity(0.5),
                      ),
                    ),
                    child: SingleChildScrollView(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: appState.loadingLogs.isEmpty
                            ? [
                                const Text(
                                  'In attesa eventi di rete...',
                                  style: TextStyle(
                                    color: Colors.grey,
                                    fontSize: 12,
                                  ),
                                ),
                              ]
                            : appState.loadingLogs
                                .map((log) => Padding(
                                      padding: const EdgeInsets.symmetric(
                                        vertical: 2,
                                      ),
                                      child: Text(
                                        log,
                                        style: const TextStyle(
                                          color: Colors.white70,
                                          fontSize: 11,
                                          fontFamily: 'monospace',
                                        ),
                                      ),
                                    ))
                                .toList(),
                      ),
                    ),
                  );
                },
              ),
            ],
          ),
        ),
      ),
    );
  }
}

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: SingleChildScrollView(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Text(
                '🌐 Connessione alla rete...',
                style: TextStyle(fontSize: 28, fontWeight: FontWeight.bold),
              ),
              const SizedBox(height: 20),
              const Text(
                'Connessione alla rete P2P...',
                style: TextStyle(fontSize: 16, color: Colors.grey),
              ),
              const SizedBox(height: 30),
              // Spinner
              RotationTransition(
                turns: _spinnerController,
                child: Container(
                  width: 100,
                  height: 100,
                  decoration: BoxDecoration(
                    border: Border.all(
                      color: const Color(0xFF6699FF),
                      width: 4,
                    ),
                    borderRadius: BorderRadius.circular(50),
                  ),
                  child: const Center(
                    child: Text(
                      '🦦',
                      style: TextStyle(fontSize: 48),
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 30),
              // Logs
              Consumer<AppState>(
                builder: (context, appState, _) {
                  return Container(
                    width: 300,
                    height: 200,
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: Colors.black12,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(
                        color: Colors.grey.withOpacity(0.5),
                      ),
                    ),
                    child: SingleChildScrollView(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: appState.loadingLogs.isEmpty
                            ? [
                                const Text(
                                  'In attesa eventi di rete...',
                                  style: TextStyle(
                                    color: Colors.grey,
                                    fontSize: 12,
                                  ),
                                ),
                              ]
                            : appState.loadingLogs
                                .map((log) => Padding(
                                      padding: const EdgeInsets.symmetric(
                                        vertical: 2,
                                      ),
                                      child: Text(
                                        log,
                                        style: const TextStyle(
                                          color: Colors.white70,
                                          fontSize: 11,
                                          fontFamily: 'monospace',
                                        ),
                                      ),
                                    ))
                                .toList(),
                      ),
                    ),
                  );
                },
              ),
            ],
          ),
        ),
      ),
    );
  }
}
