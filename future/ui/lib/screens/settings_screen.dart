import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../providers/daemon_provider.dart';
import '../theme/app_theme.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key});

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  bool _launchAtStartup = false;
  bool _autoStartDaemon = true;
  bool _darkMode = true;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(32),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Settings',
            style: Theme.of(context).textTheme.headlineMedium,
          ),
          const SizedBox(height: 32),

          // ── General ──────────────────────────────────────────────────
          _SectionHeader(title: 'General',),
          const SizedBox(height: 12),
          _SettingsCard(
            children: [
              _SwitchTile(
                icon: Icons.power_settings_new_rounded,
                title: 'Launch at startup',
                subtitle: 'Automatically open UniKey when you log in',
                value: _launchAtStartup,
                onChanged: (v) => setState(() => _launchAtStartup = v),
              ),
              _Divider(),
              _SwitchTile(
                icon: Icons.memory_rounded,
                title: 'Start daemon automatically',
                subtitle: 'Run the backend service in the background',
                value: _autoStartDaemon,
                onChanged: (v) => setState(() => _autoStartDaemon = v),
              ),
            ],
          ),

          const SizedBox(height: 24),

          // ── Appearance ───────────────────────────────────────────────
          _SectionHeader(title: 'Appearance'),
          const SizedBox(height: 12),
          _SettingsCard(
            children: [
              ListTile(
                leading: const Icon(Icons.dark_mode_rounded, color: AppColors.accent),
                title: const Text('Dark Mode'),
                subtitle: const Text('Switch between dark and light themes'),
                trailing: SegmentedButton<bool>(
                  segments: const [
                    ButtonSegment(value: false, label: Text('Light', style: TextStyle(fontSize: 13))),
                    ButtonSegment(value: true, label: Text('Dark', style: TextStyle(fontSize: 13))),
                  ],
                  selected: {_darkMode},
                  onSelectionChanged: (v) => setState(() => _darkMode = v.first),
                ),
              ),
            ],
          ),

          const SizedBox(height: 24),

          // ── About ────────────────────────────────────────────────────
          _SectionHeader(title: 'About'),
          const SizedBox(height: 12),
          _SettingsCard(
            children: [
              Consumer<DaemonProvider>(
                builder: (_, daemon, __) {
                  return Column(
                    children: [
                      _InfoTile(
                        icon: Icons.info_outline_rounded,
                        title: 'Version',
                        subtitle: daemon.health?.version ?? '0.1.0',
                      ),
                      _Divider(),
                      _InfoTile(
                        icon: Icons.computer_rounded,
                        title: 'Device',
                        subtitle: daemon.health?.deviceName ?? 'Unknown',
                      ),
                      _Divider(),
                      _InfoTile(
                        icon: Icons.dns_rounded,
                        title: 'Platform',
                        subtitle: daemon.health?.platform ?? 'Unknown',
                      ),
                    ],
                  );
                },
              ),
            ],
          ),
        ],
      ),
    );
  }
}

// ── Reusable components ───────────────────────────────────────────────

class _SectionHeader extends StatelessWidget {
  final String title;
  const _SectionHeader({required this.title});

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
      child: Text(
        title.toUpperCase(),
        style: TextStyle(
          fontSize: 12,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.8,
          color: isDark ? AppColors.textSecondary : AppColors.textLightSecondary,
        ),
      ),
    );
  }
}

class _SettingsCard extends StatelessWidget {
  final List<Widget> children;
  const _SettingsCard({required this.children});

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return Container(
      decoration: BoxDecoration(
        color: isDark ? AppColors.darkCard : AppColors.lightCard,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: isDark ? AppColors.darkBorder : AppColors.lightBorder,
        ),
      ),
      child: Column(children: children),
    );
  }
}

class _SwitchTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final bool value;
  final ValueChanged<bool> onChanged;

  const _SwitchTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.value,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    return SwitchListTile(
      secondary: Icon(icon, color: AppColors.accent),
      title: Text(title, style: const TextStyle(fontSize: 14)),
      subtitle: Text(subtitle, style: const TextStyle(fontSize: 12)),
      value: value,
      onChanged: onChanged,
      activeTrackColor: AppColors.accent.withValues(alpha: 0.3),
      activeThumbColor: AppColors.accent,
    );
  }
}

class _InfoTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;

  const _InfoTile({
    required this.icon,
    required this.title,
    required this.subtitle,
  });

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: Icon(icon, color: AppColors.accent),
      title: Text(title, style: const TextStyle(fontSize: 14)),
      subtitle: Text(subtitle, style: const TextStyle(fontSize: 12)),
    );
  }
}

class _Divider extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return Divider(
      height: 1,
      thickness: 1,
      indent: 16,
      endIndent: 16,
      color: isDark ? AppColors.darkBorder : AppColors.lightBorder,
    );
  }
}
