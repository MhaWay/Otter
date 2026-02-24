import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/app_state.dart';

class MainAppScreen extends StatefulWidget {
  const MainAppScreen({Key? key}) : super(key: key);

  @override
  State<MainAppScreen> createState() => _MainAppScreenState();
}

class _MainAppScreenState extends State<MainAppScreen> {
  int _selectedTab = 0;

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
    return Center(
      child: Consumer<AppState>(
        builder: (context, appState, _) {
          return Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Text(
                '✓ Rete Pronta',
                style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
              ),
              const SizedBox(height: 20),
              Text(
                'Peer ID: ${appState.peerId?.substring(0, 8)}...',
                style: const TextStyle(fontSize: 12, color: Colors.grey),
              ),
              const SizedBox(height: 30),
              ElevatedButton(
                onPressed: () {
                  context.read<AppState>().setScreen(Screen.home);
                },
                child: const Text('Logout'),
              ),
            ],
          );
        },
      ),
    );
  }

  Widget _buildContactsTab() {
    return const Center(
      child: Text('Contatti - In sviluppo'),
    );
  }

  Widget _buildProfileTab() {
    return const Center(
      child: Text('Profilo - In sviluppo'),
    );
  }

  Widget _buildSettingsTab() {
    return const Center(
      child: Text('Impostazioni - In sviluppo'),
    );
  }
}
