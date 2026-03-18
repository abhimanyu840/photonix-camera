import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../models/camera_state.dart';

class AppSettings {
  final QualityTier qualityTier;
  final bool debugModeEnabled;
  final bool showTimingOverlay;

  const AppSettings({
    this.qualityTier = QualityTier.aiEnhanced,
    this.debugModeEnabled = false,
    this.showTimingOverlay = false,
  });

  AppSettings copyWith({
    QualityTier? qualityTier,
    bool? debugModeEnabled,
    bool? showTimingOverlay,
  }) {
    return AppSettings(
      qualityTier: qualityTier ?? this.qualityTier,
      debugModeEnabled: debugModeEnabled ?? this.debugModeEnabled,
      showTimingOverlay: showTimingOverlay ?? this.showTimingOverlay,
    );
  }
}

class SettingsNotifier extends Notifier<AppSettings> {
  @override
  AppSettings build() => const AppSettings();

  void setQualityTier(QualityTier tier) {
    state = state.copyWith(qualityTier: tier);
  }

  void toggleDebugMode() {
    state = state.copyWith(debugModeEnabled: !state.debugModeEnabled);
  }

  void toggleTimingOverlay() {
    state = state.copyWith(showTimingOverlay: !state.showTimingOverlay);
  }
}

final settingsProvider = NotifierProvider<SettingsNotifier, AppSettings>(
  SettingsNotifier.new,
);
