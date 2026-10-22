import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';

import 'app.dart';
import 'providers/daemon_provider.dart';
import 'providers/devices_provider.dart';
import 'providers/pairing_provider.dart';
import 'services/api_client.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();

  // Set window defaults for desktop
  SystemChrome.setPreferredOrientations([
    DeviceOrientation.landscapeLeft,
    DeviceOrientation.landscapeRight,
  ]);

  // Shared API client
  final apiClient = ApiClient();

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider(
          create: (_) => DaemonProvider(apiClient),
        ),
        ChangeNotifierProvider(
          create: (_) => DevicesProvider(apiClient),
        ),
        ChangeNotifierProvider(
          create: (_) => PairingProvider(apiClient),
        ),
      ],
      child: const UniKeyApp(),
    ),
  );
}
