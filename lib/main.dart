import 'package:flutter/material.dart';
import 'src/rust/frb_generated.dart';
import 'src/features/debug/bridge_test_screen.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const PhotonixApp());
}

class PhotonixApp extends StatelessWidget {
  const PhotonixApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Photonix Camera',
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark(),
      home: const BridgeTestScreen(),
    );
  }
}
