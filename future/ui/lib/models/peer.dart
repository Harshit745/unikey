class Peer {
  final String id;
  final String name;
  final String platform;
  final String address;
  final String state;

  Peer({
    required this.id,
    required this.name,
    required this.platform,
    required this.address,
    required this.state,
  });

  factory Peer.fromJson(Map<String, dynamic> json) {
    return Peer(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? '',
      platform: json['platform'] as String? ?? '',
      address: json['address'] as String? ?? '',
      state: json['state'] as String? ?? 'Online',
    );
  }

  bool get isOnline => state.toLowerCase() == 'online';

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
