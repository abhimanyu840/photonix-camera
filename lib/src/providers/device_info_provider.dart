import 'dart:io';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../models/camera_state.dart';

/// Detected device capability tier based on available RAM.
enum DeviceTier {
  /// > 2GB RAM — all AI enabled
  high,

  /// 1-2GB RAM — Real-ESRGAN disabled, burst=3
  medium,

  /// < 1GB RAM — classical pipeline only
  low,
}

class DeviceInfo {
  final DeviceTier tier;
  final int ramMb;

  const DeviceInfo({required this.tier, required this.ramMb});

  /// Quality tier that matches device capability.
  QualityTier get maxQualityTier => switch (tier) {
    DeviceTier.high => QualityTier.aiEnhanced,
    DeviceTier.medium => QualityTier.standard,
    DeviceTier.low => QualityTier.fast,
  };

  String get tierLabel => switch (tier) {
    DeviceTier.high => 'AI Enhanced',
    DeviceTier.medium => 'Standard',
    DeviceTier.low => 'Basic',
  };
}

class DeviceInfoNotifier extends AsyncNotifier<DeviceInfo> {
  static const _channel = MethodChannel('com.photonix/device');

  @override
  Future<DeviceInfo> build() async {
    final ramMb = await _getRamMb();
    final tier = ramMb > 2048
        ? DeviceTier.high
        : ramMb > 1024
        ? DeviceTier.medium
        : DeviceTier.low;
    return DeviceInfo(tier: tier, ramMb: ramMb);
  }

  Future<int> _getRamMb() async {
    try {
      final result = await _channel.invokeMethod<int>('getAvailableRam');
      return (result ?? 2048) ~/ (1024 * 1024);
    } catch (_) {
      // Fallback: read /proc/meminfo on Android
      try {
        final f = File('/proc/meminfo');
        if (await f.exists()) {
          final lines = await f.readAsLines();
          for (final line in lines) {
            if (line.startsWith('MemTotal:')) {
              final kb = int.tryParse(
                line.split(RegExp(r'\s+')).elementAtOrNull(1) ?? '',
              );
              if (kb != null) return kb ~/ 1024;
            }
          }
        }
      } catch (_) {}
      return 2048; // assume medium device on failure
    }
  }
}

final deviceInfoProvider =
    AsyncNotifierProvider<DeviceInfoNotifier, DeviceInfo>(
      DeviceInfoNotifier.new,
    );
