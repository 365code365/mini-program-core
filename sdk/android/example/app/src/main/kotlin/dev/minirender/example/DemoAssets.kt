package dev.minirender.example

import android.content.Context
import android.content.res.AssetManager
import java.io.File

/**
 * 把 APK 里的 `assets/demo/` 解包到 `filesDir/demo/`：引擎按目录读小程序，不读 APK 里的 asset。
 *
 * 只在 APK 换过之后重解（按 `lastUpdateTime` 记一个标记）：每次冷启都整包重写的话，
 * 首帧要多等几百毫秒，也会白白磨闪存。真实业务里这一步是「下载 → 校验 → 解包」，属于宿主的事。
 */
object DemoAssets {
    fun unpack(ctx: Context): File {
        val dst = File(ctx.filesDir, "demo")
        val stamp = File(ctx.filesDir, "demo.stamp")
        val version = ctx.packageManager.getPackageInfo(ctx.packageName, 0).lastUpdateTime.toString()
        if (dst.isDirectory && stamp.isFile && stamp.readText() == version) return dst
        dst.deleteRecursively()
        copy(ctx.assets, "demo", dst)
        stamp.writeText(version)
        return dst
    }

    private fun copy(am: AssetManager, path: String, dst: File) {
        val children = am.list(path).orEmpty()
        if (children.isEmpty()) {
            // 叶子：文件（AssetManager 对空目录与文件都返回空列表，演示包里没有空目录）
            dst.parentFile?.mkdirs()
            am.open(path).use { input -> dst.outputStream().use { input.copyTo(it) } }
            return
        }
        dst.mkdirs()
        for (c in children) copy(am, "$path/$c", File(dst, c))
    }
}
