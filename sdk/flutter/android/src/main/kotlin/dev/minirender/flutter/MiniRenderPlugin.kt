package dev.minirender.flutter

import android.app.ActivityManager
import android.content.Context
import android.os.Build
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.embedding.engine.plugins.activity.ActivityAware
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import dev.minirender.MiniEngine
import java.io.File
import org.json.JSONObject

/**
 * Flutter 插件入口：通道 `dev.minirender/engine`。
 *
 * 一个进程同一时刻只有一个小程序实例（引擎的小程序根目录、数据目录、网络回包队列都是
 * 进程级的），所以这里只持有一个 [EngineHost]；Dart 侧在上一个 dispose 之前再建会收到
 * `ALREADY_RUNNING`。热重启后 Dart 先调 `init`，把旧实例清掉。
 */
class MiniRenderPlugin : FlutterPlugin, MethodChannel.MethodCallHandler, ActivityAware {
    companion object {
        /** 独立图层的平台视图类型（与 Dart 侧 `MiniProgramView` 同一个字符串） */
        const val VIEW_TYPE = "dev.minirender/engine-view"
    }

    private lateinit var channel: MethodChannel
    private lateinit var binding: FlutterPlugin.FlutterPluginBinding
    private var host: EngineHost? = null

    /** 插件自带的宿主能力：选图 / 选视频 / 选文件 / 存相册 / 打开文档（要 Activity 收选择器的结果） */
    private lateinit var picker: MediaPicker

    override fun onAttachedToEngine(flutterPluginBinding: FlutterPlugin.FlutterPluginBinding) {
        binding = flutterPluginBinding
        picker = MediaPicker(flutterPluginBinding.applicationContext)
        channel = MethodChannel(flutterPluginBinding.binaryMessenger, "dev.minirender/engine")
        channel.setMethodCallHandler(this)
        // 独立图层上屏（EngineView.kt）：Dart 在 create 回了 view=true 时建这个平台视图
        flutterPluginBinding.platformViewRegistry.registerViewFactory(VIEW_TYPE, EngineViewFactory { host })
    }

    override fun onDetachedFromEngine(flutterPluginBinding: FlutterPlugin.FlutterPluginBinding) {
        channel.setMethodCallHandler(null)
        host?.dispose()
        host = null
    }

    override fun onAttachedToActivity(b: ActivityPluginBinding) = picker.attach(b)

    override fun onDetachedFromActivityForConfigChanges() = picker.detach()

    override fun onReattachedToActivityForConfigChanges(b: ActivityPluginBinding) = picker.attach(b)

    override fun onDetachedFromActivity() = picker.detach()

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
            // 不碰引擎实例：文件直接写进 hostCall 给的 tempDir
            "pickMedia", "pickFiles", "saveToAlbum", "openDocument" -> picker.handle(call, result)
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

    /**
     * 系统信息里宿主没给的设备字段在这里补（与原生 MiniProgramView 同一组）。Dart 拿不到
     * Android 版本号（`Platform.operatingSystemVersion` 是内核串），不补的话 `wx.getSystemInfoSync().system`
     * 落到逻辑层的缺省 `iOS 15.0` —— 真机首页显示「android iOS 15.0」。宿主给了的不覆盖。
     */
    private fun withDeviceFields(options: String): String = try {
        val o = JSONObject(options)
        val si = o.optJSONObject("systemInfo") ?: JSONObject().also { o.put("systemInfo", it) }
        if (!si.has("system")) si.put("system", "Android ${Build.VERSION.RELEASE}")
        if (!si.has("brand")) si.put("brand", Build.BRAND)
        if (!si.has("model")) si.put("model", Build.MODEL)
        o.toString()
    } catch (e: org.json.JSONException) {
        options
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
        // `adb shell setprop debug.mini.fps 1` 这类诊断开关：引擎创建时读环境，必须在这之前设好
        DiagProps.apply()
        val h = EngineHost(
            binding.applicationContext,
            binding.textureRegistry,
            onEvents = { channel.invokeMethod("events", it) },
            onTexture = { channel.invokeMethod("texture", it) },
            // "view"（缺省）：自己的 SurfaceView，系统直接合成；"texture"：Flutter 外部纹理（旧路）
            viewMode = call.argument<String>("presentation") != "texture",
        )
        host = h
        h.create(
            appDir = appDir,
            dataDir = dataDir,
            route = call.argument<String>("route"),
            width = (call.argument<Number>("width") ?: 1).toInt(),
            height = (call.argument<Number>("height") ?: 1).toInt(),
            dpr = (call.argument<Number>("dpr") ?: 1).toFloat(),
            options = withDeviceFields(call.argument<String>("options") ?: "{}"),
            // 0 auto / 1 cpu / 2 gpu（MiniEngine.BACKEND_*）；默认 auto：有 GPU 就用 GPU，不行回退 CPU
            backend = (call.argument<Number>("backend") ?: MiniEngine.BACKEND_AUTO).toInt(),
            device = deviceInfo(),
            result = result,
            onFailed = { if (host === h) host = null },
        )
    }
}
