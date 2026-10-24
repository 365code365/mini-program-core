package dev.minirender.example

import android.app.Activity
import android.app.AlertDialog
import android.content.Context
import android.os.Build
import android.os.Process
import android.util.Log
import android.widget.Toast
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import java.util.zip.GZIPOutputStream
import kotlin.concurrent.thread

/**
 * 「上传诊断日志」：把**本进程**的 logcat（引擎 `mini-render`、宿主事件 `mini-example`、系统报错都在里面）
 * 连同机型、系统、当前小程序、本地装着的版本打成一份 JSON，gzip 后 POST 到 mini-store 的 `/api/diag`。
 * 开发机上 `bash tools/mini-store/diag.sh get latest` 拉回来分析。
 *
 * App 读自己进程的 logcat 不要权限（`--pid` 只看自己）；logcat 是环形缓冲，太久以前的会被冲掉 ——
 * 所以要在出问题之后尽快点。与 Flutter 示例 lib/diag_upload.dart 上传的是同一种 JSON
 */
object DiagUpload {
    private const val MAX_LOG_BYTES = 3 shl 20

    /** 问一句要不要附一句说明，然后后台收集 + 上传，结果弹框给出诊断 id */
    fun ask(activity: Activity, baseUrl: String, app: JSONObject? = null, backend: String? = null) {
        val input = android.widget.EditText(activity).apply { hint = "发生了什么（可不填）" }
        AlertDialog.Builder(activity)
            .setTitle("上传诊断日志")
            .setMessage("会上传本 App 的运行日志、机型与系统版本，用于排查问题。")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("上传") { _, _ -> upload(activity, baseUrl, input.text.toString(), app, backend) }
            .show()
    }

    fun upload(activity: Activity, baseUrl: String, note: String, app: JSONObject?, backend: String?) {
        Toast.makeText(activity, "正在收集日志…", Toast.LENGTH_SHORT).show()
        thread(name = "diag-upload") {
            val result = runCatching {
                val body = collect(activity, note, app, backend)
                post("$baseUrl/api/diag", body)
            }
            activity.runOnUiThread {
                if (activity.isFinishing) return@runOnUiThread
                val (title, msg) = result.fold(
                    { id -> "已上传" to "诊断 id：$id\n\n把这个 id 发给开发，或在开发机上跑\nbash tools/mini-store/diag.sh get $id" },
                    { e -> "上传失败" to (e.message ?: e.javaClass.simpleName) },
                )
                Log.i(MainActivity.TAG, "[诊断] $title $msg")
                AlertDialog.Builder(activity).setTitle(title).setMessage(msg).setPositiveButton("好", null).show()
            }
        }
    }

    private fun collect(ctx: Context, note: String, app: JSONObject?, backend: String?): JSONObject {
        val pkg = ctx.packageManager.getPackageInfo(ctx.packageName, 0)
        // 本地装着的商店小程序（版本对不上是常见问题来源）
        val installed = JSONArray()
        File(ctx.filesDir, "mini-apps").listFiles { f -> f.isFile && f.name.endsWith(".json") && f.name != "index.json" }
            ?.forEach { f -> runCatching { installed.put(JSONObject(f.readText())) } }
        return JSONObject()
            .put("host", "android-native")
            .put("hostVersion", "${pkg.versionName} (${pkg.lastUpdateTime})")
            .put("platform", "android")
            .put("device", "${Build.BRAND} ${Build.MODEL}")
            .put("system", "Android ${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT})")
            .put("abi", Build.SUPPORTED_ABIS.joinToString(","))
            .put("backend", backend ?: JSONObject.NULL)
            .put("app", app ?: JSONObject.NULL)
            .put("store", JSONObject().put("installed", installed))
            .put("note", note)
            .put("clientTime", System.currentTimeMillis())
            .put("logs", logcat())
    }

    /** 本进程的 logcat（`-d` 读完就退出）；太长只留最后 [MAX_LOG_BYTES] */
    private fun logcat(): String {
        val p = ProcessBuilder("logcat", "-d", "-v", "threadtime", "--pid=${Process.myPid()}")
            .redirectErrorStream(true).start()
        val text = p.inputStream.bufferedReader().readText()
        p.waitFor()
        return if (text.length > MAX_LOG_BYTES) "…（前面截掉 ${text.length - MAX_LOG_BYTES} 字符）\n" + text.takeLast(MAX_LOG_BYTES) else text
    }

    private fun post(url: String, body: JSONObject): String {
        val raw = ByteArrayOutputStream().also { out -> GZIPOutputStream(out).use { it.write(body.toString().toByteArray()) } }.toByteArray()
        val c = URL(url).openConnection() as HttpURLConnection
        c.requestMethod = "POST"
        c.doOutput = true
        c.connectTimeout = 8000
        c.readTimeout = 30000
        c.setRequestProperty("Content-Type", "application/json")
        c.setRequestProperty("Content-Encoding", "gzip")
        c.setFixedLengthStreamingMode(raw.size)
        try {
            c.outputStream.use { it.write(raw) }
            val code = c.responseCode
            val text = (if (code in 200..299) c.inputStream else c.errorStream)?.bufferedReader()?.readText().orEmpty()
            val j = runCatching { JSONObject(text) }.getOrNull()
            if (code !in 200..299) throw java.io.IOException(j?.optString("error")?.ifEmpty { null } ?: "HTTP $code")
            return j?.optString("id")?.ifEmpty { null } ?: throw java.io.IOException("服务器没回诊断 id")
        } finally {
            c.disconnect()
        }
    }
}
