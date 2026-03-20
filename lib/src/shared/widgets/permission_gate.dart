import 'package:flutter/material.dart';
import 'package:permission_handler/permission_handler.dart';
import '../theme/app_theme.dart';

/// Wraps any widget that requires camera permission.
/// Shows a clear recovery UI if permission is denied.
class PermissionGate extends StatefulWidget {
  final Widget child;
  const PermissionGate({super.key, required this.child});

  @override
  State<PermissionGate> createState() => _PermissionGateState();
}

class _PermissionGateState extends State<PermissionGate>
    with WidgetsBindingObserver {
  PermissionStatus _status = PermissionStatus.denied;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _check();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    // Re-check when user returns from Settings
    if (state == AppLifecycleState.resumed) _check();
  }

  Future<void> _check() async {
    final status = await Permission.camera.status;
    if (mounted) setState(() => _status = status);
  }

  Future<void> _request() async {
    final status = await Permission.camera.request();
    if (mounted) setState(() => _status = status);
  }

  @override
  Widget build(BuildContext context) {
    if (_status.isGranted) return widget.child;

    return Scaffold(
      backgroundColor: PhotonixColors.background,
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Icon(
                Icons.camera_alt_outlined,
                color: PhotonixColors.accent,
                size: 56,
              ),
              const SizedBox(height: 24),
              const Text(
                'Camera access needed',
                style: TextStyle(
                  color: PhotonixColors.textPrimary,
                  fontSize: 22,
                  fontWeight: FontWeight.w500,
                ),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 12),
              Text(
                _status.isPermanentlyDenied
                    ? 'Camera permission was permanently denied. Open Settings to enable it.'
                    : 'Photonix needs camera access to capture photos.',
                style: const TextStyle(
                  color: PhotonixColors.textSecondary,
                  fontSize: 15,
                ),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 32),
              SizedBox(
                width: double.infinity,
                child: ElevatedButton(
                  onPressed: _status.isPermanentlyDenied
                      ? openAppSettings
                      : _request,
                  child: Text(
                    _status.isPermanentlyDenied
                        ? 'Open Settings'
                        : 'Grant Permission',
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
