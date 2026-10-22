import 'package:flutter/material.dart';

import '../models/peer.dart';
import '../theme/app_theme.dart';
import 'glass_container.dart';

class PeerCard extends StatelessWidget {
  final Peer peer;
  final bool isTrusted;
  final VoidCallback? onPair;

  const PeerCard({
    super.key,
    required this.peer,
    this.isTrusted = false,
    this.onPair,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final textColor = isDark ? AppColors.textPrimary : AppColors.textLightPrimary;
    final subColor = isDark ? AppColors.textSecondary : AppColors.textLightSecondary;

    return GlassContainer(
      padding: const EdgeInsets.all(16),
      margin: const EdgeInsets.only(bottom: 12),
      child: Row(
        children: [
          // Platform icon
          Container(
            width: 44,
            height: 44,
            decoration: BoxDecoration(
              color: isDark ? AppColors.darkCard : AppColors.lightBg,
              borderRadius: BorderRadius.circular(12),
            ),
            child: Center(
              child: Text(
                peer.platformIcon,
                style: const TextStyle(fontSize: 20),
              ),
            ),
          ),
          const SizedBox(width: 14),
          // Peer info
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  peer.name,
                  style: TextStyle(
                    fontSize: 15,
                    fontWeight: FontWeight.w600,
                    color: textColor,
                  ),
                ),
                const SizedBox(height: 3),
                Row(
                  children: [
                    Container(
                      width: 8,
                      height: 8,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: peer.isOnline ? AppColors.green : AppColors.red,
                      ),
                    ),
                    const SizedBox(width: 6),
                    Text(
                      peer.platform,
                      style: TextStyle(fontSize: 12, color: subColor),
                    ),
                  ],
                ),
              ],
            ),
          ),
          // Pair button
          if (!isTrusted && onPair != null)
            ElevatedButton(
              onPressed: onPair,
              style: ElevatedButton.styleFrom(
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                minimumSize: Size.zero,
              ),
              child: const Text('Pair', style: TextStyle(fontSize: 13)),
            ),
          if (isTrusted)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
              decoration: BoxDecoration(
                color: AppColors.green.withValues(alpha: 0.15),
                borderRadius: BorderRadius.circular(8),
              ),
              child: const Text(
                'Trusted',
                style: TextStyle(fontSize: 12, color: AppColors.green),
              ),
            ),
        ],
      ),
    );
  }
}
