import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models/device.dart';
import '../models/peer.dart';
import '../providers/daemon_provider.dart';
import '../providers/devices_provider.dart';
import '../providers/pairing_provider.dart';
import '../theme/app_theme.dart';
import '../widgets/device_card.dart';
import '../widgets/peer_card.dart';
import '../widgets/success_sheet.dart';
import 'pair_device_screen.dart';
import 'settings_screen.dart';

class DashboardScreen extends StatefulWidget {
  const DashboardScreen({super.key});

  @override
  State<DashboardScreen> createState() => _DashboardScreenState();
}

class _DashboardScreenState extends State<DashboardScreen> {
  int _selectedIndex = 0;

  @override
  void initState() {
    super.initState();
    // Start device polling
    final devices = context.read<DevicesProvider>();
    devices.startPolling();
  }

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final bgColor = isDark ? AppColors.darkBg : AppColors.lightBg;
    final surfaceColor = isDark ? AppColors.darkSurface : AppColors.lightSurface;

    return Scaffold(
      backgroundColor: bgColor,
      body: Row(
        children: [
          // ── Sidebar ──────────────────────────────────────────────────
          Container(
            width: 220,
            decoration: BoxDecoration(
              color: surfaceColor,
              border: Border(
                right: BorderSide(
                  color: isDark ? AppColors.darkBorder : AppColors.lightBorder,
                ),
              ),
            ),
            child: Column(
              children: [
                const SizedBox(height: 24),
                // App logo + name
                Row(
                  children: [
                    const SizedBox(width: 20),
                    Container(
                      width: 32,
                      height: 32,
                      decoration: BoxDecoration(
                        color: AppColors.accent,
                        borderRadius: BorderRadius.circular(10),
                      ),
                      child: const Center(
                        child: Text(
                          '⌘',
                          style: TextStyle(
                            fontSize: 16,
                            color: Colors.white,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(width: 10),
                    Text(
                      'UniKey',
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                  ],
                ),
                const SizedBox(height: 32),
                // Navigation items
                _SidebarItem(
                  icon: Icons.devices_rounded,
                  label: 'Devices',
                  selected: _selectedIndex == 0,
                  onTap: () => setState(() => _selectedIndex = 0),
                ),
                _SidebarItem(
                  icon: Icons.add_link_rounded,
                  label: 'Pair Device',
                  selected: _selectedIndex == 1,
                  onTap: () => setState(() => _selectedIndex = 1),
                ),
                _SidebarItem(
                  icon: Icons.settings_rounded,
                  label: 'Settings',
                  selected: _selectedIndex == 2,
                  onTap: () => setState(() => _selectedIndex = 2),
                ),
                const Spacer(),
                // Daemon status indicator
                Consumer<DaemonProvider>(
                  builder: (_, daemon, __) {
                    final connected = daemon.isConnected;
                    return Container(
                      margin: const EdgeInsets.all(16),
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                      decoration: BoxDecoration(
                        color: connected
                            ? AppColors.green.withValues(alpha:0.1)
                            : AppColors.red.withValues(alpha:0.1),
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Row(
                        children: [
                          Container(
                            width: 8,
                            height: 8,
                            decoration: BoxDecoration(
                              shape: BoxShape.circle,
                              color:
                                  connected ? AppColors.green : AppColors.red,
                            ),
                          ),
                          const SizedBox(width: 8),
                          Text(
                            connected ? 'Daemon Active' : 'Disconnected',
                            style: TextStyle(
                              fontSize: 12,
                              color: connected
                                  ? AppColors.green
                                  : AppColors.red,
                            ),
                          ),
                        ],
                      ),
                    );
                  },
                ),
                const SizedBox(height: 8),
              ],
            ),
          ),

          // ── Main Panel ──────────────────────────────────────────────
          Expanded(
            child: AnimatedSwitcher(
              duration: const Duration(milliseconds: 300),
              child: _buildPanel(),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildPanel() {
    switch (_selectedIndex) {
      case 0:
        return _DevicesPanel();
      case 1:
        return const PairDeviceScreen();
      case 2:
        return const SettingsScreen();
      default:
        return _DevicesPanel();
    }
  }
}

// ── Sidebar Item ──────────────────────────────────────────────────────

class _SidebarItem extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  const _SidebarItem({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 2),
      child: Material(
        color: selected
            ? (isDark ? AppColors.accent.withValues(alpha:0.15) : AppColors.accent.withValues(alpha:0.08))
            : Colors.transparent,
        borderRadius: BorderRadius.circular(10),
        child: InkWell(
          borderRadius: BorderRadius.circular(10),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
            child: Row(
              children: [
                Icon(
                  icon,
                  size: 18,
                  color: selected
                      ? AppColors.accent
                      : (isDark ? AppColors.textSecondary : AppColors.textLightSecondary),
                ),
                const SizedBox(width: 12),
                Text(
                  label,
                  style: TextStyle(
                    fontSize: 14,
                    fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                    color: selected
                        ? AppColors.accent
                        : (isDark ? AppColors.textPrimary : AppColors.textLightPrimary),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

// ── Devices Panel ────────────────────────────────────────────────────

class _DevicesPanel extends StatelessWidget {
  void _showRenameDialog(BuildContext context, TrustedDevice device) {
    final controller = TextEditingController(text: device.displayName);
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Rename Device'),
        content: TextField(
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(labelText: 'Name'),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () async {
              final newName = controller.text.trim();
              if (newName.isNotEmpty) {
                await context
                    .read<DevicesProvider>()
                    .renameDevice(device.id, newName);
              }
              if (ctx.mounted) Navigator.pop(ctx);
            },
            child: const Text('Save'),
          ),
        ],
      ),
    );
  }

  void _confirmRemove(BuildContext context, TrustedDevice device) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Remove Device'),
        content: Text('Remove "${device.displayName}" from trusted devices?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            style: ElevatedButton.styleFrom(backgroundColor: AppColors.red),
            onPressed: () async {
              await context.read<DevicesProvider>().removeDevice(device.id);
              if (ctx.mounted) Navigator.pop(ctx);
            },
            child: const Text('Remove'),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final subColor = isDark ? AppColors.textSecondary : AppColors.textLightSecondary;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(32),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'My Devices',
            style: Theme.of(context).textTheme.headlineMedium,
          ),
          const SizedBox(height: 24),

          // Trusted devices
          Consumer<DevicesProvider>(
            builder: (_, provider, __) {
              if (provider.devices.isEmpty) {
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'No trusted devices yet',
                      style: TextStyle(color: subColor, fontSize: 15),
                    ),
                    const SizedBox(height: 6),
                    Text(
                      'Pair a device to get started.',
                      style: TextStyle(color: subColor, fontSize: 13),
                    ),
                  ],
                );
              }

              return Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Trusted Devices',
                      style: TextStyle(
                          fontSize: 13,
                          fontWeight: FontWeight.w600,
                          color: subColor)),
                  const SizedBox(height: 12),
                  ...provider.devices.map(
                    (d) => DeviceCard(
                      device: d,
                      onRename: () => _showRenameDialog(context, d),
                      onRemove: () => _confirmRemove(context, d),
                    ),
                  ),
                ],
              );
            },
          ),

          const SizedBox(height: 32),

          // Nearby devices
          Text(
            'Nearby Devices',
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 4),
          Text(
            'Devices discovered on your network',
            style: TextStyle(color: subColor, fontSize: 13),
          ),
          const SizedBox(height: 16),

          Consumer2<DaemonProvider, DevicesProvider>(
            builder: (_, daemon, devices, __) {
              final trustedIds = devices.devices.map((d) => d.id).toSet();

              if (daemon.peers.isEmpty) {
                return Container(
                  padding: const EdgeInsets.all(24),
                  decoration: BoxDecoration(
                    color: isDark ? AppColors.darkCard : AppColors.lightCard,
                    borderRadius: BorderRadius.circular(16),
                    border: Border.all(
                      color: isDark ? AppColors.darkBorder : AppColors.lightBorder,
                    ),
                  ),
                  child: Center(
                    child: Column(
                      children: [
                        Text(
                          'No devices found nearby',
                          style: TextStyle(color: subColor, fontSize: 14),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          'Make sure UniKey is running on other devices',
                          style: TextStyle(color: subColor, fontSize: 12),
                        ),
                      ],
                    ),
                  ),
                );
              }

              return Column(
                children: daemon.peers
                    .where((p) => !trustedIds.contains(p.id))
                    .map((p) => PeerCard(
                          peer: p,
                          isTrusted: trustedIds.contains(p.id),
                          onPair: () {
                            // Switch to Pair Device tab with pre-selected peer
                            // For now, just show a dialog
                            _showPairWithPeer(context, p);
                          },
                        ))
                    .toList(),
              );
            },
          ),
        ],
      ),
    );
  }

  void _showPairWithPeer(BuildContext context, Peer peer) {
    final codeController = TextEditingController();
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('Pair with ${peer.name}'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              'Enter the 6-digit code shown on ${peer.name}:',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            const SizedBox(height: 16),
            TextField(
              controller: codeController,
              maxLength: 6,
              keyboardType: TextInputType.number,
              textAlign: TextAlign.center,
              style: const TextStyle(fontSize: 24, letterSpacing: 8),
              decoration: const InputDecoration(
                counterText: '',
                hintText: '------',
              ),
              onSubmitted: (_) {
                if (codeController.text.length == 6) {
                  Navigator.pop(ctx);
                  _initiatePairing(context, codeController.text, peer);
                }
              },
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () {
              if (codeController.text.length == 6) {
                Navigator.pop(ctx);
                _initiatePairing(context, codeController.text, peer);
              }
            },
            child: const Text('Pair'),
          ),
        ],
      ),
    );
  }

  void _initiatePairing(BuildContext context, String code, Peer peer) async {
    final provider = context.read<PairingProvider>();
    await provider.initiateWithCode(code, peer);

    if (!context.mounted) return;

    if (provider.flowState == PairingFlowState.success) {
      _showSuccess(context);
    } else if (provider.flowState == PairingFlowState.failed) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(provider.errorMessage ?? 'Pairing failed'),
          backgroundColor: AppColors.red,
        ),
      );
    }
  }

  void _showSuccess(BuildContext context) {
    final provider = context.read<PairingProvider>();
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (_) => PairingSuccessSheet(
        deviceName: provider.pairedDeviceName,
        devicePlatform: provider.pairedDevicePlatform,
        onDone: () {
          Navigator.pop(context);
          provider.reset();
        },
        onRename: () {
          Navigator.pop(context);
          provider.reset();
          // The devices list will refresh automatically
        },
      ),
    );
  }
}
