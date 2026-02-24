import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/app_state.dart';
import '../services/network_service.dart';

class MainAppScreen extends StatefulWidget {
  const MainAppScreen({Key? key}) : super(key: key);

  @override
  State<MainAppScreen> createState() => _MainAppScreenState();
}

class _MainAppScreenState extends State<MainAppScreen> {
  int _selectedTab = 0;

  @override
  void initState() {
    super.initState();
    NetworkService.instance.addListener(_onNetworkUpdate);
  }
  
  @override
  void dispose() {
    NetworkService.instance.removeListener(_onNetworkUpdate);
    super.dispose();
  }
  
  void _onNetworkUpdate() {
    if (!mounted) return;
    final appState = context.read<AppState>();
    final networkService = NetworkService.instance;
    
    final peers = networkService.peers.map((p) => Peer(
      peerId: p.peerId,
      nickname: p.nickname,
      connectedAt: p.connectedAt,
    )).toList();
    
    appState.updatePeers(peers);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('🦦 Otter'),
        elevation: 0,
      ),
      body: _buildTabContent(_selectedTab),
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _selectedTab,
        onTap: (index) {
          setState(() => _selectedTab = index);
        },
        items: const [
          BottomNavigationBarItem(
            icon: Icon(Icons.home),
            label: 'Home',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.people),
            label: 'Contatti',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.person),
            label: 'Profilo',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.settings),
            label: 'Impostazioni',
          ),
        ],
      ),
    );
  }

  Widget _buildTabContent(int tab) {
    switch (tab) {
      case 0:
        return _buildHomeTab();
      case 1:
        return _buildContactsTab();
      case 2:
        return _buildProfileTab();
      case 3:
        return _buildSettingsTab();
      default:
        return _buildHomeTab();
    }
  }

  Widget _buildHomeTab() {
    return Consumer<AppState>(
      builder: (context, appState, _) {
        return ListView(
          padding: const EdgeInsets.all(20),
          children: [
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  children: [
                    Text(
                      appState.isNetworkReady ? '✅ Rete Connessa' : '⏳ In attesa...',
                      style: const TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                    ),
                    const SizedBox(height: 12),
                    Text(
                      '${appState.peerCount} peers connessi',
                      style: const TextStyle(fontSize: 16, color: Colors.grey),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 16),
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'Il tuo Peer ID:',
                      style: TextStyle(fontSize: 12, color: Colors.grey),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      appState.peerId ?? 'N/A',
                      style: const TextStyle(fontSize: 11, fontFamily: 'monospace'),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 24),
            ElevatedButton.icon(
              onPressed: () async {
                final ns = NetworkService.instance;
                await ns.sendMessage('otter-global', 'Test da ${appState.peerId?.substring(0, 8)}');
                if (mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('Messaggio inviato!')),
                  );
                }
              },
              icon: const Icon(Icons.send),
              label: const Text('Invia Test Message'),
            ),
          ],
        );
      },
    );
  }

  Widget _buildContactsTab() {
    return Consumer<AppState>(
      builder: (context, appState, _) {
        if (appState.connectedPeers.isEmpty) {
          return const Center(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(Icons.people, size: 64, color: Colors.grey),
                SizedBox(height: 16),
                Text('Nessun peer connesso', style: TextStyle(color: Colors.grey)),
              ],
            ),
          );
        }
        
        return ListView.builder(
          padding: const EdgeInsets.all(16),
          itemCount: appState.connectedPeers.length,
          itemBuilder: (context, index) {
            final peer = appState.connectedPeers[index];
            return Card(
              child: ListTile(
                leading: CircleAvatar(
                  child: Text(peer.nickname[0].toUpperCase()),
                ),
                title: Text(peer.nickname),
                subtitle: Text(
                  peer.shortId,
                  style: const TextStyle(fontFamily: 'monospace', fontSize: 11),
                ),
                trailing: const Icon(Icons.circle, size: 12, color: Colors.green),
              ),
            );
          },
        );
      },
    );
  }

  Widget _buildProfileTab() {
    final networkService = NetworkService.instance;
    final messages = networkService.messages;
    
    return Column(
      children: [
        Expanded(
          child: messages.isEmpty
              ? const Center(
                  child: Text('Nessun messaggio', style: TextStyle(color: Colors.grey)),
                )
              : ListView.builder(
                  padding: const EdgeInsets.all(16),
                  itemCount: messages.length,
                  itemBuilder: (context, index) {
                    final msg = messages[index];
                    return Card(
                      child: ListTile(
                        leading: const Icon(Icons.message),
                        title: Text(msg.data),
                        subtitle: Text('Da: ${msg.from.substring(0, 8)}...'),
                        trailing: Text(
                          '${msg.timestamp.hour}:${msg.timestamp.minute.toString().padLeft(2, '0')}',
                          style: const TextStyle(fontSize: 11, color: Colors.grey),
                        ),
                      ),
                    );
                  },
                ),
        ),
      ],
    );
  }

  Widget _buildSettingsTab() {
    return Consumer<AppState>(
      builder: (context, appState, _) {
        return ListView(
          children: [
            ListTile(
              leading: const Icon(Icons.info),
              title: const Text('Versione'),
              subtitle: const Text('0.1.0'),
            ),
            ListTile(
              leading: const Icon(Icons.network_check),
              title: const Text('Stato Rete'),
              subtitle: Text(appState.isNetworkReady ? 'Connesso' : 'Disconnesso'),
              trailing: Icon(
                Icons.circle,
                size: 12,
                color: appState.isNetworkReady ? Colors.green : Colors.red,
              ),
            ),
            const Divider(),
            ListTile(
              leading: const Icon(Icons.logout, color: Colors.red),
              title: const Text('Disconnetti', style: TextStyle(color: Colors.red)),
              onTap: () async {
                await NetworkService.instance.stop();
                if (context.mounted) {
                  appState.setScreen(Screen.home);
                }
              },
            ),
          ],
        );
      },
    );
  }
}
