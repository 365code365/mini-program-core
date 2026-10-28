package dev.minirender.example

import android.app.Activity
import android.os.Bundle
import android.view.WindowInsets
import android.view.WindowInsetsController
import dev.minirender.MiniGameView
import java.io.File

/**
 * 小游戏示例页：打开 APK 里 `assets/game/<名字>/` 的一个夹具（sample-game/，构建时拷进来），
 * 或商店下载解好的小游戏目录（[storeIntent]：MainActivity 准备好包之后转过来）。
 *
 *     adb shell am start -n dev.minirender.example/.GameActivity --es game g-square --es query "max=600"
 *
 * 全屏沉浸（game.json `showStatusBar` 默认 false）；横竖屏由 game.json 决定在 G6 接。
 */
class GameActivity : Activity() {
    private var view: MiniGameView? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val storeDir = intent.getStringExtra(EXTRA_DIR)
        val name = intent.getStringExtra("game") ?: "g-square"
        val dir = storeDir?.let { File(it) } ?: GameAssets.unpack(this, name)
        val launch = intent.getStringExtra(EXTRA_LAUNCH)?.let { org.json.JSONObject(it) } ?: run {
            val q = org.json.JSONObject()
            (intent.getStringExtra("query") ?: "").split('&').filter { it.contains('=') }
                .forEach { kv -> q.put(kv.substringBefore('='), kv.substringAfter('=')) }
            org.json.JSONObject().put("query", q).put("scene", 1001)
        }
        val opts = org.json.JSONObject()
            // 存档按 appId 分：商店的用真 AppID（换了解包目录也不丢档），夹具用 wxdemo-<名字>
            .put("appId", intent.getStringExtra(EXTRA_APP_ID) ?: "wxdemo-$name")
            .put("launchOptions", launch)
            // --ez rotate true：横屏包在竖屏里由引擎自己转（G6.4）
            .put("rotateInEngine", intent.getBooleanExtra("rotate", false))
            // 示例宿主接的能力：支付、激励视频、分享、登录（GameHostApis：演示实现，正式接入换成真 SDK）
            .put("hostApis", org.json.JSONArray(GameHostApis.APIS))
        val v = MiniGameView(this, dir.absolutePath, File(filesDir, "game-data").absolutePath, opts.toString())
        // wx.exitMiniProgram 关掉这一页；wx.restartMiniProgram 重建（新实例从 game.js 重新跑）
        val hosts = GameHostApis(this) { view }
        v.onHostEvent = { type, ev ->
            when (type) {
                "exit" -> finish()
                "restart" -> recreate()
                "hostCall" -> if (!hosts.handle(ev)) v.hostReply(ev.optInt("id"), false, org.json.JSONObject().put("errMsg", ev.optString("api") + ":fail 示例宿主未实现").toString())
            }
        }
        view = v
        // loading 层盖到第一帧上屏：MainActivity 的 loading 页在转过来时就 finish 了，建实例 + 跑 game.js + 第一帧
        // 这 0.5s 上下 SurfaceView 是空的、透出黑底（「打开游戏黑一下」）。与商店 loading 同一个样子，接上看不出换了页
        val root = android.widget.FrameLayout(this)
        root.addView(v)
        val cover = StoreViews.LoadingCover(this, intent.getStringExtra(EXTRA_ICON_ID) ?: name, intent.getStringExtra(EXTRA_NAME) ?: name) { finish() }
        root.addView(cover)
        var covered = true
        val reveal = { if (covered) { covered = false; cover.hide() } }
        v.onFirstFrame = reveal
        // 兜底：游戏一直不出帧（脚本报错、卡在加载）也别让 loading 永远盖着
        root.postDelayed(reveal, 10_000)
        setContentView(root)
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

    companion object {
        const val EXTRA_DIR = "dir"
        const val EXTRA_APP_ID = "appId"
        const val EXTRA_LAUNCH = "launchOptions"
        /** loading 层的图标（商店 id）与名字 */
        const val EXTRA_ICON_ID = "iconId"
        const val EXTRA_NAME = "name"

        /** 商店里的小游戏：[dir] 是解好的包（含 game.json），[launch] 是 StoreLink 的 launchOptions（scene / query / referrerInfo） */
        fun storeIntent(ctx: android.content.Context, dir: File, app: MiniStore.App, launch: org.json.JSONObject) =
            android.content.Intent(ctx, GameActivity::class.java)
                .putExtra(EXTRA_DIR, dir.absolutePath).putExtra(EXTRA_APP_ID, app.key).putExtra(EXTRA_LAUNCH, launch.toString())
                .putExtra(EXTRA_ICON_ID, app.id).putExtra(EXTRA_NAME, app.name)
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
