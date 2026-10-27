package dev.minirender.example

import android.app.Activity
import android.os.Bundle
import android.view.WindowInsets
import android.view.WindowInsetsController
import dev.minirender.MiniGameView
import java.io.File

/**
 * 小游戏示例页：打开 APK 里 `assets/game/<名字>/` 的一个夹具（sample-game/，构建时拷进来）。
 *
 *     adb shell am start -n dev.minirender.example/.GameActivity --es game g-square --es query "max=600"
 *
 * 全屏沉浸（game.json `showStatusBar` 默认 false）；横竖屏由 game.json 决定在 G6 接。
 */
class GameActivity : Activity() {
    private var view: MiniGameView? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val name = intent.getStringExtra("game") ?: "g-square"
        val query = intent.getStringExtra("query") ?: ""
        val dir = GameAssets.unpack(this, name)
        val q = org.json.JSONObject()
        query.split('&').filter { it.contains('=') }.forEach { kv -> q.put(kv.substringBefore('='), kv.substringAfter('=')) }
        val opts = org.json.JSONObject()
            .put("appId", "wxdemo-$name")
            .put("launchOptions", org.json.JSONObject().put("query", q).put("scene", 1001))
        val v = MiniGameView(this, dir.absolutePath, File(filesDir, "game-data").absolutePath, opts.toString())
        // wx.exitMiniProgram 关掉这一页；wx.restartMiniProgram 重建（新实例从 game.js 重新跑）
        v.onHostEvent = { type, _ ->
            when (type) {
                "exit" -> finish()
                "restart" -> recreate()
            }
        }
        view = v
        setContentView(v)
        window.decorView.post {
            window.insetsController?.let {
                it.hide(WindowInsets.Type.statusBars() or WindowInsets.Type.navigationBars())
                it.systemBarsBehavior = WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
            }
        }
    }

    override fun onResume() {
        super.onResume()
        view?.onResume()
    }

    override fun onPause() {
        view?.onPause()
        super.onPause()
    }

    override fun onDestroy() {
        view?.dispose()
        view = null
        super.onDestroy()
    }
}

/** `assets/game/<名字>/` + `assets/game/_lib/` 解包到 `filesDir/game/<名字>/`（_lib 拷进包里，require('_lib/t.js') 按包根解析） */
object GameAssets {
    fun unpack(ctx: android.content.Context, name: String): File {
        val dst = File(ctx.filesDir, "game/$name")
        val stamp = File(ctx.filesDir, "game/$name.stamp")
        val version = ctx.packageManager.getPackageInfo(ctx.packageName, 0).lastUpdateTime.toString()
        if (dst.isDirectory && stamp.isFile && stamp.readText() == version) return dst
        dst.deleteRecursively()
        copy(ctx.assets, "game/$name", dst)
        // 打包时 _lib 改名成 shared-lib（AAPT 丢弃 _ 开头的目录，见 build.gradle.kts），这里还原
        copy(ctx.assets, "game/shared-lib", File(dst, "_lib"))
        stamp.writeText(version)
        return dst
    }

    private fun copy(am: android.content.res.AssetManager, path: String, dst: File) {
        val children = am.list(path).orEmpty()
        if (children.isEmpty()) {
            dst.parentFile?.mkdirs()
            am.open(path).use { input -> dst.outputStream().use { input.copyTo(it) } }
            return
        }
        dst.mkdirs()
        for (c in children) copy(am, "$path/$c", File(dst, c))
    }
}
