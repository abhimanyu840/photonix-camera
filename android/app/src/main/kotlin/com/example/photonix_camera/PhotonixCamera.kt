package com.example.photonix_camera

import android.content.Context
import android.hardware.camera2.CaptureRequest
import android.util.Log
import android.util.Size
import androidx.camera.camera2.interop.Camera2Interop
import androidx.camera.camera2.interop.ExperimentalCamera2Interop
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageCapture
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.core.content.ContextCompat
import androidx.lifecycle.LifecycleOwner
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

private const val TAG = "PhotonixCamera"

/**
 * Owns the CameraX session lifecycle.
 *
 * Critical settings applied via Camera2Interop:
 *   NOISE_REDUCTION_MODE = OFF  — Rust DnCNN handles denoising
 *   EDGE_MODE = OFF             — Rust sharpen handles edge enhancement
 *   COLOR_CORRECTION_MODE = FAST — keep AWB, disable aggressive tone mapping
 *
 * These are set on both Preview and ImageCapture use cases.
 */
@OptIn(ExperimentalCamera2Interop::class)
class PhotonixCamera(
    private val context: Context,
    private val lifecycleOwner: LifecycleOwner
) {

    lateinit var imageCapture: ImageCapture
        private set

    lateinit var previewView: PreviewView
        private set

    private var cameraProvider: ProcessCameraProvider? = null
    val cameraExecutor: ExecutorService = Executors.newSingleThreadExecutor()

    /** Current camera selector — back camera by default */
    private var cameraSelector = CameraSelector.DEFAULT_BACK_CAMERA

    /**
     * Initialises CameraX and binds use cases to the lifecycle.
     * Called once from CameraControlBridge after the Flutter texture is ready.
     */
    fun startCamera(onReady: (textureId: Long) -> Unit) {
        previewView = PreviewView(context).apply {
            implementationMode = PreviewView.ImplementationMode.COMPATIBLE
        }

        val cameraProviderFuture = ProcessCameraProvider.getInstance(context)
        cameraProviderFuture.addListener({
            cameraProvider = cameraProviderFuture.get()
            bindUseCases(onReady)
        }, ContextCompat.getMainExecutor(context))
    }

    private fun bindUseCases(onReady: (textureId: Long) -> Unit) {
        val provider = cameraProvider ?: return

        // ── Preview use case ─────────────────────────────────────────────────
        val previewBuilder = Preview.Builder()
            .setTargetResolution(Size(1920, 1080))

        // Disable hardware post-processing on preview too
        Camera2Interop.Extender(previewBuilder)
            .setCaptureRequestOption(
                CaptureRequest.NOISE_REDUCTION_MODE,
                CaptureRequest.NOISE_REDUCTION_MODE_OFF
            )
            .setCaptureRequestOption(
                CaptureRequest.EDGE_MODE,
                CaptureRequest.EDGE_MODE_OFF
            )

        val preview = previewBuilder.build().also {
            it.surfaceProvider = previewView.surfaceProvider
        }

        // ── ImageCapture use case ─────────────────────────────────────────────
        val imageCaptureBuilder = ImageCapture.Builder()
            .setCaptureMode(ImageCapture.CAPTURE_MODE_MINIMIZE_LATENCY)
            .setTargetResolution(Size(4032, 3024)) // 12MP — adjust per device
            .setJpegQuality(95)

        // CRITICAL: disable hardware NR and sharpening
        // Our Rust AI handles both — hardware processing corrupts the noise
        // distribution that DnCNN was trained to remove
        Camera2Interop.Extender(imageCaptureBuilder)
            .setCaptureRequestOption(
                CaptureRequest.NOISE_REDUCTION_MODE,
                CaptureRequest.NOISE_REDUCTION_MODE_OFF
            )
            .setCaptureRequestOption(
                CaptureRequest.EDGE_MODE,
                CaptureRequest.EDGE_MODE_OFF
            )
            .setCaptureRequestOption(
                CaptureRequest.COLOR_CORRECTION_MODE,
                CaptureRequest.COLOR_CORRECTION_MODE_FAST
            )
            // Keep auto-exposure and auto-white-balance for metering
            .setCaptureRequestOption(
                CaptureRequest.CONTROL_MODE,
                CaptureRequest.CONTROL_MODE_AUTO
            )

        imageCapture = imageCaptureBuilder.build()

        try {
            provider.unbindAll()
            val camera = provider.bindToLifecycle(
                lifecycleOwner,
                cameraSelector,
                preview,
                imageCapture
            )

            Log.d(TAG, "CameraX bound — NR=OFF, EDGE=OFF")

            // Return the Flutter texture ID for the PreviewView
            // In practice: Flutter's camera plugin manages the texture.
            // Here we signal readiness; the PreviewView is passed to the
            // AndroidView widget in Flutter.
            onReady(0L)

        } catch (e: Exception) {
            Log.e(TAG, "Failed to bind use cases: ${e.message}")
        }
    }

    fun flipCamera() {
        cameraSelector = if (cameraSelector == CameraSelector.DEFAULT_BACK_CAMERA)
            CameraSelector.DEFAULT_FRONT_CAMERA
        else
            CameraSelector.DEFAULT_BACK_CAMERA

        cameraProvider?.let { bindUseCases {} }
    }

    fun shutdown() {
        cameraExecutor.shutdown()
        cameraProvider?.unbindAll()
    }
}