import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../providers/pairing_provider.dart';
import '../theme/app_theme.dart';
import '../widgets/pair_code_display.dart';
import '../widgets/qr_display.dart';

class PairDeviceScreen extends StatefulWidget {
  const PairDeviceScreen({super.key});

  @override
  State<PairDeviceScreen> createState() => _PairDeviceScreenState();
}

class _PairDeviceScreenState extends State<PairDeviceScreen>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final subColor = isDark ? AppColors.textSecondary : AppColors.textLightSecondary;

    return Consumer<PairingProvider>(
      builder: (_, provider, __) {
        return SingleChildScrollView(
          padding: const EdgeInsets.all(32),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'Pair Device',
                style: Theme.of(context).textTheme.headlineMedium,
              ),
              const SizedBox(height: 6),
              Text(
                'Connect with another device securely',
                style: TextStyle(color: subColor, fontSize: 14),
              ),
              const SizedBox(height: 24),

              // Tab bar
              Container(
                decoration: BoxDecoration(
                  color: isDark ? AppColors.darkCard : AppColors.lightBg,
                  borderRadius: BorderRadius.circular(12),
                ),
                child: TabBar(
                  controller: _tabController,
                  indicator: BoxDecoration(
                    color: AppColors.accent,
                    borderRadius: BorderRadius.circular(10),
                  ),
                  indicatorSize: TabBarIndicatorSize.tab,
                  labelColor: Colors.white,
                  unselectedLabelColor: subColor,
                  dividerColor: Colors.transparent,
                  splashFactory: NoSplash.splashFactory,
                  padding: const EdgeInsets.all(4),
                  tabs: const [
                    Tab(text: 'QR Code'),
                    Tab(text: 'Pair Code'),
                  ],
                ),
              ),
              const SizedBox(height: 32),

              // Tab content
              SizedBox(
                height: 400,
                child: TabBarView(
                  controller: _tabController,
                  children: [
                    _QrTab(provider: provider),
                    _CodeTab(provider: provider),
                  ],
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}

// ── QR Tab ────────────────────────────────────────────────────────────

class _QrTab extends StatelessWidget {
  final PairingProvider provider;

  const _QrTab({required this.provider});

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final subColor = isDark ? AppColors.textSecondary : AppColors.textLightSecondary;

    if (provider.flowState == PairingFlowState.idle) {
      // Show tablet explaining QR pairing
      return Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const SizedBox(height: 32),
          Text(
            'Show this code to the device you want to pair with.',
            textAlign: TextAlign.center,
            style: TextStyle(color: subColor, fontSize: 14),
          ),
          const SizedBox(height: 24),
          ElevatedButton.icon(
            onPressed: () => provider.generateSession(),
            icon: const Icon(Icons.qr_code_rounded),
            label: const Text('Show QR Code'),
          ),
        ],
      );
    }

    if (provider.currentSession == null) {
      return Center(
        child: Column(
          children: [
            const CircularProgressIndicator(color: AppColors.accent),
            const SizedBox(height: 16),
            Text('Preparing pairing session...',
                style: TextStyle(color: subColor)),
          ],
        ),
      );
    }

    final session = provider.currentSession!;

    return Column(
      children: [
        // QR Code
        QrDisplay(data: session.qrData),
        const SizedBox(height: 20),
        Text(
          'Scan with another device',
          style: TextStyle(color: subColor, fontSize: 13),
        ),
        const SizedBox(height: 6),
        Text(
          'Expires in ${session.expiresIn}s',
          style: TextStyle(
            color: AppColors.orange,
            fontSize: 12,
            fontWeight: FontWeight.w500,
          ),
        ),
        const SizedBox(height: 20),
        TextButton.icon(
          onPressed: () => provider.generateSession(),
          icon: const Icon(Icons.refresh, size: 16),
          label: const Text('Refresh Code'),
        ),
      ],
    );
  }
}

// ── Code Tab ──────────────────────────────────────────────────────────

class _CodeTab extends StatelessWidget {
  final PairingProvider provider;

  const _CodeTab({required this.provider});

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final subColor = isDark ? AppColors.textSecondary : AppColors.textLightSecondary;

    if (provider.flowState == PairingFlowState.idle) {
      return Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const SizedBox(height: 32),
          Text(
            'Generate a 6-digit code to enter on another device.',
            textAlign: TextAlign.center,
            style: TextStyle(color: subColor, fontSize: 14),
          ),
          const SizedBox(height: 24),
          ElevatedButton.icon(
            onPressed: () => provider.generateSession(),
            icon: const Icon(Icons.pin_rounded),
            label: const Text('Generate Code'),
          ),
        ],
      );
    }

    if (provider.currentSession == null) {
      return Center(
        child: Column(
          children: [
            const CircularProgressIndicator(color: AppColors.accent),
            const SizedBox(height: 16),
            Text('Generating code...', style: TextStyle(color: subColor)),
          ],
        ),
      );
    }

    final session = provider.currentSession!;

    return Column(
      children: [
        Text(
          'Enter this code on the other device:',
          style: TextStyle(color: subColor, fontSize: 13),
        ),
        const SizedBox(height: 20),
        PairCodeDisplay(
          code: session.pairCode,
          onRefresh: () => provider.generateSession(),
        ),
        const SizedBox(height: 16),
        Text(
          'Expires in ${session.expiresIn}s',
          style: TextStyle(
            color: AppColors.orange,
            fontSize: 12,
            fontWeight: FontWeight.w500,
          ),
        ),
        const SizedBox(height: 20),
      ],
    );
  }
}
