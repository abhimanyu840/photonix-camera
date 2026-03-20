package com.example.photonix_camera

import android.content.Context
import android.util.Log
import androidx.camera.core.ImageCapture
import androidx.camera.core.ImageCaptureException
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import java.io.File
import java.util.concurrent.Executor
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

private const val TAG = "BurstCapture"
private const val FRAME_TIMEOUT_MS = 5000L

class BurstCaptureManager(
    private val context: Context,
    private val imageCapture: ImageCapture,
    private val executor: Executor
) {
    suspend fun captureBurst(frameCount: Int): List<ByteArray> {
        Log.d(TAG, "Starting burst: $frameCount frames")
        val frames = mutableListOf<ByteArray>()

        repeat(frameCount) { index ->
            val frame = withTimeoutOrNull(FRAME_TIMEOUT_MS) {
                captureFrame(index)
            } ?: throw RuntimeException("Frame $index timed out after ${FRAME_TIMEOUT_MS}ms")

            frames.add(frame)
            Log.d(TAG, "Captured frame ${index + 1}/$frameCount — ${frame.size} bytes")
        }

        Log.d(TAG, "Burst complete: ${frames.size} frames")
        return frames
    }

    private suspend fun captureFrame(index: Int): ByteArray =
        suspendCancellableCoroutine { cont ->
            val tempFile = File.createTempFile("photonix_frame_$index", ".jpg", context.cacheDir)

            val outputOptions = ImageCapture.OutputFileOptions.Builder(tempFile).build()

            imageCapture.takePicture(
                outputOptions,
                executor,
                object : ImageCapture.OnImageSavedCallback {
                    override fun onImageSaved(output: ImageCapture.OutputFileResults) {
                        try {
                            val bytes = tempFile.readBytes()
                            tempFile.delete()
                            Log.d(TAG, "Frame $index saved: ${bytes.size} bytes")
                            cont.resume(bytes)
                        } catch (e: Exception) {
                            tempFile.delete()
                            cont.resumeWithException(e)
                        }
                    }

                    override fun onError(exception: ImageCaptureException) {
                        tempFile.delete()
                        Log.e(TAG, "Frame $index error: ${exception.message}")
                        cont.resumeWithException(exception)
                    }
                }
            )
        }
}