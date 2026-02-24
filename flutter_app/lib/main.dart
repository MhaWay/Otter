import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'lib/screens/home_screen.dart';
import 'lib/screens/loading_screen.dart';
import 'lib/screens/main_app_screen.dart';
import 'lib/models/app_state.dart';

void main() {
  runApp(
    ChangeNotifierProvider(
      create: (_) => AppState(),
      child: const OtterApp(),
    ),
  );
}

class OtterApp extends StatelessWidget {
  const OtterApp({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Otter - Privacy-Focused Chat',
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF6699FF),
          brightness: Brightness.dark,
        ),
        fontFamily: 'Roboto',
      ),
      home: const OtterHome(),
      debugShowCheckedModeBanner: false,
    );
  }
}

class OtterHome extends StatelessWidget {
  const OtterHome({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Consumer<AppState>(
      builder: (context, appState, _) {
        switch (appState.currentScreen) {
          case Screen.home:
            return const HomeScreen();
          case Screen.loading:
            return const LoadingScreen();
          case Screen.mainApp:
            return const MainAppScreen();
          default:
            return const HomeScreen();
        }
      },
    );
  }
}
