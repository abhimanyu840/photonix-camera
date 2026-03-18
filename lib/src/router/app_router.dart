import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import '../features/camera/camera_screen.dart';
import '../features/gallery/gallery_screen.dart';
import '../features/photo_viewer/photo_viewer_screen.dart';
import '../features/settings/settings_screen.dart';
import '../features/debug/bridge_test_screen.dart';

/// All app routes.
/// Shell route wraps camera + gallery + settings in a bottom nav bar.
final appRouter = GoRouter(
  initialLocation: '/camera',
  debugLogDiagnostics: false,
  routes: [
    // ── Shell: bottom nav (camera / gallery / settings) ───────────────────
    ShellRoute(
      builder: (context, state, child) => _AppShell(child: child),
      routes: [
        GoRoute(
          path: '/camera',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: CameraScreen()),
        ),
        GoRoute(
          path: '/gallery',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: GalleryScreen()),
        ),
        GoRoute(
          path: '/settings',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: SettingsScreen()),
        ),
      ],
    ),

    // ── Full-screen routes (no bottom nav) ────────────────────────────────
    GoRoute(
      path: '/photo/:id',
      builder: (context, state) =>
          PhotoViewerScreen(photoId: state.pathParameters['id']!),
    ),
    GoRoute(
      path: '/debug/bridge',
      builder: (context, state) => const BridgeTestScreen(),
    ),
  ],
);

/// Bottom navigation shell — wraps the three main tabs.
class _AppShell extends StatelessWidget {
  final Widget child;
  const _AppShell({required this.child});

  @override
  Widget build(BuildContext context) {
    final location = GoRouterState.of(context).matchedLocation;

    return Scaffold(
      body: child,
      bottomNavigationBar: _BottomNav(currentLocation: location),
    );
  }
}

class _BottomNav extends StatelessWidget {
  final String currentLocation;
  const _BottomNav({required this.currentLocation});

  @override
  Widget build(BuildContext context) {
    final currentIndex = switch (currentLocation) {
      '/gallery' => 1,
      '/settings' => 2,
      _ => 0, // /camera is default
    };

    return NavigationBar(
      backgroundColor: const Color(0xFF0E0E0E),
      indicatorColor: const Color(0xFF1C1C1C),
      selectedIndex: currentIndex,
      onDestinationSelected: (i) {
        switch (i) {
          case 0:
            context.go('/camera');
          case 1:
            context.go('/gallery');
          case 2:
            context.go('/settings');
        }
      },
      destinations: const [
        NavigationDestination(
          icon: Icon(Icons.camera_outlined),
          selectedIcon: Icon(Icons.camera),
          label: 'Camera',
        ),
        NavigationDestination(
          icon: Icon(Icons.photo_library_outlined),
          selectedIcon: Icon(Icons.photo_library),
          label: 'Gallery',
        ),
        NavigationDestination(
          icon: Icon(Icons.tune_outlined),
          selectedIcon: Icon(Icons.tune),
          label: 'Settings',
        ),
      ],
    );
  }
}
