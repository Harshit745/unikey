class HealthStatus {
  final String status;
  final String version;
  final String deviceName;
  final String deviceId;
  final String platform;

  HealthStatus({
    required this.status,
    required this.version,
    required this.deviceName,
    required this.deviceId,
    required this.platform,
  });

  factory HealthStatus.fromJson(Map<String, dynamic> json) {
    return HealthStatus(
      status: json['status'] as String? ?? '',
      version: json['version'] as String? ?? '',
      deviceName: json['device_name'] as String? ?? '',
      deviceId: json['device_id'] as String? ?? '',
      platform: json['platform'] as String? ?? '',
    );
  }

  bool get isRunning => status == 'running';
}
