import 'dart:async';
import 'package:camera/camera.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../models/camera_state.dart' as app_state;
import '../../providers/providers.dart';
import '../../providers/device_info_provider.dart';
import '../../shared/theme/app_theme.dart';
import '../../shared/widgets/capture_button.dart';
import '../../shared/widgets/processing_overlay.dart';
import '../../shared/widgets/permission_gate.dart';
import 'camera_channel.dart';
import 'camera_preview_widget.dart';
import 'capture_coordinator.dart';

class CameraScreen extends ConsumerStatefulWidget {
  const CameraScreen({super.key});
  @override
  ConsumerState<CameraScreen> createState() => _CameraScreenState();
}

class _CameraScreenState extends ConsumerState<CameraScreen>
    with WidgetsBindingObserver {
  bool _isSwitching = false;
  CameraMode _mode = CameraMode.photo;
  bool _showProControls = false;
  Offset? _focusPoint;
  Timer? _focusTimer;
  double _baseZoom = 1.0;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _focusTimer?.cancel();
    CameraChannel().dispose();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final appMode = ref.watch(cameraStateProvider);
    final settings = ref.watch(settingsProvider);
    final device = ref.watch(deviceInfoProvider).valueOrNull;
    final ch = CameraChannel();

    return PermissionGate(
      child: Scaffold(
        backgroundColor: Colors.black,
        body: Stack(
          fit: StackFit.expand,
          children: [
            // ── Preview ────────────────────────────────────────────────────
            GestureDetector(
              onTapUp: (d) => _onTapFocus(d, context),
              onScaleStart: (_) => _baseZoom = ch.currentZoom,
              onScaleUpdate: _onPinchZoom,
              child: CameraPreviewWidget(key: ValueKey(ch.isFront)),
            ),

            const ProcessingOverlay(),

            if (_isSwitching)
              const ColoredBox(
                color: Colors.black,
                child: Center(
                  child: CircularProgressIndicator(
                    color: PhotonixColors.accent,
                    strokeWidth: 2,
                  ),
                ),
              ),

            // ── Focus indicator ────────────────────────────────────────────
            if (_focusPoint != null) _FocusIndicator(position: _focusPoint!),

            // ── Top bar ────────────────────────────────────────────────────
            SafeArea(
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 10,
                ),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    _FlashButton(
                      mode: ch.flashMode,
                      onTap: () async {
                        await ch.cycleFlash();
                        setState(() {});
                      },
                    ),
                    _ZoomBadge(zoom: ch.currentZoom),
                    _QualityBadge(
                      tier: settings
                          .qualityTier, // this is already app_state.QualityTier
                      deviceTier: device?.tier,
                    ),
                  ],
                ),
              ),
            ),

            // ── Pro EV slider ──────────────────────────────────────────────
            if (_mode == CameraMode.pro && _showProControls)
              _ProControls(channel: ch, onChanged: () => setState(() {})),

            // ── Zoom bar ───────────────────────────────────────────────────
            Positioned(
              bottom: 180,
              left: 0,
              right: 0,
              child: _ZoomBar(
                channel: ch,
                onZoom: (z) async {
                  await ch.setZoom(z);
                  setState(() {});
                },
              ),
            ),

            // ── Mode selector ──────────────────────────────────────────────
            Positioned(
              bottom: 128,
              left: 0,
              right: 0,
              child: _ModeSelector(
                current: _mode,
                onSelect: (m) {
                  setState(() {
                    _mode = m;
                    _showProControls = m == CameraMode.pro;
                  });
                  CameraChannel().setCameraMode(m);
                },
              ),
            ),

            // ── Bottom controls ────────────────────────────────────────────
            Align(
              alignment: Alignment.bottomCenter,
              child: SafeArea(
                child: Padding(
                  padding: const EdgeInsets.only(bottom: 32),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                    crossAxisAlignment: CrossAxisAlignment.center,
                    children: [
                      Semantics(
                        label: 'Open gallery',
                        button: true,
                        child: GestureDetector(
                          onTap: () {},
                          child: Container(
                            width: 52,
                            height: 52,
                            decoration: BoxDecoration(
                              color: Colors.white12,
                              borderRadius: BorderRadius.circular(8),
                              border: Border.all(color: Colors.white24),
                            ),
                            child: const Icon(
                              Icons.photo_library_outlined,
                              color: Colors.white,
                              size: 24,
                            ),
                          ),
                        ),
                      ),

                      Semantics(
                        label: 'Capture photo',
                        button: true,
                        enabled:
                            appMode == app_state.CameraMode.idle &&
                            !_isSwitching,
                        child: CaptureButton(
                          onPressed:
                              (appMode == app_state.CameraMode.idle &&
                                  !_isSwitching)
                              ? _onShutter
                              : null,
                          isPortrait: _mode == CameraMode.portrait,
                        ),
                      ),

                      Semantics(
                        label: 'Switch camera',
                        button: true,
                        child: GestureDetector(
                          onTap:
                              (appMode == app_state.CameraMode.idle &&
                                  !_isSwitching)
                              ? _onFlip
                              : null,
                          child: Container(
                            width: 52,
                            height: 52,
                            decoration: BoxDecoration(
                              color: Colors.white12,
                              shape: BoxShape.circle,
                              border: Border.all(color: Colors.white24),
                            ),
                            child: Icon(
                              ch.isFront
                                  ? Icons.camera_front_outlined
                                  : Icons.camera_rear_outlined,
                              color: Colors.white,
                              size: 24,
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),

            Semantics(
              liveRegion: true,
              label: _a11y(appMode),
              child: const SizedBox.shrink(),
            ),

            if (settings.debugModeEnabled)
              _DebugOverlay(
                appMode: appMode,
                device: device,
                zoom: ch.currentZoom,
                cameraMode: _mode,
              ),
          ],
        ),
      ),
    );
  }

  Future<void> _onTapFocus(TapUpDetails d, BuildContext ctx) async {
    final size = MediaQuery.of(ctx).size;
    final norm = Offset(
      d.localPosition.dx / size.width,
      d.localPosition.dy / size.height,
    );
    setState(() => _focusPoint = d.localPosition);
    _focusTimer?.cancel();
    _focusTimer = Timer(const Duration(seconds: 2), () {
      if (mounted) setState(() => _focusPoint = null);
    });
    await CameraChannel().setFocusPoint(norm);
    HapticFeedback.selectionClick();
  }

  Future<void> _onPinchZoom(ScaleUpdateDetails d) async {
    final ch = CameraChannel();
    final zoom = (_baseZoom * d.scale).clamp(ch.minZoom, ch.maxZoom);
    await ch.setZoom(zoom);
    setState(() {});
  }

  void _onShutter() {
    HapticFeedback.mediumImpact();
    ref.read(captureCoordinatorProvider).capture();
  }

  Future<void> _onFlip() async {
    setState(() => _isSwitching = true);
    HapticFeedback.selectionClick();
    try {
      await CameraChannel().switchCamera();
      if (mounted) setState(() {});
    } catch (e) {
      debugPrint('[Camera] Flip: $e');
    } finally {
      if (mounted) setState(() => _isSwitching = false);
    }
  }

  String _a11y(app_state.CameraMode mode) => switch (mode) {
    app_state.CameraMode.idle => 'Camera ready',
    app_state.CameraMode.capturing => 'Capturing',
    app_state.CameraMode.processing => 'Processing',
    app_state.CameraMode.done => 'Done',
  };
}

// ── Widgets ───────────────────────────────────────────────────────────────────

class _FocusIndicator extends StatefulWidget {
  final Offset position;
  const _FocusIndicator({required this.position});
  @override
  State<_FocusIndicator> createState() => _FocusIndicatorState();
}

class _FocusIndicatorState extends State<_FocusIndicator>
    with SingleTickerProviderStateMixin {
  late AnimationController _ctrl;
  late Animation<double> _scale;

  @override
  void initState() {
    super.initState();
    _ctrl = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 250),
    );
    _scale = Tween(
      begin: 1.6,
      end: 1.0,
    ).animate(CurvedAnimation(parent: _ctrl, curve: Curves.easeOut));
    _ctrl.forward();
  }

  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Positioned(
    left: widget.position.dx - 30,
    top: widget.position.dy - 30,
    child: ScaleTransition(
      scale: _scale,
      child: Container(
        width: 60,
        height: 60,
        decoration: BoxDecoration(
          border: Border.all(color: PhotonixColors.accent, width: 1.5),
        ),
      ),
    ),
  );
}

class _FlashButton extends StatelessWidget {
  final FlashMode mode;
  final VoidCallback onTap;
  const _FlashButton({required this.mode, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final (icon, color) = switch (mode) {
      FlashMode.auto => (Icons.flash_auto, Colors.white),
      FlashMode.always => (Icons.flash_on, Colors.amber),
      FlashMode.off => (Icons.flash_off, Colors.white54),
      FlashMode.torch => (Icons.highlight, Colors.amber),
    };
    return GestureDetector(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: Icon(icon, color: color, size: 24),
      ),
    );
  }
}

class _ZoomBadge extends StatelessWidget {
  final double zoom;
  const _ZoomBadge({required this.zoom});
  @override
  Widget build(BuildContext context) {
    if (zoom <= 1.05) return const SizedBox.shrink();
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: Colors.black54,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        '${zoom.toStringAsFixed(1)}×',
        style: const TextStyle(
          color: Colors.white,
          fontSize: 12,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

class _ZoomBar extends StatelessWidget {
  final CameraChannel channel;
  final ValueChanged<double> onZoom;
  const _ZoomBar({required this.channel, required this.onZoom});

  @override
  Widget build(BuildContext context) {
    final List<double> presets = [
      channel.minZoom,
      1.0.clamp(channel.minZoom, channel.maxZoom),
      2.0.clamp(channel.minZoom, channel.maxZoom),
    ].toSet().toList()..sort();

    if (presets.length < 2) return const SizedBox.shrink();

    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: presets.map((z) {
        final isActive = (channel.currentZoom - z).abs() < 0.3;
        final label = z <= 1.05
            ? '1×'
            : z <= 2.1
            ? '2×'
            : '${z.toStringAsFixed(0)}×';
        return GestureDetector(
          onTap: () => onZoom(z),
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 200),
            margin: const EdgeInsets.symmetric(horizontal: 6),
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 5),
            decoration: BoxDecoration(
              color: isActive ? Colors.black87 : Colors.black45,
              borderRadius: BorderRadius.circular(20),
              border: Border.all(
                color: isActive ? Colors.white : Colors.white24,
              ),
            ),
            child: Text(
              label,
              style: TextStyle(
                color: isActive ? Colors.white : Colors.white60,
                fontSize: 12,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        );
      }).toList(),
    );
  }
}

class _ModeSelector extends StatelessWidget {
  final CameraMode current;
  final ValueChanged<CameraMode> onSelect;
  const _ModeSelector({required this.current, required this.onSelect});

  @override
  Widget build(BuildContext context) {
    // List of (mode, label) pairs — no record destructuring for compatibility
    final modes = <MapEntry<CameraMode, String>>[
      MapEntry(CameraMode.macro, 'MACRO'),
      MapEntry(CameraMode.portrait, 'PORTRAIT'),
      MapEntry(CameraMode.photo, 'PHOTO'),
      MapEntry(CameraMode.pro, 'PRO'),
      MapEntry(CameraMode.video, 'VIDEO'),
    ];

    return SizedBox(
      height: 30,
      child: ListView(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 16),
        children: modes.map((entry) {
          final isActive = current == entry.key;
          return GestureDetector(
            onTap: () => onSelect(entry.key),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 10),
              child: Text(
                entry.value,
                style: TextStyle(
                  color: isActive ? PhotonixColors.accent : Colors.white54,
                  fontSize: 12,
                  fontWeight: isActive ? FontWeight.w700 : FontWeight.w400,
                  letterSpacing: 0.8,
                ),
              ),
            ),
          );
        }).toList(),
      ),
    );
  }
}

class _ProControls extends StatelessWidget {
  final CameraChannel channel;
  final VoidCallback onChanged;
  const _ProControls({required this.channel, required this.onChanged});

  @override
  Widget build(BuildContext context) => Positioned(
    right: 16,
    top: 80,
    bottom: 200,
    child: Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        const Text('EV', style: TextStyle(color: Colors.white54, fontSize: 10)),
        Expanded(
          child: RotatedBox(
            quarterTurns: 3,
            child: Slider(
              value: channel.currentExposure,
              min: channel.minExposure,
              max: channel.maxExposure,
              divisions: 16,
              activeColor: PhotonixColors.accent,
              inactiveColor: Colors.white24,
              onChanged: (v) async {
                await channel.setExposure(v);
                onChanged();
              },
            ),
          ),
        ),
        Text(
          '${channel.currentExposure >= 0 ? "+" : ""}${channel.currentExposure.toStringAsFixed(1)}',
          style: const TextStyle(color: Colors.white, fontSize: 11),
        ),
      ],
    ),
  );
}

class _QualityBadge extends StatelessWidget {
  final app_state.QualityTier tier;
  final DeviceTier? deviceTier;
  const _QualityBadge({required this.tier, this.deviceTier});

  @override
  Widget build(BuildContext context) {
    String label;
    Color color;
    if (tier == app_state.QualityTier.aiEnhanced) {
      label = 'AI';
      color = PhotonixColors.accent;
    } else if (tier == app_state.QualityTier.standard) {
      label = 'STD';
      color = PhotonixColors.textSecondary;
    } else {
      label = 'FAST';
      color = PhotonixColors.textTertiary;
    }
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withOpacity(0.4)),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontSize: 10,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.8,
        ),
      ),
    );
  }
}

class _DebugOverlay extends StatelessWidget {
  final app_state.CameraMode appMode;
  final DeviceInfo? device;
  final double zoom;
  final CameraMode cameraMode;
  const _DebugOverlay({
    required this.appMode,
    this.device,
    required this.zoom,
    required this.cameraMode,
  });

  @override
  Widget build(BuildContext context) => Positioned(
    bottom: 105,
    left: 0,
    right: 0,
    child: Center(
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
        color: Colors.black54,
        child: Text(
          '${appMode.name.toUpperCase()} ${cameraMode.name.toUpperCase()} '
          '${zoom.toStringAsFixed(1)}x RAM:${device?.ramMb ?? "?"}',
          style: const TextStyle(
            color: PhotonixColors.accent,
            fontSize: 9,
            fontFamily: 'monospace',
          ),
        ),
      ),
    ),
  );
}
