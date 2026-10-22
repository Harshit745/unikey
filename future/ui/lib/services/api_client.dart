import 'dart:convert';

import 'package:http/http.dart' as http;

import '../models/device.dart';
import '../models/health_status.dart';
import '../models/pairing_session.dart';
import '../models/peer.dart';

/// HTTP client that talks to the local UniKey daemon API.
class ApiClient {
  final String _baseUrl;

  ApiClient({int port = 45895}) : _baseUrl = 'http://127.0.0.1:$port/api/v1';

  final http.Client _client = http.Client();

  // ── Health ──────────────────────────────────────────────────────────

  Future<HealthStatus> getHealth() async {
    final res = await _get('/health');
    return HealthStatus.fromJson(res['data'] as Map<String, dynamic>? ?? {});
  }

  // ── Devices ─────────────────────────────────────────────────────────

  Future<List<TrustedDevice>> getDevices() async {
    final res = await _get('/devices');
    final list = res['data'] as List<dynamic>? ?? [];
    return list
        .map((e) => TrustedDevice.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<bool> renameDevice(String id, String newName) async {
    final res = await _put('/devices/$id', {'name': newName});
    return res['success'] == true;
  }

  Future<bool> removeDevice(String id) async {
    final res = await _delete('/devices/$id');
    return res['success'] == true;
  }

  // ── Peers ───────────────────────────────────────────────────────────

  Future<List<Peer>> getPeers() async {
    final res = await _get('/peers');
    final list = res['data'] as List<dynamic>? ?? [];
    return list
        .map((e) => Peer.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  // ── Pairing ─────────────────────────────────────────────────────────

  Future<PairingSession> generatePairingSession() async {
    final res = await _post('/pairing/generate', {});
    return PairingSession.fromJson(res);
  }

  Future<InitiatePairingResponse> initiatePairing(
    String pairCode,
    String peerId,
  ) async {
    try {
      final res = await _post('/pairing/initiate', {
        'pair_code': pairCode,
        'peer_id': peerId,
      });
      return InitiatePairingResponse.fromJson(res);
    } on Exception catch (e) {
      return InitiatePairingResponse(
        success: false,
        error: e.toString(),
      );
    }
  }

  Future<PairingStatus> getPairingStatus(String sessionId) async {
    final res = await _get('/pairing/status/$sessionId');
    return PairingStatus.fromJson(res);
  }

  // ── HTTP helpers ────────────────────────────────────────────────────

  Future<Map<String, dynamic>> _get(String path) async {
    final uri = Uri.parse('$_baseUrl$path');
    final res = await _client.get(uri).timeout(const Duration(seconds: 10));
    return jsonDecode(res.body) as Map<String, dynamic>;
  }

  Future<Map<String, dynamic>> _post(
    String path,
    Map<String, dynamic> body,
  ) async {
    final uri = Uri.parse('$_baseUrl$path');
    final res = await _client
        .post(uri, headers: _headers, body: jsonEncode(body))
        .timeout(const Duration(seconds: 35));
    return jsonDecode(res.body) as Map<String, dynamic>;
  }

  Future<Map<String, dynamic>> _put(
    String path,
    Map<String, dynamic> body,
  ) async {
    final uri = Uri.parse('$_baseUrl$path');
    final res = await _client
        .put(uri, headers: _headers, body: jsonEncode(body))
        .timeout(const Duration(seconds: 10));
    return jsonDecode(res.body) as Map<String, dynamic>;
  }

  Future<Map<String, dynamic>> _delete(String path) async {
    final uri = Uri.parse('$_baseUrl$path');
    final res = await _client.delete(uri).timeout(const Duration(seconds: 10));
    return jsonDecode(res.body) as Map<String, dynamic>;
  }

  Map<String, String> get _headers => {
        'Content-Type': 'application/json',
      };

  void dispose() {
    _client.close();
  }
}
