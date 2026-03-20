import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../models/camera_state.dart';
import '../../providers/providers.dart';
import '../../providers/device_info_provider.dart';
import '../../shared/theme/app_theme.dart';

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settings = ref.watch(settingsProvider);
    final notifier = ref.read(settingsProvider.notifier);
    final device = ref.watch(deviceInfoProvider).valueOrNull;

    return Scaffold(
      backgroundColor: PhotonixColors.background,
      appBar: AppBar(title: const Text('Settings')),
      body: ListView(
        children: [
          // ── Device capability notice ──────────────────────────────────────
          if (device != null && device.tier != DeviceTier.high)
            _Notice(
              message:
                  'Some features limited on this device '
                  '(${device.ramMb}MB RAM detected). '
                  'Running in ${device.tierLabel} mode.',
            ),

          // ── Quality tier ──────────────────────────────────────────────────
          _SectionHeader('QUALITY'),
          ...QualityTier.values.map((tier) {
            final isAvailable = _isTierAvailable(tier, device);
            return Semantics(
              label: '${_tierLabel(tier)}: ${_tierDescription(tier)}',
              child: RadioListTile<QualityTier>(
                title: Text(
                  _tierLabel(tier),
                  style: TextStyle(
                    color: isAvailable
                        ? PhotonixColors.textPrimary
                        : PhotonixColors.textTertiary,
                  ),
                ),
                subtitle: Text(
                  isAvailable
                      ? _tierDescription(tier)
                      : 'Not available on this device',
                  style: const TextStyle(
                    color: PhotonixColors.textSecondary,
                    fontSize: 12,
                  ),
                ),
                value: tier,
                groupValue: settings.qualityTier,
                activeColor: PhotonixColors.accent,
                onChanged: isAvailable
                    ? (v) => v != null ? notifier.setQualityTier(v) : null
                    : null,
              ),
            );
          }),

          const Divider(height: 32),

          // ── Developer options ─────────────────────────────────────────────
          _SectionHeader('DEVELOPER'),
          Semantics(
            label: 'Debug mode toggle',
            child: SwitchListTile(
              title: const Text('Debug mode'),
              subtitle: const Text(
                'Show pipeline state, device tier, timing on camera',
                style: TextStyle(
                  color: PhotonixColors.textSecondary,
                  fontSize: 12,
                ),
              ),
              value: settings.debugModeEnabled,
              activeColor: PhotonixColors.accent,
              onChanged: (_) => notifier.toggleDebugMode(),
            ),
          ),
          Semantics(
            label: 'Timing overlay toggle',
            child: SwitchListTile(
              title: const Text('Timing overlay'),
              subtitle: const Text(
                'Show per-stage latency during processing',
                style: TextStyle(
                  color: PhotonixColors.textSecondary,
                  fontSize: 12,
                ),
              ),
              value: settings.showTimingOverlay,
              activeColor: PhotonixColors.accent,
              onChanged: (_) => notifier.toggleTimingOverlay(),
            ),
          ),

          const Divider(height: 32),

          // ── About ─────────────────────────────────────────────────────────
          _SectionHeader('ABOUT'),
          ListTile(
            title: const Text('Engine version'),
            subtitle: const Text(
              'Photonix Engine v0.1.0',
              style: TextStyle(
                color: PhotonixColors.textSecondary,
                fontSize: 12,
              ),
            ),
            trailing: const Icon(
              Icons.info_outline,
              size: 16,
              color: PhotonixColors.textTertiary,
            ),
          ),
          ListTile(
            title: const Text('Bridge validation'),
            subtitle: const Text(
              'Run P2 bridge tests',
              style: TextStyle(
                color: PhotonixColors.textSecondary,
                fontSize: 12,
              ),
            ),
            trailing: const Icon(
              Icons.arrow_forward_ios,
              size: 14,
              color: PhotonixColors.textTertiary,
            ),
            onTap: () => context.push('/debug/bridge'),
          ),

          const SizedBox(height: 40),
        ],
      ),
    );
  }

  bool _isTierAvailable(QualityTier tier, DeviceInfo? device) {
    if (device == null) return true;
    return switch (tier) {
      QualityTier.aiEnhanced => device.tier == DeviceTier.high,
      QualityTier.standard => device.tier != DeviceTier.low,
      QualityTier.fast => true,
    };
  }

  String _tierLabel(QualityTier tier) => switch (tier) {
    QualityTier.aiEnhanced => 'AI Enhanced',
    QualityTier.standard => 'Standard',
    QualityTier.fast => 'Fast',
  };

  String _tierDescription(QualityTier tier) => switch (tier) {
    QualityTier.aiEnhanced =>
      'DnCNN + Real-ESRGAN + MiDaS — best quality, ~330ms',
    QualityTier.standard => 'Burst stack + HDR + tone map — no AI, ~120ms',
    QualityTier.fast => 'Single frame, minimal processing, ~30ms',
  };
}

class _SectionHeader extends StatelessWidget {
  final String text;
  const _SectionHeader(this.text);

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 20, 16, 8),
      child: Text(
        text,
        style: Theme.of(context).textTheme.labelSmall?.copyWith(
          color: PhotonixColors.textTertiary,
          letterSpacing: 0.8,
        ),
      ),
    );
  }
}

class _Notice extends StatelessWidget {
  final String message;
  const _Notice({required this.message});

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.all(16),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: PhotonixColors.accent.withOpacity(0.08),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: PhotonixColors.accent.withOpacity(0.3)),
      ),
      child: Row(
        children: [
          const Icon(
            Icons.info_outline,
            color: PhotonixColors.accent,
            size: 16,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              message,
              style: const TextStyle(
                color: PhotonixColors.textSecondary,
                fontSize: 12,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
