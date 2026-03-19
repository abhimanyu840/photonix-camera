package com.example.photonix_camera

import android.content.Context
import io.flutter.plugin.common.StandardMessageCodec
import io.flutter.plugin.platform.PlatformView
import io.flutter.plugin.platform.PlatformViewFactory
import androidx.camera.view.PreviewView

class CameraPreviewFactory(private val activity: MainActivity) :
    PlatformViewFactory(StandardMessageCodec.INSTANCE) {

    override fun create(context: Context, viewId: Int, args: Any?): PlatformView {
        return CameraPlatformView(activity)
    }
}

class CameraPlatformView(private val activity: MainActivity) : PlatformView {
    // Return the PreviewView from the PhotonixCamera instance
    // photonixCamera is held in CameraControlBridge
    private val previewView = PreviewView(activity)

    override fun getView() = previewView
    override fun dispose() {}
}