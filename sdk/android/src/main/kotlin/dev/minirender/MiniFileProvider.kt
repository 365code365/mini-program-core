package dev.minirender

import android.content.ContentProvider
import android.content.ContentValues
import android.content.Context
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import android.webkit.MimeTypeMap
import java.io.File
import java.io.FileNotFoundException

/**
 * 把 App 自己的文件交给别的应用（系统相机写拍好的照片、`openDocument` 让别的应用打开）用的 `content://`。
 *
 * 为什么不用 androidx 的 FileProvider：SDK 不带任何 androidx 依赖（宿主不一定用 androidx），
 * 要的只有「按授权读 / 写缓存目录与 files 目录里的一个文件」这一件事。
 *
 * - `content://<包名>.minirender.sdkfiles/cache/<相对路径>` → `cacheDir`
 * - `content://<包名>.minirender.sdkfiles/files/<相对路径>` → `filesDir`（引擎的 `wxfile://tmp` 在这下面）
 *
 * 清单里 `exported="false"` + `grantUriPermissions="true"`：别的应用只能碰到这边**按次授权**给它的那一个 URI。
 * 路径先规范化再判是否还在根目录里，`..` 走不出去。
 * Flutter 插件用的是自己的 `MiniRenderFileProvider`（authority `.minirender.files`），两边不冲突。
 */
class MiniFileProvider : ContentProvider() {
    companion object {
        fun authority(ctx: Context) = "${ctx.packageName}.minirender.sdkfiles"

        /** 文件 → 可以授权给别的应用的 URI。不在 cacheDir / filesDir 下面的抛 IllegalArgumentException */
        fun uriFor(ctx: Context, file: File): Uri {
            val path = file.canonicalPath
            for ((name, root) in roots(ctx)) {
                val base = root.canonicalPath
                if (path.startsWith("$base/")) {
                    return Uri.Builder().scheme("content").authority(authority(ctx))
                        .appendPath(name).appendEncodedPath(Uri.encode(path.substring(base.length + 1), "/")).build()
                }
            }
            throw IllegalArgumentException("$path 不在 cacheDir / filesDir 下面")
        }

        private fun roots(ctx: Context) = listOf("cache" to ctx.cacheDir, "files" to ctx.filesDir)
    }

    override fun onCreate() = true

    private fun fileOf(uri: Uri): File {
        val ctx = context ?: throw FileNotFoundException("没有 Context")
        val segs = uri.pathSegments
        val root = roots(ctx).firstOrNull { it.first == segs.firstOrNull() }?.second
            ?: throw FileNotFoundException("不认识的根：$uri")
        val f = File(root, segs.drop(1).joinToString("/")).canonicalFile
        if (!f.path.startsWith(root.canonicalPath + "/")) throw FileNotFoundException("走出了根目录：$uri")
        return f
    }

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor =
        ParcelFileDescriptor.open(fileOf(uri), ParcelFileDescriptor.parseMode(mode))

    override fun getType(uri: Uri): String =
        MimeTypeMap.getSingleton().getMimeTypeFromExtension(fileOf(uri).extension.lowercase()) ?: "application/octet-stream"

    /** 别的应用问显示名 / 大小（`OpenableColumns`）时回答；别的列没有 */
    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, args: Array<out String>?, sort: String?): Cursor {
        val f = fileOf(uri)
        val cols = projection ?: arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE)
        val known = cols.filter { it == OpenableColumns.DISPLAY_NAME || it == OpenableColumns.SIZE }.toTypedArray()
        return MatrixCursor(known, 1).apply {
            addRow(known.map { if (it == OpenableColumns.DISPLAY_NAME) f.name else f.length() })
        }
    }

    override fun insert(uri: Uri, values: ContentValues?): Uri? = null
    override fun delete(uri: Uri, selection: String?, args: Array<out String>?) = 0
    override fun update(uri: Uri, values: ContentValues?, selection: String?, args: Array<out String>?) = 0
}
