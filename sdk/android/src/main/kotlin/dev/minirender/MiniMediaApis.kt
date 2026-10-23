package dev.minirender

import android.Manifest
import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.ext.SdkExtensions
import android.provider.MediaStore
import android.webkit.MimeTypeMap
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.concurrent.Executors

/**
 * SDK 自带的宿主能力：选图 / 拍照 / 选视频 / 选文件 / 存相册 / 打开文档（[MiniProgramView.builtinHostApis]）。
 *
 * 与 Flutter 插件的同一组（Dart `media.dart` + Kotlin `MediaPicker.kt`）一条一条对齐：同样的参数换算、同样的回包形状、
 * 同样的错误码 → errMsg。不同的只有「怎么拿到选择器的结果」：插件挂在 Flutter 的 Activity 上，
 * 这里由透明的 [MiniMediaActivity] 代办（宿主不用转发 onActivityResult）。
 *
 * - 相册走系统的**照片选择器**（Android 13+，或 11+ 带 SDK 扩展 2）：不用任何存储权限；更老的系统退到 `ACTION_GET_CONTENT`；
 * - 相机走系统相机应用：宿主清单**没声明** CAMERA 时不用申请（系统相机自己有权限），声明了就先申请；
 *   拍的东西先写进 [MiniFileProvider] 能交出去的缓存目录，再挪进引擎的 tempDir；
 * - 拷贝 / 摆正 / 压缩 / 取视频封面在后台线程（[MediaImport]），一段视频几十 MB，放主线程就是 ANR。
 */
internal class MiniMediaApis(context: Context) {
    companion object {
        /** 自带的 API 名（与 Flutter `MiniBuiltinHostApis.names` 一致） */
        val NAMES = listOf(
            "chooseImage", "chooseMedia", "chooseVideo", "chooseMessageFile",
            "saveImageToPhotosAlbum", "saveVideoToPhotosAlbum", "openDocument",
        )
    }

    /** 原生侧的失败：`code` 与插件一致（cancel / denied / not_found / busy / io） */
    private class Fail(val code: String, msg: String? = null) : Exception(msg)

    private val ctx = context.applicationContext
    private val io = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())
    private val import = MediaImport(ctx)
    /** 一次只开一个选择器（与插件一致：第二个回 busy） */
    private var busy = false

    /** 主线程调 */
    fun handle(activity: Activity?, call: MiniHostCall, reply: MiniHostReply) {
        val done = { r: Result<JSONObject> ->
            r.fold({ reply.success(it) }, { reply.fail("${call.api}:fail ${reason(it)}") })
        }
        try {
            route(activity, call, done)
        } catch (e: Exception) {
            done(Result.failure(e))
        }
    }

    private fun route(activity: Activity?, call: MiniHostCall, done: (Result<JSONObject>) -> Unit) {
        val a = call.args
        when (call.api) {
            "chooseImage" -> pickMedia(activity, call, "image", a.optInt("count", 9), onlyCompressed(a), done) { items ->
                JSONObject()
                    .put("tempFilePaths", JSONArray(items.map { it["path"] }))
                    .put("tempFiles", JSONArray(items.map { JSONObject().put("path", it["path"]).put("size", it["size"] ?: 0) }))
            }
            "chooseMedia" -> {
                val types = strings(a.opt("mediaType"))
                val img = "image" in types || "mix" in types
                val vid = "video" in types || "mix" in types
                val kind = if (img && vid) "mix" else if (vid) "video" else "image"
                pickMedia(activity, call, kind, a.optInt("count", 9), onlyCompressed(a), done) { items ->
                    JSONObject().put("tempFiles", JSONArray(items.map { f ->
                        JSONObject().put("tempFilePath", f["path"]).put("size", f["size"] ?: 0)
                            .put("width", f["width"] ?: 0).put("height", f["height"] ?: 0)
                            .put("fileType", f["fileType"] ?: "image").apply {
                                if (f["fileType"] == "video") put("duration", f["duration"] ?: 0).put("thumbTempFilePath", f["thumb"] ?: "")
                            }
                    }))
                }
            }
            "chooseVideo" -> pickMedia(activity, call, "video", 1, a.optBoolean("compressed", true), done) { items ->
                val f = items.first()
                JSONObject().put("tempFilePath", f["path"]).put("duration", f["duration"] ?: 0).put("size", f["size"] ?: 0)
                    .put("width", f["width"] ?: 0).put("height", f["height"] ?: 0).apply {
                        if ("${f["thumb"] ?: ""}".isNotEmpty()) put("thumbTempFilePath", f["thumb"])
                    }
            }
            "chooseMessageFile" -> pickFiles(activity, call, done)
            "saveImageToPhotosAlbum", "saveVideoToPhotosAlbum" ->
                saveToAlbum(activity, localPath(call), if (call.api == "saveVideoToPhotosAlbum") "video" else "image", done)
            "openDocument" -> openDocument(activity, localPath(call), a.optString("fileType"), done)
            else -> throw Fail("io", "SDK 没有内置这个 API")
        }
    }

    // ───────────────────────── 选图 / 拍照 / 选视频 ─────────────────────────

    private fun pickMedia(
        activity: Activity?,
        call: MiniHostCall,
        kind: String,
        count: Int,
        compressed: Boolean,
        done: (Result<JSONObject>) -> Unit,
        shape: (List<Map<String, Any?>>) -> JSONObject,
    ) {
        val a = call.args
        val dir = tempDir(call)
        val n = count.coerceAtLeast(1)
        // 引擎在移动端已经问过「拍照 / 从手机相册选择」，这里只剩一种来源
        val camera = "album" !in strings(a.opt("sourceType"))
        val shot = if (camera) MediaImport.newFile(File(ctx.cacheDir, "mini-render-capture").apply { mkdirs() }, if (kind == "video") "mp4" else "jpg") else null
        val intent = if (shot != null) captureIntent(shot, kind == "video", a, compressed) else albumIntent(kind, n)
        val perm = Manifest.permission.CAMERA.takeIf { shot != null && MiniMediaActivity.declares(ctx, it) }
        // 取消 / 失败时拍了一半的文件别留在缓存里（成功时 MediaImport 已经把它挪走）
        val finish = { r: Result<JSONObject> ->
            if (r.isFailure) shot?.delete()
            done(r)
        }
        launch(activity, perm, intent, finish) { data ->
            val sources = if (shot != null) {
                if (shot.length() == 0L) throw Fail("cancel")
                listOf(MediaImport.Source.Shot(shot))
            } else {
                picked(data).take(n).map { MediaImport.Source.Picked(it) }.ifEmpty { throw Fail("cancel") }
            }
            shape(import.media(sources, dir, compressed).ifEmpty { throw Fail("cancel") })
        }
    }

    private fun albumIntent(kind: String, count: Int): Intent = if (photoPickerAvailable()) {
        Intent(MediaStore.ACTION_PICK_IMAGES).apply {
            when (kind) {
                "image" -> type = "image/*"
                "video" -> type = "video/*"
            }
            // 只能在 >1 时给，=1 时给了会直接抛异常
            if (count > 1) putExtra(MediaStore.EXTRA_PICK_IMAGES_MAX, count.coerceAtMost(MediaStore.getPickImagesMaxLimit()))
        }
    } else {
        Intent(Intent.ACTION_GET_CONTENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = when (kind) {
                "image" -> "image/*"
                "video" -> "video/*"
                else -> "*/*"
            }
            if (kind == "mix") putExtra(Intent.EXTRA_MIME_TYPES, arrayOf("image/*", "video/*"))
            if (count > 1) putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
        }
    }

    private fun photoPickerAvailable(): Boolean = when {
        Build.VERSION.SDK_INT >= 33 -> true
        Build.VERSION.SDK_INT >= 30 -> SdkExtensions.getExtensionVersion(Build.VERSION_CODES.R) >= 2
        else -> false
    }

    /** 系统相机。`mix` 拍照片（一个 Intent 没法「点拍照、按住录像」） */
    private fun captureIntent(shot: File, video: Boolean, a: JSONObject, compressed: Boolean): Intent {
        val uri = MiniFileProvider.uriFor(ctx, shot)
        return Intent(if (video) MediaStore.ACTION_VIDEO_CAPTURE else MediaStore.ACTION_IMAGE_CAPTURE).apply {
            putExtra(MediaStore.EXTRA_OUTPUT, uri)
            // 有的相机应用只认 ClipData 上的授权
            clipData = ClipData.newRawUri("", uri)
            addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_READ_URI_PERMISSION)
            if (video) {
                putExtra(MediaStore.EXTRA_DURATION_LIMIT, a.optInt("maxDuration", 60))
                putExtra(MediaStore.EXTRA_VIDEO_QUALITY, if (compressed) 0 else 1)
            }
            if (a.optString("camera") == "front") {
                // 没有标准写法：各家相机认的键不同，三个都给
                putExtra("android.intent.extras.CAMERA_FACING", 1)
                putExtra("android.intent.extras.LENS_FACING_FRONT", 1)
                putExtra("android.intent.extra.USE_FRONT_CAMERA", true)
            }
        }
    }

    // ───────────────────────── 选文件 ─────────────────────────

    private fun pickFiles(activity: Activity?, call: MiniHostCall, done: (Result<JSONObject>) -> Unit) {
        val a = call.args
        val dir = tempDir(call)
        val count = a.optInt("count", 1).coerceAtLeast(1)
        val type = a.optString("type", "all")
        val exts = strings(a.opt("extension")).map { it.lowercase() }
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            this.type = "*/*"
            val mimes = when (type) {
                "image" -> listOf("image/*")
                "video" -> listOf("video/*")
                // 扩展名都认得出 MIME 才过滤；有一个认不出就不过滤（落盘时再按扩展名筛）
                "file" -> exts.map { MimeTypeMap.getSingleton().getMimeTypeFromExtension(it) }
                    .takeIf { it.isNotEmpty() && it.all { m -> m != null } }?.filterNotNull()
                else -> null
            }
            if (mimes != null) putExtra(Intent.EXTRA_MIME_TYPES, mimes.toTypedArray())
            if (count > 1) putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
        }
        launch(activity, null, intent, done) { data ->
            val uris = picked(data).take(count).ifEmpty { throw Fail("cancel") }
            val items = import.files(uris, dir, type, exts).ifEmpty { throw Fail("cancel") }
            JSONObject().put("tempFiles", JSONArray(items.map { f ->
                JSONObject().put("path", f["path"]).put("size", f["size"] ?: 0).put("name", f["name"] ?: "")
                    .put("type", f["fileType"] ?: "file").put("time", f["time"] ?: 0)
            }))
        }
    }

    // ───────────────────────── 存相册 / 打开文档 ─────────────────────────

    private fun saveToAlbum(activity: Activity?, file: File, kind: String, done: (Result<JSONObject>) -> Unit) {
        if (!file.isFile) throw Fail("not_found")
        val go = { background(done) { import.saveToAlbum(file, kind); JSONObject() } }
        // Android 10+ 往相册写自己的文件不要权限；更老的要 WRITE_EXTERNAL_STORAGE（宿主声明了才能申请）
        if (Build.VERSION.SDK_INT >= 29) return go()
        val perm = Manifest.permission.WRITE_EXTERNAL_STORAGE
        if (!MiniMediaActivity.declares(ctx, perm)) throw Fail("denied", "宿主清单没有声明 $perm")
        if (ctx.checkSelfPermission(perm) == android.content.pm.PackageManager.PERMISSION_GRANTED) return go()
        if (busy) throw Fail("busy", "上一次选择还没结束")
        busy = true
        MiniMediaActivity.launch(ctx, activity, MiniMediaActivity.Request(perm, null) { o, close ->
            busy = false
            close()
            if (o is MiniMediaActivity.Outcome.Result && o.code == Activity.RESULT_OK) go()
            else done(Result.failure(if (o is MiniMediaActivity.Outcome.Denied) Fail("denied", "storage") else Fail("cancel")))
        })
    }

    private fun openDocument(activity: Activity?, file: File, fileType: String, done: (Result<JSONObject>) -> Unit) {
        if (!file.isFile) throw Fail("not_found")
        val shared = try {
            MiniFileProvider.uriFor(ctx, file)
        } catch (e: IllegalArgumentException) {
            // 宿主的 dataDir 不在 cacheDir / filesDir 下面：拷一份到缓存再交出去
            val copy = File(File(ctx.cacheDir, "mini-render-share").apply { mkdirs() }, file.name)
            file.copyTo(copy, overwrite = true)
            MiniFileProvider.uriFor(ctx, copy)
        }
        val ext = fileType.ifEmpty { file.extension }
        val mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(ext.lowercase()) ?: "*/*"
        val intent = Intent(Intent.ACTION_VIEW).setDataAndType(shared, mime).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        try {
            if (activity != null) activity.startActivity(intent) else ctx.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            done(Result.success(JSONObject()))
        } catch (e: ActivityNotFoundException) {
            throw Fail("io", "没有能打开 .$ext 的应用")
        }
    }

    // ───────────────────────── 公用 ─────────────────────────

    /** 代办 Activity 开选择器；拿到结果后 [import] 在后台线程上落盘，回包回到主线程 */
    private fun launch(
        activity: Activity?,
        permission: String?,
        intent: Intent,
        done: (Result<JSONObject>) -> Unit,
        import: (Intent?) -> JSONObject,
    ) {
        if (busy) throw Fail("busy", "上一次选择还没结束")
        busy = true
        MiniMediaActivity.launch(ctx, activity, MiniMediaActivity.Request(permission, intent) { o, close ->
            busy = false
            when (o) {
                is MiniMediaActivity.Outcome.Denied -> close().also { done(Result.failure(Fail("denied", "camera"))) }
                is MiniMediaActivity.Outcome.NoApp -> close().also { done(Result.failure(Fail("io", "系统里没有能处理的应用（${o.action}）"))) }
                is MiniMediaActivity.Outcome.Result ->
                    if (o.code != Activity.RESULT_OK) {
                        close()
                        done(Result.failure(Fail("cancel")))
                    } else {
                        // 拷完再关代办 Activity：选择器给的 URI 授权跟着它走（见 MiniMediaActivity）
                        background({ r -> close(); done(r) }) { import(o.data) }
                    }
            }
        })
    }

    private fun background(done: (Result<JSONObject>) -> Unit, work: () -> JSONObject) = io.execute {
        val r = try {
            Result.success(work())
        } catch (e: Exception) {
            Result.failure(e)
        }
        main.post { done(r) }
    }

    private fun picked(data: Intent?): List<Uri> {
        val clip = data?.clipData
        if (clip != null && clip.itemCount > 0) return (0 until clip.itemCount).mapNotNull { clip.getItemAt(it).uri }
        return listOfNotNull(data?.data)
    }

    private fun tempDir(call: MiniHostCall): File {
        if (call.tempDir.isEmpty()) throw Fail("io", "引擎没有给 tempDir（引擎库太旧）")
        return File(call.tempDir).apply { mkdirs() }
    }

    /** `filePath` 的真实路径：引擎在 [MiniHostCall.files] 里给了；本来就是本地绝对路径的原样用 */
    private fun localPath(call: MiniHostCall): File {
        val p = call.args.optString("filePath")
        val real = call.files[p] ?: if (p.startsWith("file://")) p.substring(7) else p.takeIf { it.startsWith("/") }
        return File(real ?: throw Fail("not_found", "file not found"))
    }

    private fun onlyCompressed(a: JSONObject): Boolean {
        val s = strings(a.opt("sizeType"))
        return "compressed" in s && "original" !in s
    }

    private fun strings(v: Any?): List<String> = when (v) {
        is JSONArray -> (0 until v.length()).map { "${v.opt(it)}" }
        is String -> listOf(v)
        else -> emptyList()
    }

    /** 原生侧的错误 → 微信的 errMsg 尾巴（与插件 media.dart `_reason` 一致） */
    private fun reason(e: Throwable): String = when (e) {
        is Fail -> when (e.code) {
            "cancel" -> "cancel"
            "denied" -> "auth deny" + (e.message?.let { "（$it）" } ?: "")
            "not_found" -> "file not found"
            else -> e.message ?: e.code
        }
        else -> e.message ?: e.toString()
    }
}
