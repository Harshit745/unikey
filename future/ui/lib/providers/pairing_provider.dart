import 'dart:async';

import 'package:flutter/foundation.dart';

import '../models/pairing_session.dart';
import '../models/peer.dart';
import '../services/api_client.dart';

/// State machine for the pairing flow.
enum PairingFlowState {
  /// Initial state — user hasn't started pairing.
  idle,

  /// Pairing session generated (Device A, QR/Code displayed).
  waiting,

  /// User on Device B has entered a code or scanned QR.
  initiating,

  /// Pairing completed successfully.
  success,

  /// Pairing failed with an error.
  failed,
}

/// Manages the pairing flow for both initiator and responder roles.
class PairingProvider extends ChangeNotifier {
  final ApiClient _api;

  PairingFlowState _flowState = PairingFlowState.idle;
  PairingSession? _currentSession;
  String? _errorMessage;
  String? _pairedDeviceName;
  String? _pairedDevicePlatform;
  Timer? _statusTimer;

  PairingProvider(this._api);

  PairingFlowState get flowState => _flowState;
  PairingSession? get currentSession => _currentSession;
  String? get errorMessage => _errorMessage;
  String? get pairedDeviceName => _pairedDeviceName;
  String? get pairedDevicePlatform => _pairedDevicePlatform;

  // ── Device A: Generate pairing session ──────────────────────────────

  Future<void> generateSession() async {
    _reset();
    _flowState = PairingFlowState.waiting;
    notifyListeners();

    try {
      _currentSession = await _api.generatePairingSession();
      _errorMessage = null;

      // Poll for status changes
      _startStatusPolling();
    } catch (e) {
      _flowState = PairingFlowState.failed;
      _errorMessage = 'Failed to generate pairing session: $e';
    }

    notifyListeners();
  }

  // ── Device B: Initiate pairing with code + peer ─────────────────────

  Future<void> initiateWithCode(String code, Peer peer) async {
    _reset();
    _flowState = PairingFlowState.initiating;
    notifyListeners();

    try {
      final result = await _api.initiatePairing(code, peer.id);
      if (result.success) {
        _flowState = PairingFlowState.success;
        _pairedDeviceName = result.deviceName;
        _pairedDevicePlatform = result.devicePlatform;
        _errorMessage = null;
      } else {
        _flowState = PairingFlowState.failed;
        _errorMessage = result.error ?? 'Pairing was rejected';
      }
    } catch (e) {
      _flowState = PairingFlowState.failed;
      _errorMessage = 'Pairing failed: $e';
    }

    notifyListeners();
  }

  // ── Reset ───────────────────────────────────────────────────────────

  void reset() {
    _stopStatusPolling();
    _reset();
  }

  void _reset() {
    _flowState = PairingFlowState.idle;
    _currentSession = null;
    _errorMessage = null;
    _pairedDeviceName = null;
    _pairedDevicePlatform = null;
  }

  // ── Status polling ──────────────────────────────────────────────────

  void _startStatusPolling() {
    _stopStatusPolling();
    final sid = _currentSession?.sessionId;
    if (sid == null) return;

    _statusTimer = Timer.periodic(const Duration(seconds: 2), (_) async {
      try {
        final status = await _api.getPairingStatus(sid);
        if (status.isCompleted) {
          _stopStatusPolling();
          _flowState = PairingFlowState.success;
          notifyListeners();
        } else if (status.isExpired) {
          _stopStatusPolling();
          _flowState = PairingFlowState.failed;
          _errorMessage = 'Pairing code expired';
          notifyListeners();
        }
      } catch (_) {}
    });
  }

  void _stopStatusPolling() {
    _statusTimer?.cancel();
    _statusTimer = null;
  }

  @override
  void dispose() {
    _stopStatusPolling();
    super.dispose();
  }
}
