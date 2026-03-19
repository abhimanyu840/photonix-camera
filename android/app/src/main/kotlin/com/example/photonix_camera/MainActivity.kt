package com.example.photonix_camera

import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.platform.PlatformViewRegistry

class MainActivity : FlutterActivity() {

    private var cameraControlBridge: CameraControlBridge? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
    super.configureFlutterEngine(flutterEngine)

    // Register camera MethodChannel
    cameraControlBridge = CameraControlBridge(
        context = applicationContext,
        lifecycleOwner = this,
        flutterEngine = flutterEngine
    )

    // Register native view factory for the camera preview
    flutterEngine.platformViewsController.registry
        .registerViewFactory(
            "com.photonix/preview",
            CameraPreviewFactory(this)
        )
}

    override fun onDestroy() {
        cameraControlBridge = null
        super.onDestroy()
    }
}