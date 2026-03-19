package com.example.photonix_camera

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.ImageFormat
import android.graphics.Rect
import android.graphics.YuvImage
import android.media.Image
import java.io.ByteArrayOutputStream

/**
 * Converts Android YUV_420_888 Image objects to JPEG byte arrays.
 *
 * Uses Android's hardware YuvImage encoder — faster than any software path.
 * Quality 95 balances file size with detail preservation for AI processing.
 *
 * Why not use ImageCapture.OutputFileOptions directly?
 * Because we need raw YUV access so hardware NR is disabled via Camera2Interop.
 * ImageCapture with file output applies post-processing we explicitly don't want.
 */
object YuvConverter {

    private const val JPEG_QUALITY = 95

    /**
     * Converts a YUV_420_888 Image to a JPEG byte array.
     * Called on a background thread — never on the main thread.
     */
    fun toJpeg(image: Image): ByteArray {
        require(image.format == ImageFormat.YUV_420_888) {
            "Expected YUV_420_888, got format: ${image.format}"
        }

        val yBuffer = image.planes[0].buffer
        val uBuffer = image.planes[1].buffer
        val vBuffer = image.planes[2].buffer

        val ySize = yBuffer.remaining()
        val uSize = uBuffer.remaining()
        val vSize = vBuffer.remaining()

        // NV21 layout: Y plane followed by interleaved V+U
        val nv21 = ByteArray(ySize + uSize + vSize)
        yBuffer.get(nv21, 0, ySize)
        vBuffer.get(nv21, ySize, vSize)
        uBuffer.get(nv21, ySize + vSize, uSize)

        val yuvImage = YuvImage(
            nv21,
            ImageFormat.NV21,
            image.width,
            image.height,
            null
        )

        val outputStream = ByteArrayOutputStream()
        yuvImage.compressToJpeg(
            Rect(0, 0, image.width, image.height),
            JPEG_QUALITY,
            outputStream
        )

        return outputStream.toByteArray()
    }

    /**
     * Returns the EXIF rotation degrees for a given sensor rotation
     * and whether the camera is front-facing.
     */
    fun getRotationDegrees(sensorRotation: Int, isFrontFacing: Boolean): Int {
        return if (isFrontFacing) (360 - sensorRotation) % 360
        else sensorRotation
    }
}