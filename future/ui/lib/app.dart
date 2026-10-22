import 'package:flutter/material.dart';

import 'theme/app_theme.dart';
import 'screens/splash_screen.dart';

class UniKeyApp extends StatelessWidget {
  final bool darkMode;

  const UniKeyApp({super.key, this.darkMode = true});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'UniKey',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.light(),
      darkTheme: AppTheme.dark(),
      themeMode: darkMode ? ThemeMode.dark : ThemeMode.light,
      home: const SplashScreen(),
    );
  }
}
