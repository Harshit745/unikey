import 'dart:async';

import 'package:flutter/foundation.dart';

import '../models/device.dart';
import '../services/api_client.dart';

/// Manages the list of trusted (paired) devices.
class DevicesProvider extends ChangeNotifier {
  final ApiClient _api;

  List<TrustedDevice> _devices = [];
  bool _loading = false;
  String? _error;
  Timer? _pollTimer;

  DevicesProvider(this._api);

  List<TrustedDevice> get devices => _devices;
  bool get loading => _loading;
  String? get error => _error;

  void startPolling() {
    refresh();
    _pollTimer = Timer.periodic(const Duration(seconds: 5), (_) => refresh());
  }

  void stopPolling() {
    _pollTimer?.cancel();
    _pollTimer = null;
  }

  Future<void> refresh() async {
    _loading = true;
    notifyListeners();

    try {
      _devices = await _api.getDevices();
      _error = null;
    } catch (e) {
      _error = e.toString();
    }

    _loading = false;
    notifyListeners();
  }

  Future<bool> renameDevice(String id, String newName) async {
    final ok = await _api.renameDevice(id, newName);
    if (ok) await refresh();
    return ok;
  }

  Future<bool> removeDevice(String id) async {
    final ok = await _api.removeDevice(id);
    if (ok) await refresh();
    return ok;
  }

  @override
  void dispose() {
    stopPolling();
    super.dispose();
  }
}
