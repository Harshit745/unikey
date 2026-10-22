import 'dart:async';

import 'package:flutter/foundation.dart';

import '../models/health_status.dart';
import '../models/peer.dart';
import '../services/api_client.dart';

/// Possible states of the daemon connection.
enum DaemonState { connecting, connected, error }

/// Provides daemon health status and nearby peer discovery state.
class DaemonProvider extends ChangeNotifier {
  final ApiClient _api;

  DaemonState _state = DaemonState.connecting;
  HealthStatus? _health;
  List<Peer> _peers = [];
  String? _error;
  Timer? _pollTimer;

  DaemonProvider(this._api);

  DaemonState get state => _state;
  HealthStatus? get health => _health;
  List<Peer> get peers => _peers;
  String? get error => _error;
  bool get isConnected => _state == DaemonState.connected;

  /// Start polling the daemon for health + peers.
  void startPolling() {
    _pollTimer?.cancel();
    _poll();
    _pollTimer = Timer.periodic(const Duration(seconds: 3), (_) => _poll());
  }

  void stopPolling() {
    _pollTimer?.cancel();
    _pollTimer = null;
  }

  Future<void> _poll() async {
    try {
      final health = await _api.getHealth();
      _health = health;
      _state = DaemonState.connected;
      _error = null;

      // Fetch peers in background
      _api.getPeers().then((peers) {
        _peers = peers;
        notifyListeners();
      });

      notifyListeners();
    } catch (e) {
      if (_state == DaemonState.connected) {
        _state = DaemonState.connecting;
        _error = null;
        notifyListeners();
      }
    }
  }

  @override
  void dispose() {
    stopPolling();
    super.dispose();
  }
}
