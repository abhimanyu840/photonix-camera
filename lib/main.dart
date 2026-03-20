import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'src/rust/frb_generated.dart';
import 'src/router/app_router.dart';
import 'src/shared/theme/app_theme.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  await SystemChrome.setPreferredOrientations([
    DeviceOrientation.portraitUp,
    DeviceOrientation.portraitDown,
  ]);

  SystemChrome.setSystemUIOverlayStyle(
    const SystemUiOverlayStyle(
      statusBarColor: Colors.transparent,
      systemNavigationBarColor: Colors.transparent,
    ),
  );

  // Initialize Rust bridge with timeout — ORT init can hang on some devices
  try {
    await RustLib.init().timeout(
      await rust_api.initPhotonixEngine();
      const Duration(seconds: 10),
      onTimeout: () {
        debugPrint('[Main] RustLib.init() timed out — continuing anyway');
      },
    );
  } catch (e) {
    debugPrint('[Main] RustLib.init() failed: $e — continuing anyway');
  }
  

  runApp(const ProviderScope(child: PhotonixApp()));
}

class PhotonixApp extends StatelessWidget {
  const PhotonixApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp.router(
      title: 'Photonix Camera',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.dark,
      routerConfig: appRouter,
    );
  }
}
