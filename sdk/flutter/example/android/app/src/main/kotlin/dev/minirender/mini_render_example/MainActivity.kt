package dev.minirender.mini_render_example

import android.content.Intent
import android.os.Build
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

/**
 * 外部 App / H5 拉起小程序的链接（`minirender://open?appid=…`，清单里的 intent-filter）交给 Dart
 * （lib/store/store_links.dart）：冷启动带进来的那条用 `initial` 取，运行中来的（singleTop → onNewIntent）推 `link`。
 * 同一条通道还给「上传诊断日志」（lib/diag_upload.dart）取本进程的 logcat（`logcat`）
 */
class MainActivity : FlutterActivity() {
    private var links: MethodChannel? = null
    private var initial: String? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        initial = linkOf(intent)
        links = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "mini_render_example/links").apply {
            setMethodCallHandler { call, result ->
                when (call.method) {
                    "initial" -> {
                        result.success(initial)
                        initial = null
                    }
                    // 读 logcat 要几百毫秒：放后台线程，回主线程交结果
                    "logcat" -> Thread {
                        val r = runCatching { logcat() }
                        runOnUiThread { r.fold({ result.success(it) }, { result.error("LOGCAT", it.message, null) }) }
                    }.start()
                    else -> result.notImplemented()
                }
            }
        }
    }

    /** 「上传诊断日志」用：本进程的 logcat（读自己的不要权限）+ 机型。太长只留最后 3MB */
    private fun logcat(): Map<String, Any> {
        val p = ProcessBuilder("logcat", "-d", "-v", "threadtime", "--pid=${android.os.Process.myPid()}")
            .redirectErrorStream(true).start()
        var text = p.inputStream.bufferedReader().readText()
        p.waitFor()
        val max = 3 shl 20
        if (text.length > max) text = "…（前面截掉 ${text.length - max} 字符）\n" + text.takeLast(max)
        return mapOf(
            "logcat" to text,
            "device" to "${Build.BRAND} ${Build.MODEL}",
            "system" to "Android ${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT})",
            "abi" to Build.SUPPORTED_ABIS.joinToString(","),
        )
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        linkOf(intent)?.let { links?.invokeMethod("link", it) }
    }

    private fun linkOf(i: Intent?): String? =
        i?.takeIf { it.action == Intent.ACTION_VIEW }?.dataString?.takeIf { it.startsWith("minirender:") }
}
