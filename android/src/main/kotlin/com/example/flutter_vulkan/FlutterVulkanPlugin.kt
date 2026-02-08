package com.example.flutter_vulkan

import android.os.Handler
import android.os.Looper
import android.view.Surface
import android.graphics.SurfaceTexture
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugin.common.MethodChannel.MethodCallHandler
import io.flutter.view.TextureRegistry

class FlutterVulkanPlugin : FlutterPlugin, MethodCallHandler {
    private lateinit var channel: MethodChannel
    private var textureEntry: TextureRegistry.SurfaceTextureEntry? = null
    private var surface: Surface? = null
    private var textureRegistry: TextureRegistry? = null

    companion object {
        init {
            System.loadLibrary("flutter_vulkan_plugin")
        }
    }

    override fun onAttachedToEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        channel = MethodChannel(binding.binaryMessenger, "flutter_vulkan_plugin")
        channel.setMethodCallHandler(this)
        textureRegistry = binding.textureRegistry
    }

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "createSurface" -> {
                val width = call.argument<Int>("width")
                val height = call.argument<Int>("height")
                if (width == null || height == null || width <= 0 || height <= 0) {
                    result.error("100", "createSurface() called without valid width and height parameters!", null)
                    return
                }

                // Clean up previous surface
                if (textureEntry != null) {
                    nativeDestroySurface()
                    surface?.release()
                    textureEntry?.release()
                }

                val entry = textureRegistry!!.createSurfaceTexture()
                textureEntry = entry

                val surfaceTexture: SurfaceTexture = entry.surfaceTexture()
                surfaceTexture.setDefaultBufferSize(width, height)
                val newSurface = Surface(surfaceTexture)
                surface = newSurface

                val textureId = entry.id()
                val mainHandler = Handler(Looper.getMainLooper())
                Thread {
                    nativeCreateSurface(newSurface, width, height)
                    mainHandler.post { result.success(textureId) }
                }.start()
            }
            else -> result.notImplemented()
        }
    }

    override fun onDetachedFromEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        channel.setMethodCallHandler(null)
        nativeDestroySurface()
        surface?.release()
        textureEntry?.release()
        surface = null
        textureEntry = null
        textureRegistry = null
    }

    private external fun nativeCreateSurface(surface: Surface, width: Int, height: Int)
    private external fun nativeDestroySurface()
}
