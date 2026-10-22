class PairingSession {
  final String sessionId;
  final String qrData;
  final String pairCode;
  final int expiresIn;

  PairingSession({
    required this.sessionId,
    required this.qrData,
    required this.pairCode,
    required this.expiresIn,
  });

  factory PairingSession.fromJson(Map<String, dynamic> json) {
    return PairingSession(
      sessionId: json['session_id'] as String? ?? '',
      qrData: json['qr_data'] as String? ?? '',
      pairCode: json['pair_code'] as String? ?? '',
      expiresIn: json['expires_in'] as int? ?? 0,
    );
  }
}

class InitiatePairingResponse {
  final bool success;
  final String? deviceName;
  final String? devicePlatform;
  final String? error;

  InitiatePairingResponse({
    required this.success,
    this.deviceName,
    this.devicePlatform,
    this.error,
  });

  factory InitiatePairingResponse.fromJson(Map<String, dynamic> json) {
    return InitiatePairingResponse(
      success: json['data']?['success'] as bool? ?? false,
      deviceName: json['data']?['device_name'] as String?,
      devicePlatform: json['data']?['device_platform'] as String?,
      error: json['data']?['error'] as String? ?? json['error'] as String?,
    );
  }
}

class PairingStatus {
  final String state;
  final int remainingSeconds;

  PairingStatus({
    required this.state,
    required this.remainingSeconds,
  });

  factory PairingStatus.fromJson(Map<String, dynamic> json) {
    return PairingStatus(
      state: json['data']?['state'] as String? ?? 'Expired',
      remainingSeconds: json['data']?['remaining_seconds'] as int? ?? 0,
    );
  }

  bool get isPending => state == 'Pending';
  bool get isAccepted => state == 'Accepted(Uuid)' || state.startsWith('Accepted');
  bool get isCompleted => state == 'Completed';
  bool get isExpired => state == 'Expired';
}
