package com.example.photonix_camera

import android.content.Context
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraManager
import android.util.Log
import androidx.camera.camera2.interop.Camera2CameraInfo
import androidx.camera.camera2.interop.ExperimentalCamera2Interop
import androidx.camera.core.CameraControl
import androidx.camera.core.ExposureState
import androidx.lifecycle.LifecycleOwner
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

private const val TAG = "CameraControlBridge"
private const val CHANNEL = "com.photonix/camera"

/**
 * Flutter MethodChannel bridge — exposes all camera operations to Dart.
 *
 * Channel: "com.photonix/camera"
 *
 * Methods:
 *   initCamera()                       → void
 *   captureBurst(frameCount: Int)      → List<ByteArray>
 *   getCapabilities()                  → Map<String, Any>
 *   setExposureCompensation(Int)       → void
 *   flipCamera()                       → void
 *   dispose()                          → void
 */
@OptIn(ExperimentalCamera2Interop::class)
class CameraControlBridge(
    private val context: Context,
    private val lifecycleOwner: LifecycleOwner,
    flutterEngine: FlutterEngine
) : MethodChannel.MethodCallHandler {

    private val channel = MethodChannel(
        flutterEngine.dartExecutor.binaryMessenger,
        CHANNEL
    )

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private var photonixCamera: PhotonixCamera? = null
    private var burstManager: BurstCaptureManager? = null

    init {
        channel.setMethodCallHandler(this)
        Log.d(TAG, "CameraControlBridge registered on channel: $CHANNEL")
    }

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "initCamera" -> initCamera(result)
            "captureBurst" -> {
                val frameCount = call.argument<Int>("frameCount") ?: 3
                captureBurst(frameCount, result)
            }
            "getCapabilities" -> getCapabilities(result)
            "setExposureCompensation" -> {
                val index = call.argument<Int>("index") ?: 0
                setExposureCompensation(index, result)
            }
            "flipCamera" -> {
                photonixCamera?.flipCamera()
                result.success(null)
            }
            "dispose" -> {
                photonixCamera?.shutdown()
                result.success(null)
            }
            else -> result.notImplemented()
        }
    }

    private fun initCamera(result: MethodChannel.Result) {
        if (photonixCamera != null) {
            result.success(null)
            return
        }

        photonixCamera = PhotonixCamera(context, lifecycleOwner).apply {
            startCamera { _ ->
                // Camera fully bound — NOW create burst manager and signal ready
                burstManager = BurstCaptureManager(context, imageCapture, cameraExecutor)
                Log.d(TAG, "Camera and burst manager ready")
                // Must call result.success on main thread
                android.os.Handler(android.os.Looper.getMainLooper()).post {
                    result.success(null)
                }
            }
        }
    }

    private fun captureBurst(frameCount: Int, result: MethodChannel.Result) {
        val manager = burstManager
        if (manager == null) {
            result.error("NOT_INITIALIZED", "Call initCamera first", null)
            return
        }

        scope.launch {
            try {
                Log.d(TAG, "captureBurst: $frameCount frames")
                val frames = manager.captureBurst(frameCount)

                // Return as List<ByteArray> — Flutter maps this to List<Uint8List>
                result.success(frames)

                Log.d(TAG, "captureBurst done: ${frames.size} frames returned to Dart")
            } catch (e: Exception) {
                Log.e(TAG, "captureBurst failed: ${e.message}")
                result.error("CAPTURE_FAILED", e.message, null)
            }
        }
    }

    private fun getCapabilities(result: MethodChannel.Result) {
        try {
            val cameraManager = context.getSystemService(Context.CAMERA_SERVICE) as CameraManager
            val cameraId = cameraManager.cameraIdList.firstOrNull() ?: run {
                result.error("NO_CAMERA", "No camera found", null)
                return
            }

            val chars = cameraManager.getCameraCharacteristics(cameraId)

            val minISO = chars.get(CameraCharacteristics.SENSOR_INFO_SENSITIVITY_RANGE)?.lower ?: 0
            val maxISO = chars.get(CameraCharacteristics.SENSOR_INFO_SENSITIVITY_RANGE)?.upper ?: 0
            val minExposure = chars.get(CameraCharacteristics.SENSOR_INFO_EXPOSURE_TIME_RANGE)?.lower ?: 0L
            val maxExposure = chars.get(CameraCharacteristics.SENSOR_INFO_EXPOSURE_TIME_RANGE)?.upper ?: 0L
            val focalLength = chars.get(CameraCharacteristics.LENS_INFO_AVAILABLE_FOCAL_LENGTHS)?.firstOrNull() ?: 0f
            val hasRaw = chars.get(CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES)
                ?.contains(CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES_RAW) ?: false

            val capabilities = mapOf(
                "minISO" to minISO,
                "maxISO" to maxISO,
                "minExposureNs" to minExposure,
                "maxExposureNs" to maxExposure,
                "focalLengthMm" to focalLength.toDouble(),
                "supportsRaw" to hasRaw,
                "cameraId" to cameraId
            )

            Log.d(TAG, "Capabilities: ISO $minISO-$maxISO, raw=$hasRaw")
            result.success(capabilities)

        } catch (e: Exception) {
            result.error("CAPABILITIES_FAILED", e.message, null)
        }
    }

    private fun setExposureCompensation(index: Int, result: MethodChannel.Result) {
        val camera = photonixCamera ?: run {
            result.error("NOT_INITIALIZED", "Camera not started", null)
            return
        }
        // Exposure compensation via CameraControl
        // Full manual ISO/shutter via Camera2Interop added in P9 optimization
        result.success(null)
    }
}