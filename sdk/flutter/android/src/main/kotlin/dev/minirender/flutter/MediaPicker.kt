package dev.minirender.flutter

import android.Manifest
import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.ext.SdkExtensions
import android.provider.MediaStore
import android.webkit.MimeTypeMap
import androidx.core.content.FileProvider
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugin.common.PluginRegistry
import java.io.File
import java.util.concurrent.Executors

/**
 * 插件自带的宿主能力的原生一半（Dart 那一半见 `lib/src/media.dart`）：打开系统的选择器 / 相机，
 * 拿到的东西交给 [MediaImport] 在后台线程落盘。通道方法：
 *
 * - `pickMedia` {dir, kind: image|video|mix, source: album|camera, count, compressed, maxDuration, camera: back|front}
 * - `pickFiles` {dir, count, type: all|image|video|file, extension}
 * - `saveToAlbum` {path, kind: image|video}
 * - `openDocument` {path, fileType}
 *
 * 错误码：`cancel`（用户返回 / 什么都没选）、`denied`（没给权限）、`not_found`、`busy`、`io`。
 *
 * 相册走系统的**照片选择器**（Android 13+，或 11+ 带 SDK 扩展 2）：不用任何存储权限，用户只交出选中的那几张；
 * 更老的系统退到 `ACTION_GET_CONTENT`。相机走系统相机应用：宿主清单里**没声明** CAMERA 权限时不用申请
 * （系统相机自己有权限）；声明了就必须先拿到授权，否则系统直接拒绝启动相机 —— 这里替宿主申请。
 * 拍的照片先写进插件的 FileProvider 能给出去的缓存目录，再挪进引擎的 tempDir（宿主的 dataDir 可能在
 * FileProvider 管不到的地方）。
 */
class MediaPicker(private val ctx: Context) :
    PluginRegistry.ActivityResultListener,
    PluginRegistry.RequestPermissionsResultListener {

    private companion object {
        // 与别的插件错开：Activity 的请求码是全局共用的
        const val REQ_PICK = 0x6d72
        const val REQ_CAPTURE = 0x6d73
        const val REQ_FILES = 0x6d74
        const val REQ_PERMISSION = 0x6d75
    }

    /** 一次只开一个选择器：回来的时候按它的参数落盘 */
    private class Pending(
        val req: Int,
        val call: MethodCall,
        val result: MethodChannel.Result,
        val dir: File,
        val shot: File? = null,
    )

    private var binding: ActivityPluginBinding? = null
    private var pending: Pending? = null
    private var onPermission: ((Boolean) -> Unit)? = null
    private val io = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())
    private val import = MediaImport(ctx)

    fun attach(b: ActivityPluginBinding) {
        binding = b
        b.addActivityResultListener(this)
        b.addRequestPermissionsResultListener(this)
    }

    fun detach() {
        binding?.removeActivityResultListener(this)
        binding?.removeRequestPermissionsResultListener(this)
        binding = null
        // Activity 没了，选择器的结果回不来：别让小程序那边永远等着
        pending?.result?.error("cancel", null, null)
        pending = null
        onPermission = null
    }

    fun handle(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "pickMedia" -> pickMedia(call, result)
            "pickFiles" -> pickFiles(call, result)
            "saveToAlbum" -> saveToAlbum(call, result)
            "openDocument" -> openDocument(call, result)
            else -> result.notImplemented()
        }
    }

    private fun ready(call: MethodCall, result: MethodChannel.Result): Pair<Activity, File>? {
        val act = binding?.activity
        if (act == null) {
            result.error("io", "插件还没接上 Activity", null)
            return null
        }
        if (pending != null || onPermission != null) {
            result.error("busy", "上一次选择还没结束", null)
            return null
        }
        val dir = call.argument<String>("dir")
        if (dir.isNullOrEmpty()) {
            result.error("io", "缺 dir", null)
            return null
        }
        return act to File(dir).apply { mkdirs() }
    }

    private fun pickMedia(call: MethodCall, result: MethodChannel.Result) {
        val (act, dir) = ready(call, result) ?: return
        val kind = call.argument<String>("kind") ?: "image"
        val count = (call.argument<Number>("count") ?: 1).toInt().coerceAtLeast(1)
        if (call.argument<String>("source") == "camera") {
            withPermission(Manifest.permission.CAMERA, "camera", result) { capture(act, call, result, dir, kind) }
            return
        }
        val intent = if (photoPickerAvailable()) {
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
        start(act, intent, Pending(REQ_PICK, call, result, dir))
    }

    private fun photoPickerAvailable(): Boolean = when {
        Build.VERSION.SDK_INT >= 33 -> true
        Build.VERSION.SDK_INT >= 30 -> SdkExtensions.getExtensionVersion(Build.VERSION_CODES.R) >= 2
        else -> false
    }

    /** 系统相机。`mix` 拍照片（一个 Intent 没法「点拍照、按住录像」） */
    private fun capture(act: Activity, call: MethodCall, result: MethodChannel.Result, dir: File, kind: String) {
        val video = kind == "video"
        val shot = MediaImport.newFile(File(ctx.cacheDir, "mini-render-capture").apply { mkdirs() }, if (video) "mp4" else "jpg")
        val uri = FileProvider.getUriForFile(ctx, authority(), shot)
        val intent = Intent(if (video) MediaStore.ACTION_VIDEO_CAPTURE else MediaStore.ACTION_IMAGE_CAPTURE).apply {
            putExtra(MediaStore.EXTRA_OUTPUT, uri)
            // 有的相机应用只认 ClipData 上的授权
            clipData = ClipData.newRawUri("", uri)
            addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_READ_URI_PERMISSION)
            if (video) {
                putExtra(MediaStore.EXTRA_DURATION_LIMIT, (call.argument<Number>("maxDuration") ?: 60).toInt())
                putExtra(MediaStore.EXTRA_VIDEO_QUALITY, if (call.argument<Boolean>("compressed") == true) 0 else 1)
            }
            if (call.argument<String>("camera") == "front") {
                // 没有标准写法：各家相机认的键不同，三个都给
                putExtra("android.intent.extras.CAMERA_FACING", 1)
                putExtra("android.intent.extras.LENS_FACING_FRONT", 1)
                putExtra("android.intent.extra.USE_FRONT_CAMERA", true)
            }
        }
        start(act, intent, Pending(REQ_CAPTURE, call, result, dir, shot))
    }

    private fun pickFiles(call: MethodCall, result: MethodChannel.Result) {
        val (act, dir) = ready(call, result) ?: return
        val count = (call.argument<Number>("count") ?: 1).toInt().coerceAtLeast(1)
        val type = call.argument<String>("type") ?: "all"
        val exts = call.argument<List<String>>("extension").orEmpty()
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            this.type = "*/*"
            val mimes = when (type) {
                "image" -> listOf("image/*")
                "video" -> listOf("video/*")
                // 扩展名都认得出 MIME 才过滤；有一个认不出就不过滤（落盘时再按扩展名筛）
                "file" -> exts.map { MimeTypeMap.getSingleton().getMimeTypeFromExtension(it.lowercase()) }
                    .takeIf { it.isNotEmpty() && it.all { m -> m != null } }?.filterNotNull()
                else -> null
            }
            if (mimes != null) putExtra(Intent.EXTRA_MIME_TYPES, mimes.toTypedArray())
            if (count > 1) putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
        }
        start(act, intent, Pending(REQ_FILES, call, result, dir))
    }

    private fun start(act: Activity, intent: Intent, p: Pending) {
        try {
            pending = p
            act.startActivityForResult(intent, p.req)
        } catch (e: ActivityNotFoundException) {
            pending = null
            p.shot?.delete()
            p.result.error("io", "系统里没有能处理的应用（${intent.action}）", null)
        }
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?): Boolean {
        val p = pending ?: return false
        if (requestCode != p.req) return false
        pending = null
        val shot = p.shot
        if (resultCode != Activity.RESULT_OK || (shot != null && shot.length() == 0L)) {
            shot?.delete()
            p.result.error("cancel", null, null)
            return true
        }
        val count = (p.call.argument<Number>("count") ?: 1).toInt().coerceAtLeast(1)
        val uris = if (shot != null) emptyList() else picked(data).take(count)
        if (shot == null && uris.isEmpty()) {
            p.result.error("cancel", null, null)
            return true
        }
        // 拷贝 / 转码 / 取封面在后台线程：一段视频几十 MB，放主线程就是 ANR
        io.execute {
            try {
                val out = when (p.req) {
                    REQ_FILES -> import.files(
                        uris,
                        p.dir,
                        p.call.argument<String>("type") ?: "all",
                        p.call.argument<List<String>>("extension").orEmpty().map { it.lowercase() },
                    )
                    else -> {
                        val compressed = p.call.argument<Boolean>("compressed") == true
                        val sources = if (shot != null) listOf(MediaImport.Source.Shot(shot)) else uris.map { MediaImport.Source.Picked(it) }
                        import.media(sources, p.dir, compressed)
                    }
                }
                main.post { if (out.isEmpty()) p.result.error("cancel", null, null) else p.result.success(out) }
            } catch (e: Exception) {
                main.post { p.result.error("io", e.message ?: e.toString(), null) }
            }
        }
        return true
    }

    private fun picked(data: Intent?): List<Uri> {
        val clip = data?.clipData
        if (clip != null && clip.itemCount > 0) return (0 until clip.itemCount).mapNotNull { clip.getItemAt(it).uri }
        return listOfNotNull(data?.data)
    }

    private fun saveToAlbum(call: MethodCall, result: MethodChannel.Result) {
        val file = File(call.argument<String>("path") ?: "")
        if (!file.isFile) {
            result.error("not_found", null, null)
            return
        }
        val kind = call.argument<String>("kind") ?: "image"
        val go = {
            io.execute {
                try {
                    import.saveToAlbum(file, kind)
                    main.post { result.success(null) }
                } catch (e: Exception) {
                    main.post { result.error("io", e.message ?: e.toString(), null) }
                }
            }
        }
        // Android 10+ 往相册写自己的文件不要权限；更老的要 WRITE_EXTERNAL_STORAGE（宿主声明了才能申请）
        if (Build.VERSION.SDK_INT >= 29) {
            go()
        } else {
            withPermission(Manifest.permission.WRITE_EXTERNAL_STORAGE, "storage", result, requireDeclared = true) { go() }
        }
    }

    private fun openDocument(call: MethodCall, result: MethodChannel.Result) {
        val act = binding?.activity
        val file = File(call.argument<String>("path") ?: "")
        if (act == null || !file.isFile) {
            result.error(if (act == null) "io" else "not_found", null, null)
            return
        }
        val uri = try {
            FileProvider.getUriForFile(ctx, authority(), file)
        } catch (e: IllegalArgumentException) {
            // 宿主的 dataDir 不在 FileProvider 管的目录里：拷一份到缓存再给出去
            val copy = File(File(ctx.cacheDir, "mini-render-share").apply { mkdirs() }, file.name)
            file.copyTo(copy, overwrite = true)
            FileProvider.getUriForFile(ctx, authority(), copy)
        }
        val ext = call.argument<String>("fileType")?.takeIf { it.isNotEmpty() } ?: file.extension
        val mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(ext.lowercase()) ?: "*/*"
        val intent = Intent(Intent.ACTION_VIEW).setDataAndType(uri, mime).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        try {
            act.startActivity(intent)
            result.success(null)
        } catch (e: ActivityNotFoundException) {
            result.error("io", "没有能打开 .$ext 的应用", null)
        }
    }

    /**
     * 要权限的那几步：宿主清单里没声明就不用申请（系统相机自己有相机权限）——除非 [requireDeclared]
     * （老系统写相册，没声明就是做不了）；声明了、还没授权就申请一次
     */
    private fun withPermission(perm: String, label: String, result: MethodChannel.Result, requireDeclared: Boolean = false, go: () -> Unit) {
        val declared = declares(perm)
        if (!declared && !requireDeclared) return go()
        if (!declared) return result.error("denied", "宿主清单没有声明 $perm", null)
        if (ctx.checkSelfPermission(perm) == PackageManager.PERMISSION_GRANTED) return go()
        val act = binding?.activity ?: return result.error("io", "插件还没接上 Activity", null)
        onPermission = { granted -> if (granted) go() else result.error("denied", label, null) }
        act.requestPermissions(arrayOf(perm), REQ_PERMISSION)
    }

    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<out String>, grantResults: IntArray): Boolean {
        if (requestCode != REQ_PERMISSION) return false
        val cb = onPermission ?: return false
        onPermission = null
        cb(grantResults.isNotEmpty() && grantResults.all { it == PackageManager.PERMISSION_GRANTED })
        return true
    }

    @Suppress("DEPRECATION")
    private fun declares(perm: String): Boolean = try {
        ctx.packageManager.getPackageInfo(ctx.packageName, PackageManager.GET_PERMISSIONS).requestedPermissions?.contains(perm) == true
    } catch (e: PackageManager.NameNotFoundException) {
        false
    }

    /** 与 AndroidManifest.xml 里 MiniRenderFileProvider 的 authorities 一致 */
    private fun authority() = "${ctx.packageName}.minirender.files"
}

/** 插件自己的 FileProvider（子类化：宿主自己也声明了 androidx 的 FileProvider 时清单合并不冲突） */
class MiniRenderFileProvider : FileProvider()
