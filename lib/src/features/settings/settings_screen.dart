import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../models/camera_state.dart';
import '../../providers/providers.dart';
import '../../shared/theme/app_theme.dart';

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settings = ref.watch(settingsProvider);
    final notifier = ref.read(settingsProvider.notifier);

    return Scaffold(
      backgroundColor: PhotonixColors.background,
      appBar: AppBar(title: const Text('Settings')),
      body: ListView(
        children: [
          // ── Quality tier ────────────────────────────────────────────────
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 20, 16, 8),
            child: Text('QUALITY',
                style: Theme.of(context).textTheme.labelSmall),
          ),
          ...QualityTier.values.map((tier) => RadioListTile<QualityTier>(
                title: Text(_tierLabel(tier)),
                subtitle: Text(_tierDescription(tier),
                    style: const TextStyle(
                        color: PhotonixColors.textSecondary, fontSize: 12)),
                value: tier,
                groupValue: settings.qualityTier,
                activeColor: PhotonixColors.accent,
                onChanged: (v) => v != null ? notifier.setQualityTier(v) : null,
              )),

          const Divider(height: 32),

          // ── Developer options ────────────────────────────────────────────
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
            child: Text('DEVELOPER',
                style: Theme.of(context).textTheme.labelSmall),
          ),
          SwitchListTile(
            title: const Text('Debug mode'),
            subtitle: const Text('Show pipeline state on camera screen',
                style: TextStyle(
                    color: PhotonixColors.textSecondary, fontSize: 12)),
            value: settings.debugModeEnabled,
            activeColor: PhotonixColors.accent,
            onChanged: (_) => notifier.toggleDebugMode(),
          ),
          SwitchListTile(
            title: const Text('Timing overlay'),
            subtitle: const Text('Show per-stage latency during processing',
                style: TextStyle(
                    color: PhotonixColors.textSecondary, fontSize: 12)),
            value: settings.showTimingOverlay,
            activeColor: PhotonixColors.accent,
            onChanged: (_) => notifier.toggleTimingOverlay(),
          ),

          const Divider(height: 32),

          // ── Bridge test shortcut ─────────────────────────────────────────
          ListTile(
            title: const Text('Bridge validation'),
            subtitle: const Text('Run P2 bridge tests',
                style: TextStyle(
                    color: PhotonixColors.textSecondary, fontSize: 12)),
            trailing: const Icon(Icons.arrow_forward_ios,
                size: 14, color: PhotonixColors.textTertiary),
            onTap: () => context.push('/debug/bridge'),
          ),

          const SizedBox(height: 40),
        ],
      ),
    );
  }

  String _tierLabel(QualityTier tier) => switch (tier) {
    QualityTier.aiEnhanced => 'AI Enhanced',
    QualityTier.standard   => 'Standard',
    QualityTier.fast       => 'Fast',
  };

  String _tierDescription(QualityTier tier) => switch (tier) {
    QualityTier.aiEnhanced =>
      'DnCNN + Real-ESRGAN + MiDaS — best quality, ~330ms',
    QualityTier.standard   =>
      'Burst stack + HDR + tone map — no AI, ~120ms',
    QualityTier.fast       =>
      'Single frame, minimal processing, ~30ms',
  };
}