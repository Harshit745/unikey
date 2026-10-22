class TrustedDevice {
  final String id;
  final String name;
  final String platform;
  final String? alias;
  final String lastSeen;

  TrustedDevice({
    required this.id,
    required this.name,
    required this.platform,
    this.alias,
    required this.lastSeen,
  });

  factory TrustedDevice.fromJson(Map<String, dynamic> json) {
    return TrustedDevice(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? '',
      platform: json['platform'] as String? ?? '',
      alias: json['alias'] as String?,
      lastSeen: json['last_seen'] as String? ?? '',
    );
  }

  String get displayName => alias ?? name;

  String get platformIcon {
    switch (platform.toLowerCase()) {
      case 'macos':
        return '';
      case 'windows':
        return '⊞';
      case 'linux':
        return '🐧';
      case 'android':
        return '📱';
      case 'ios':
        return '📲';
      default:
        return '💻';
    }
  }
}
