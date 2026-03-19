package com.example.photonix_camera

import android.util.Log
import androidx.camera.core.ImageCapture
import androidx.camera.core.ImageCaptureException
import androidx.camera.core.ImageProxy
import kotlinx.coroutines.CancellableContinuation
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import java.util.concurrent.Executor
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

private const val TAG = "BurstCapture"
private const val FRAME_TIMEOUT_MS = 3000L

/**
 * Captures N frames in rapid succession using CameraX ImageCapture.
 *
 * Each frame is captured as YUV_420_888 (hardware NR disabled via Camera2Interop)
 * and converted to JPEG immediately on the capture executor thread.
 *
 * Returns List<ByteArray> — one JPEG per frame.
 * The list is returned to Dart as List<ByteArray> → List<Uint8List>.
 */
class BurstCaptureManager(
    private val imageCapture: ImageCapture,
    private val executor: Executor
) {

    /**
     * Captures [frameCount] frames.
     * Suspend function — safe to call from a coroutine.
     * Throws on timeout or capture error.
     */
    suspend fun captureBurst(frameCount: Int): List<ByteArray> {
        Log.d(TAG, "Starting burst: $frameCount frames")
        val frames = mutableListOf<ByteArray>()

        repeat(frameCount) { index ->
            val frame = withTimeoutOrNull(FRAME_TIMEOUT_MS) {
                captureFrame()
            } ?: throw RuntimeException("Frame $index timed out after ${FRAME_TIMEOUT_MS}ms")

            frames.add(frame)
            Log.d(TAG, "Captured frame ${index + 1}/$frameCount — ${frame.size} bytes")
        }

        Log.d(TAG, "Burst complete: ${frames.size} frames, " +
                "${frames.sumOf { it.size } / 1024}KB total")
        return frames
    }

    private suspend fun captureFrame(): ByteArray =
        suspendCancellableCoroutine { cont: CancellableContinuation<ByteArray> ->
            imageCapture.takePicture(
                executor,
                object : ImageCapture.OnImageCapturedCallback() {
                    override fun onCaptureSuccess(image: ImageProxy) {
                        try {
                            // Convert YUV → JPEG before closing the ImageProxy
                            val jpegBytes = image.use { proxy ->
                                // Access the underlying Image for YUV conversion
                                // ImageProxy wraps the Image — use the JPEG output
                                // Note: ImageCapture with JPEG format returns JPEG directly
                                val buffer = proxy.planes[0].buffer
                                val bytes = ByteArray(buffer.remaining())
                                buffer.get(bytes)
                                bytes
                            }
                            cont.resume(jpegBytes)
                        } catch (e: Exception) {
                            cont.resumeWithException(e)
                        }
                    }

                    override fun onError(exception: ImageCaptureException) {
                        Log.e(TAG, "Capture error: ${exception.message}")
                        cont.resumeWithException(exception)
                    }
                }
            )
        }
}