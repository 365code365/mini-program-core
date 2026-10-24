package dev.minirender

// 原生 SDK（MiniMediaApis）与 Flutter 插件（MediaPicker，直接编这份源码）共用这一份
import android.content.ContentValues
import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Matrix
import android.media.ExifInterface
import android.media.MediaMetadataRetriever
import android.media.MediaScannerConnection
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import android.provider.OpenableColumns
import android.webkit.MimeTypeMap
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.io.InputStream
import java.util.concurrent.atomic.AtomicLong
import kotlin.math.max
import kotlin.math.roundToInt

/**
 * 选到 / 拍到的东西落进引擎的 tempDir（**后台线程**上调）。每个文件回一条
 * `{path: "file://…", size, width, height, duration, thumb, fileType, name, time}`，
 * [MiniMediaApis] 拼成微信的回包形状，引擎把 `file://` 换成 `wxfile://tmp/…`。
 *
 * 图片什么时候原样拷、什么时候重新编码：
 * - 原样：JPEG / PNG / GIF / WebP / BMP，且 EXIF 方向是正的、没要求压缩。GIF 永远原样（重编码就不动了）；
 * - 重新编码：**EXIF 方向不正**（手机竖着拍的照片几乎都是「横着存 + 方向 6」，引擎画图不读 EXIF，
 *   原样交过去在页面里就是躺着的）、HEIC 等引擎解不了的格式、`sizeType` 只要 `compressed`。
 *   压缩档长边 1920、JPEG 质量 80；不压缩的长边封顶 4096、质量 92（再大引擎也只是缩着画，白占内存）。
 *   PNG 不压缩时仍出 PNG（保留透明）。
 */
internal class MediaImport(private val ctx: Context) {
    sealed class Source {
        /** 选择器给的 content:// */
        data class Picked(val uri: Uri) : Source()

        /** 系统相机写好的文件（插件自己的缓存目录，用完挪走） */
        data class Shot(val file: File) : Source()
    }

    companion object {
        private val SEQ = AtomicLong()
        private val DIRECT = setOf("image/jpeg", "image/png", "image/gif", "image/webp", "image/bmp")

        /** tempDir 里的新文件名：`host_` 开头，与引擎自己的 `tmp_…` 错开 */
        fun newFile(dir: File, ext: String): File {
            val e = ext.lowercase().takeIf { it.isNotEmpty() && it.length <= 5 && it.all(Char::isLetterOrDigit) }
            val name = "host_${System.currentTimeMillis()}_${SEQ.incrementAndGet()}" + (e?.let { ".$it" } ?: "")
            return File(dir, name)
        }

        fun uri(f: File) = "file://${f.absolutePath}"
    }

    fun media(sources: List<Source>, dir: File, compressed: Boolean): List<Map<String, Any?>> = sources.map { src ->
        try {
            val mime = mimeOf(src)
            if (mime.startsWith("video/")) video(src, dir, mime) else image(src, dir, mime, compressed)
        } finally {
            if (src is Source.Shot) src.file.delete()
        }
    }

    private fun image(src: Source, dir: File, mime: String, compressed: Boolean): Map<String, Any?> {
        val orientation = try {
            open(src).use { ExifInterface(it).getAttributeInt(ExifInterface.TAG_ORIENTATION, ExifInterface.ORIENTATION_NORMAL) }
        } catch (e: Exception) {
            ExifInterface.ORIENTATION_NORMAL
        }
        val upright = orientation == ExifInterface.ORIENTATION_NORMAL || orientation == ExifInterface.ORIENTATION_UNDEFINED
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        open(src).use { BitmapFactory.decodeStream(it, null, bounds) }
        if (mime == "image/gif" || (mime in DIRECT && upright && !compressed)) {
            val out = newFile(dir, extOf(mime) ?: "jpg")
            copy(src, out)
            return item(out, bounds.outWidth, bounds.outHeight, "image")
        }
        val maxSide = if (compressed) 1920 else 4096
        var sample = 1
        while (max(bounds.outWidth, bounds.outHeight) / (sample * 2) >= maxSide) sample *= 2
        val decoded = open(src).use { BitmapFactory.decodeStream(it, null, BitmapFactory.Options().apply { inSampleSize = sample }) }
            ?: throw IOException("解不开这张图（$mime）")
        var bmp = upright(decoded, orientation)
        val longest = max(bmp.width, bmp.height)
        if (longest > maxSide) {
            val s = maxSide.toFloat() / longest
            bmp = Bitmap.createScaledBitmap(bmp, (bmp.width * s).roundToInt(), (bmp.height * s).roundToInt(), true)
        }
        val png = mime == "image/png" && !compressed
        val out = newFile(dir, if (png) "png" else "jpg")
        FileOutputStream(out).use { bmp.compress(if (png) Bitmap.CompressFormat.PNG else Bitmap.CompressFormat.JPEG, if (compressed) 80 else 92, it) }
        return item(out, bmp.width, bmp.height, "image")
    }

    /** 按 EXIF 方向摆正 */
    private fun upright(b: Bitmap, orientation: Int): Bitmap {
        val m = Matrix()
        when (orientation) {
            ExifInterface.ORIENTATION_ROTATE_90 -> m.postRotate(90f)
            ExifInterface.ORIENTATION_ROTATE_180 -> m.postRotate(180f)
            ExifInterface.ORIENTATION_ROTATE_270 -> m.postRotate(270f)
            ExifInterface.ORIENTATION_FLIP_HORIZONTAL -> m.postScale(-1f, 1f)
            ExifInterface.ORIENTATION_FLIP_VERTICAL -> m.postScale(1f, -1f)
            ExifInterface.ORIENTATION_TRANSPOSE -> { m.postRotate(90f); m.postScale(-1f, 1f) }
            ExifInterface.ORIENTATION_TRANSVERSE -> { m.postRotate(270f); m.postScale(-1f, 1f) }
            else -> return b
        }
        return Bitmap.createBitmap(b, 0, 0, b.width, b.height, m, true)
    }

    private fun video(src: Source, dir: File, mime: String): Map<String, Any?> {
        val out = newFile(dir, extOf(mime) ?: "mp4")
        copy(src, out)
        var w = 0
        var h = 0
        var ms = 0L
        var thumb = ""
        val r = MediaMetadataRetriever()
        try {
            r.setDataSource(out.absolutePath)
            ms = r.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)?.toLongOrNull() ?: 0L
            w = r.extractMetadata(MediaMetadataRetriever.METADATA_KEY_VIDEO_WIDTH)?.toIntOrNull() ?: 0
            h = r.extractMetadata(MediaMetadataRetriever.METADATA_KEY_VIDEO_HEIGHT)?.toIntOrNull() ?: 0
            val rot = r.extractMetadata(MediaMetadataRetriever.METADATA_KEY_VIDEO_ROTATION)?.toIntOrNull() ?: 0
            if (rot == 90 || rot == 270) w = h.also { h = w }
            r.getFrameAtTime(0)?.let { frame ->
                val f = newFile(dir, "jpg")
                FileOutputStream(f).use { frame.compress(Bitmap.CompressFormat.JPEG, 80, it) }
                thumb = uri(f)
            }
        } catch (e: Exception) {
            // 元信息读不出来不算失败：文件已经在了，播放器自己会读
        } finally {
            r.release()
        }
        return item(out, w, h, "video") + mapOf("duration" to ms / 1000.0, "thumb" to thumb)
    }

    /** chooseMessageFile：原样拷，名字取选择器给的显示名。`type`：file = 图片与视频以外的文件（官方语义） */
    fun files(uris: List<Uri>, dir: File, type: String, exts: List<String>): List<Map<String, Any?>> = uris.mapNotNull { u ->
        val name = displayName(u) ?: u.lastPathSegment ?: "file"
        val ext = name.substringAfterLast('.', "").lowercase()
        val mime = ctx.contentResolver.getType(u) ?: mimeOfExt(ext) ?: ""
        val kind = when {
            mime.startsWith("image/") -> "image"
            mime.startsWith("video/") -> "video"
            else -> "file"
        }
        val keep = when (type) {
            "image", "video" -> kind == type
            "file" -> kind == "file" && (exts.isEmpty() || ext in exts)
            else -> true
        }
        if (!keep) return@mapNotNull null
        val out = newFile(dir, ext)
        copy(Source.Picked(u), out)
        item(out, 0, 0, kind) + mapOf("name" to name, "time" to System.currentTimeMillis() / 1000)
    }

    fun saveToAlbum(file: File, kind: String) {
        val video = kind == "video"
        val ext = file.extension.lowercase().ifEmpty { if (video) "mp4" else "jpg" }
        val mime = mimeOfExt(ext) ?: if (video) "video/mp4" else "image/jpeg"
        val name = "mini_${System.currentTimeMillis()}.$ext"
        val folder = if (video) Environment.DIRECTORY_MOVIES else Environment.DIRECTORY_PICTURES
        if (Build.VERSION.SDK_INT >= 29) {
            val cr = ctx.contentResolver
            val coll = if (video) MediaStore.Video.Media.EXTERNAL_CONTENT_URI else MediaStore.Images.Media.EXTERNAL_CONTENT_URI
            val values = ContentValues().apply {
                put(MediaStore.MediaColumns.DISPLAY_NAME, name)
                put(MediaStore.MediaColumns.MIME_TYPE, mime)
                put(MediaStore.MediaColumns.RELATIVE_PATH, folder)
                put(MediaStore.MediaColumns.IS_PENDING, 1)
            }
            val uri = cr.insert(coll, values) ?: throw IOException("相册拒绝写入")
            try {
                (cr.openOutputStream(uri) ?: throw IOException("相册打不开")).use { o -> file.inputStream().use { it.copyTo(o) } }
                values.clear()
                values.put(MediaStore.MediaColumns.IS_PENDING, 0)
                cr.update(uri, values, null, null)
            } catch (e: Exception) {
                cr.delete(uri, null, null)
                throw e
            }
        } else {
            @Suppress("DEPRECATION")
            val out = File(Environment.getExternalStoragePublicDirectory(folder).apply { mkdirs() }, name)
            file.copyTo(out)
            MediaScannerConnection.scanFile(ctx, arrayOf(out.absolutePath), arrayOf(mime), null)
        }
    }

    private fun item(f: File, w: Int, h: Int, kind: String): Map<String, Any?> =
        mapOf("path" to uri(f), "size" to f.length(), "width" to w, "height" to h, "fileType" to kind)

    private fun open(src: Source): InputStream = when (src) {
        is Source.Picked -> ctx.contentResolver.openInputStream(src.uri) ?: throw IOException("打不开 ${src.uri}")
        is Source.Shot -> src.file.inputStream()
    }

    private fun copy(src: Source, out: File) {
        // 相机拍的在同一个分区：直接挪过去
        if (src is Source.Shot && src.file.renameTo(out)) return
        open(src).use { input -> FileOutputStream(out).use { input.copyTo(it) } }
    }

    private fun mimeOf(src: Source): String = when (src) {
        is Source.Picked -> ctx.contentResolver.getType(src.uri) ?: mimeOfExt((displayName(src.uri) ?: "").substringAfterLast('.', "")) ?: "image/jpeg"
        is Source.Shot -> mimeOfExt(src.file.extension) ?: "image/jpeg"
    }

    private fun mimeOfExt(ext: String): String? = MimeTypeMap.getSingleton().getMimeTypeFromExtension(ext.lowercase())

    private fun extOf(mime: String): String? = when (mime) {
        "image/jpeg" -> "jpg"
        else -> MimeTypeMap.getSingleton().getExtensionFromMimeType(mime)
    }

    private fun displayName(u: Uri): String? = try {
        ctx.contentResolver.query(u, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
            if (c.moveToFirst()) c.getString(0) else null
        }
    } catch (e: Exception) {
        null
    }
}
