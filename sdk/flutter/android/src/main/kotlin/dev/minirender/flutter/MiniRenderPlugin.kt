package dev.minirender.flutter

import android.app.ActivityManager
import android.content.Context
import android.os.Build
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import dev.minirender.MiniEngine
import java.io.File

/**
 * Flutter 插件入口：通道 `dev.minirender/engine`。
 *
 * 一个进程同一时刻只有一个小程序实例（引擎的小程序根目录、数据目录、网络回包队列都是
 * 进程级的），所以这里只持有一个 [EngineHost]；Dart 侧在上一个 dispose 之前再建会收到
 * `ALREADY_RUNNING`。热重启后 Dart 先调 `init`，把旧实例清掉。
 */
class MiniRenderPlugin : FlutterPlugin, MethodChannel.MethodCallHandler {
    private lateinit var channel: MethodChannel
    private lateinit var binding: FlutterPlugin.FlutterPluginBinding
    private var host: EngineHost? = null

    override fun onAttachedToEngine(flutterPluginBinding: FlutterPlugin.FlutterPluginBinding) {
        binding = flutterPluginBinding
        channel = MethodChannel(flutterPluginBinding.binaryMessenger, "dev.minirender/engine")
        channel.setMethodCallHandler(this)
    }

    override fun onDetachedFromEngine(flutterPluginBinding: FlutterPlugin.FlutterPluginBinding) {
        channel.setMethodCallHandler(null)
        host?.dispose()
        host = null
    }

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "init", "dispose" -> {
                host?.dispose()
                host = null
                result.success(null)
            }
            "paths" -> {
                val ctx = binding.applicationContext
                result.success(
                    mapOf(
                        "files" to ctx.filesDir.absolutePath,
                        "cache" to ctx.cacheDir.absolutePath,
                        "external" to (ctx.getExternalFilesDir(null)?.absolutePath ?: ""),
                    )
                )
            }
            "create" -> create(call, result)
            else -> {
                val h = host
                if (h == null) {
                    // 还没建好 / 已经关了：命令直接丢掉（back 回 false）
                    result.success(if (call.method == "back") false else null)
                } else {
                    h.handle(call, result)
                }
            }
        }
    }

    private fun deviceInfo(): EngineHost.DeviceInfo {
        val am = binding.applicationContext.getSystemService(Context.ACTIVITY_SERVICE) as? ActivityManager
        val mem = ActivityManager.MemoryInfo().also { am?.getMemoryInfo(it) }
        return EngineHost.DeviceInfo(mem.totalMem, am?.isLowRamDevice ?: false, Build.VERSION.SDK_INT)
    }

    private fun create(call: MethodCall, result: MethodChannel.Result) {
        if (host != null) {
            result.error("ALREADY_RUNNING", "同一进程同时只能有一个小程序（先 dispose 上一个）", null)
            return
        }
        val appDir = call.argument<String>("appDir")
        if (appDir.isNullOrEmpty()) {
            result.error("BAD_ARGS", "appDir 不能为空", null)
            return
        }
        // 数据目录必须给：storage 与图片磁盘缓存都落在这里，不给会静默失效
        val dataDir = call.argument<String>("dataDir")
            ?: File(binding.applicationContext.filesDir, "mini-render").apply { mkdirs() }.absolutePath
        val h = EngineHost(
            binding.textureRegistry,
            onEvents = { channel.invokeMethod("events", it) },
            onTexture = { channel.invokeMethod("texture", it) },
        )
        host = h
        h.create(
            appDir = appDir,
            dataDir = dataDir,
            route = call.argument<String>("route"),
            width = (call.argument<Number>("width") ?: 1).toInt(),
            height = (call.argument<Number>("height") ?: 1).toInt(),
            dpr = (call.argument<Number>("dpr") ?: 1).toFloat(),
            options = call.argument<String>("options") ?: "{}",
            // 0 auto / 1 cpu / 2 gpu（MiniEngine.BACKEND_*）；默认 cpu，与原生 SDK 一致
            backend = (call.argument<Number>("backend") ?: MiniEngine.BACKEND_CPU).toInt(),
            device = deviceInfo(),
            result = result,
            onFailed = { if (host === h) host = null },
        )
    }
}
