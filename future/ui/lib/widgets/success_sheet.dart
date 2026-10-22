import 'package:flutter/material.dart';

import '../theme/app_theme.dart';

/// Bottom sheet shown when pairing succeeds.
class PairingSuccessSheet extends StatelessWidget {
  final String? deviceName;
  final String? devicePlatform;
  final VoidCallback onDone;
  final VoidCallback onRename;

  const PairingSuccessSheet({
    super.key,
    this.deviceName,
    this.devicePlatform,
    required this.onDone,
    required this.onRename,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Container(
      padding: const EdgeInsets.fromLTRB(24, 24, 24, 32),
      decoration: BoxDecoration(
        color: isDark ? AppColors.darkSurface : Colors.white,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(24)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          // Drag handle
          Container(
            width: 40,
            height: 4,
            decoration: BoxDecoration(
              color: isDark ? AppColors.darkBorder : AppColors.lightBorder,
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          const SizedBox(height: 32),
          // Success icon
          AnimatedContainer(
            duration: const Duration(milliseconds: 500),
            width: 80,
            height: 80,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: AppColors.green.withValues(alpha: 0.15),
            ),
            child: const Icon(
              Icons.check_rounded,
              size: 40,
              color: AppColors.green,
            ),
          ),
          const SizedBox(height: 20),
          Text(
            'Device Paired Successfully',
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 8),
          if (deviceName != null)
            Text(
              '$deviceName${devicePlatform != null ? ' · $devicePlatform' : ''}',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
          const SizedBox(height: 32),
          SizedBox(
            width: double.infinity,
            child: ElevatedButton(
              onPressed: onDone,
              child: const Text('Done'),
            ),
          ),
          const SizedBox(height: 10),
          SizedBox(
            width: double.infinity,
            child: TextButton(
              onPressed: onRename,
              child: const Text('Rename Device'),
            ),
          ),
        ],
      ),
    );
  }
}
