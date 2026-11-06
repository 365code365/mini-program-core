package dev.minirender.example

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.widget.FrameLayout
import dev.minirender.MiniProgramView
import org.json.JSONObject
import java.io.File
import kotlin.concurrent.thread

/**
 * 原生集成的最小宿主：整个 Activity 只有一个 [MiniProgramView]，照 doc/原生App集成指南.md 「极简集成」那几行写。
 * 不带参数打开的是 APK 里的内置示例；从 [StoreActivity]（入口页）点开的带 [EXTRA_STORE_APP]，外部 App / H5 的链接
 * 经 [LinkActivity] 带 [EXTRA_LINK] —— 这两种都是「问服务器最新版 → 不是最新就下载 → 打开」（[openStore]）。
 * 已经开着时再来一条链接 / 小程序里 `wx.navigateToMiniProgram`：就地换成另一个小程序（单实例，先关后开）。
 *
 * 验收用的两个 intent 参数（tools/android-native-check.sh 在用）：
 * - `route`：起始页（深链），如 `pages/detail/detail?from=dump`；
 * - `dump`：第一条 `page` 事件之后 2 秒，把视图当前画的内容存成 `files/engine_frame.png`，并在日志打一行
 *   `MINI_DUMP`（视图在屏幕上的位置与尺寸）。与 `adb exec-out screencap` 裁出的同一块逐像素比，
 *   差异为 0 才说明画面没被拉伸、颜色通道没反。
 *
 * 铺满整屏、前后台 / 内存告警 / 销毁、`wx.vibrateShort` 的触感都由 SDK 自己做（MiniProgramView 的
 * autoEdgeToEdge / autoLifecycle / hapticFeedback），宿主只剩返回键一行。
 *
 * 引擎发来的每条事件都打到 logcat（tag `mini-example`），脚本靠 `[事件] page <路由>` 判断进没进对页面。
 */
class MainActivity : Activity() {
    private lateinit var mini: MiniProgramView
    private lateinit var root: FrameLayout
    private val main = Handler(Looper.getMainLooper())

    /** 当前打开着的商店小程序（navigateToMiniProgram 的 referrerInfo.appId）；内置示例时为空 */
    private var currentAppId = ""

    /** 当前打开着的小程序（诊断日志里带上）；内置示例是 {id: demo} */
    private var currentApp: JSONObject = JSONObject().put("id", "demo")

    /** 每开一次商店小程序 +1：后台线程回来时对不上就丢掉（等下载的时候用户又点了别的链接） */
    private var generation = 0

    /** 当前视图已经 open 过（换小程序时要关掉它、换一个新视图） */
    private var opened = false
    private var dumpPending = false
    private var geomLogged = false

    /** 当前（或正在打开）的商店链接：「重新进入小程序」照它再开一次；内置示例为空 */
    private var currentLink: StoreLink? = null

    /**
     * 小程序打开小程序的来路：A → B → C 时是 [A, B]。返回 / 侧滑 / 胶囊「⊙」/ `navigateBackMiniProgram` / `exitMiniProgram`
     * 先回 B、再回 A，最后才退回 App 首页（与微信一致）。一个进程同一时刻只有一个小程序实例，回去是按来路的链接
     * 重新打开（scene 1038，referrerInfo 带回 extraData）：A 的页面栈与内存里的状态不在了，storage 还在。
     * 第二项是离开 A 那一刻的画面：回去时不盖 loading、不问服务器，先盖着它，A 的第一页出来再淡掉
     */
    private val backStack = ArrayDeque<Back>()

    /**
     * 来路上的一个小程序。[parked] 不为空：它没关、挂起着留在内存里（[MiniProgramView.park]，视图藏在 root 里），
     * 回去是 resume —— 页面栈、滚动位置、列表数据、JS 状态原样，没有 loading、没有重排。
     * 挂不上（引擎还没建好）才退回老办法：关掉，回去时按 [link] 重开、先盖 [shot]
     */
    private class Back(val link: StoreLink, val shot: android.graphics.Bitmap?, val parked: Parked?)

    private class Parked(
        val view: MiniProgramView,
        val appId: String,
        val storeApp: MiniStore.App?,
        val app: JSONObject,
        val route: String?,
    )

    /** 来路作废（外部链接进来）：挂着的那些关掉 */
    private fun clearBack() {
        for (b in backStack) b.parked?.let { it.view.close(); root.removeView(it.view) }
        backStack.clear()
    }

    /** 当前打开着的商店小程序（胶囊菜单的转发 / 收藏要用）；内置示例为空 */
    internal var currentStoreApp: MiniStore.App? = null

    /** 当前页面路由（page 事件带的），转发链接带上它 */
    internal var currentRoute: String? = null

    /** 商店点开时的 loading 层（第一条 page 事件到了收起） */
    private var cover: StoreViews.LoadingCover? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        root = FrameLayout(this)
        mini = newMiniView()
        root.addView(mini)
        setContentView(root)
        dumpPending = intent.getBooleanExtra("dump", false)
        val hint = intent.getStringExtra(EXTRA_STORE_APP)?.let { MiniStore.App.fromJson(JSONObject(it)) }
        val link = intent.getStringExtra(EXTRA_LINK)?.let { StoreLink.fromJson(it) }
            ?: hint?.let { StoreLink(it.key, scene = intent.getIntExtra(EXTRA_SCENE, 1001)) }   // 1001：小程序列表点进来
        if (link == null) {
            // 内置示例：验收脚本都走这条（`am start -n …/.MainActivity [--es route …]`）
            mini.open(DemoAssets.unpack(this), intent.getStringExtra("route"))
            opened = true
            return
        }
        openStore(link, hint)
    }

    /** 一个配好宿主能力的视图（换小程序时要新建：一个视图只开一个小程序） */
    private fun newMiniView() = MiniProgramView(this).apply {
        // 入口页上的「诊断日志」开关（测试包里打开，不用 adb setprop）：帧耗时归因、整帧原因、每秒帧率
        if (getSharedPreferences(PREFS, MODE_PRIVATE).getBoolean(PREF_DIAG, false)) {
            diagnostics = setOf("damage_log", "fps")
        }
        // 登录、支付、定位、扫码……要 App 自己接的 wx.*（选图 / 选文件 SDK 自带，不用登记）
        hostApis.putAll(DemoHostApis(this@MainActivity, ::navigateToMiniProgram, { backToReferrer(it) }, { backStack.isNotEmpty() }).all)
        // 胶囊「⊙」：有来路回上一个小程序，没有就关掉这一页
        onClose = { if (!backToReferrer(null)) finish() }
        // <map> / <web-view> / <live-player> 用 SDK 自带的（MiniMapView 瓦片地图同层渲染、系统 WebView、系统播放器），不用登记
        // 胶囊「•••」：微信式底部菜单（转发、收藏 / 重新进入、上传诊断日志、关于），见 MenuSheet.kt
        onAbout = { a -> showCapsuleMenu(a) }
        onEvent = { ev ->
            // nativeViewTouch：按在地图上的每一段触摸都是一条，打出来就是刷屏（还占主线程）
            if (ev.optString("type") != "nativeViewTouch") Log.i(TAG, "[事件] " + summarize(ev))
            when (ev.optString("type")) {
                // 第一页出来了才收起 loading（冷启动加载字体那几百毫秒也盖在底下，不露白屏）
                "page" -> cover?.let { cover = null; it.hide() }
                // wx.exitMiniProgram：与栈底返回同一条路
                "exitMiniProgram" -> main.post { if (!backToReferrer(null)) finish() }
            }
        }
    }

    /**
     * 打开商店里的小程序（[link] 的 key 是 AppID 或 id）：盖 loading → 后台问服务器最新版、
     * 不是最新就下载解包 → 回主线程 open。已经开着一个时先关掉它（同步销毁引擎），再在新视图上开。
     * [resume]：回到上一个小程序（[backToReferrer]）—— 用本地那一版不问服务器，盖 [shot] 而不是 loading
     */
    private fun openStore(link: StoreLink, hint: MiniStore.App? = null, resume: Boolean = false, shot: android.graphics.Bitmap? = null) {
        val gen = ++generation
        currentLink = link
        if (opened) {
            // close 返回时旧引擎已经放手 Surface；实例在它的线程上销毁，新视图的引擎建之前会等它（不会撞上「单实例」）
            opened = false
            currentAppId = ""
            currentStoreApp = null
            mini.close()
            root.removeView(mini)
            mini = newMiniView()
            root.addView(mini, 0)
        }
        cover?.let { root.removeView(it) }
        val store = MiniStore(this, intent.getStringExtra(EXTRA_STORE_URL) ?: MiniStore.DEFAULT_URL)
        // 深链只带 AppID：装过的话先用本地记录的名字 / 图标画 loading（读一个小 JSON，不等网络）
        val known = hint ?: store.local(link.key)
        val c = StoreViews.LoadingCover(this, known?.id ?: link.key, known?.name ?: "小程序", resume, shot) { finish() }
        root.addView(c)
        cover = c
        thread(name = "store-prepare") {
            try {
                var stage = ""
                val onStatus: (String) -> Unit = { s ->
                    stage = s
                    runOnUiThread { if (gen == generation) c.progress(s) }
                }
                val onProgress: (Long, Long) -> Unit = { got, total ->
                    val mb = if (total > 0) "%.1f / %.1f MB".format(got / 1048576.0, total / 1048576.0)
                    else "%.1f MB".format(got / 1048576.0)
                    runOnUiThread { if (gen == generation) c.progress("$stage  $mb") }
                }
                val p = if (resume) store.prepareLocal(link.key, onStatus, onProgress) else store.prepare(link.key, onStatus, onProgress)
                runOnUiThread {
                    if (isDestroyed || gen != generation) return@runOnUiThread
                    val why = p.stale?.let { "连不上服务器，用本地版（${it.message}）" }
                        ?: if (resume && !p.updated) "回到上一个小程序，用本地版（不问服务器，盖${if (shot == null) "白底" else "离开时的截图"}）"
                        else if (p.updated) "已更新" else "已是最新"
                    Log.i(TAG, "[商店] 打开 ${p.app.id} ${p.app.appId} v${p.app.version} $why → ${p.dir} 起始页 ${link.route ?: "首页"}")
                    // 小游戏不走 MiniProgramView：交给 GameActivity（整屏主画布、按 game.json 锁方向），这一页让出去
                    if (p.app.isGame) {
                        // 字节码在后台编，与 GameActivity 起来、建实例并行（实例碰到正在编的脚本会等）
                        dev.minirender.MiniGame.precompileAsync(p.dir.absolutePath, java.io.File(filesDir, "game-data").absolutePath)
                        startActivity(GameActivity.storeIntent(this, p.dir, p.app, link.launchOptions()))
                        @Suppress("DEPRECATION")
                        overridePendingTransition(0, 0)
                        super.finish()
                        return@runOnUiThread
                    }
                    currentAppId = p.app.appId
                    currentStoreApp = p.app
                    currentApp = JSONObject(p.app.toJson()).put("route", link.route ?: JSONObject.NULL)
                        .put("stale", p.stale?.message ?: JSONObject.NULL)
                    c.identity(p.app.id, p.app.name)
                    mini.launchOptions = link.launchOptions()
                    mini.open(p.dir, link.route)
                    opened = true
                }
            } catch (e: Exception) {
                Log.e(TAG, "[商店] ${link.key} 打不开", e)
                runOnUiThread {
                    if (!isDestroyed && gen == generation) {
                        c.error("打不开：${e.message ?: e.javaClass.simpleName}") { openStore(link, hint) }
                    }
                }
            }
        }
    }

    /** 胶囊菜单「重新进入小程序」：商店的走 [openStore]（先关后开）；内置示例同样换一个新视图再开 */
    internal fun relaunch() {
        currentLink?.let { return openStore(it, currentStoreApp) }
        mini.close()
        root.removeView(mini)
        mini = newMiniView()
        root.addView(mini, 0)
        mini.open(DemoAssets.unpack(this), intent.getStringExtra("route"))
    }

    internal fun uploadDiag() {
        val backend = "${if (mini.activeBackend == dev.minirender.MiniEngine.BACKEND_GPU) "gpu" else "cpu"}：${mini.backendReason}"
        DiagUpload.ask(this, intent.getStringExtra(EXTRA_STORE_URL) ?: MiniStore.DEFAULT_URL, currentApp, backend)
    }

    /**
     * 小程序里的 `wx.navigateToMiniProgram`（用户已点允许）：按 AppID 换过去，scene 1037 + referrerInfo。
     * 与微信的差别：微信把目标小程序叠在上面、`navigateBackMiniProgram` 能回来；这里是**替换**（单实例）
     */
    private fun navigateToMiniProgram(args: JSONObject) {
        val appId = args.optString("appId")
        val parsed = StoreLink.parse(
            "minirender://open?" + android.net.Uri.Builder()
                .appendQueryParameter("appid", appId).appendQueryParameter("path", args.optString("path")).build().encodedQuery,
        ) ?: return
        val referrer = JSONObject().put("appId", currentAppId)
        args.opt("extraData")?.takeIf { it != JSONObject.NULL }?.let { referrer.put("extraData", it) }
        // 记下来路：B 里返回要回到这一个（A）。A 不关，挂起留在内存里（与微信一致），B 开在一个新视图上
        val from = currentLink
        val gen = generation
        val a = mini
        val t0 = android.os.SystemClock.uptimeMillis()
        a.park { ok ->
            // 挂起的这几毫秒里又换了别的（外部链接）：这次跳转作废
            if (isDestroyed || gen != generation) {
                if (ok) a.unpark()
                return@park
            }
            Log.i(TAG, "[商店] 挂起 ${currentStoreApp?.id ?: "?"} ${if (ok) "ok" else "没挂上，改为关掉"} ${android.os.SystemClock.uptimeMillis() - t0}ms")
            if (!ok) {
                // 挂不上（引擎还没建好）：老办法，截一张、关掉，回来时重开
                a.snapshot { shot ->
                    if (isDestroyed || gen != generation) return@snapshot
                    from?.let { backStack.addLast(Back(it, shot, null)) }
                    openStore(parsed.copy(scene = 1037, referrer = referrer))
                }
                return@park
            }
            from?.let { backStack.addLast(Back(it, null, Parked(a, currentAppId, currentStoreApp, currentApp, currentRoute))) }
            // A 的视图藏起来（Surface 没了引擎照样活着），B 用一个新视图；openStore 看 opened = false 不会去关 A
            a.visibility = android.view.View.GONE
            mini = newMiniView()
            root.addView(mini, 0)
            opened = false
            openStore(parsed.copy(scene = 1037, referrer = referrer))
        }
    }

    /**
     * 回到打开当前小程序的那个小程序（scene 1038，referrerInfo 带 [extra]）。没有来路返回 false，调用方照常退回 App。
     * 不走 loading：用本地那一版（不问服务器），先盖离开它时截的画面，它的第一页出来再淡掉。
     * A 是重开的：页面栈、滚动位置、内存里的状态不在（重开到 A 被打开时的那一页），storage 还在
     */
    private fun backToReferrer(extra: Any?): Boolean {
        val back = backStack.removeLastOrNull() ?: return false
        val referrer = JSONObject().put("appId", currentAppId)
        extra?.let { referrer.put("extraData", it) }
        val p = back.parked ?: run {
            openStore(back.link.copy(scene = 1038, referrer = referrer), resume = true, shot = back.shot)
            return true
        }
        // A 挂着：关掉 B、A 原样回来。顺序要紧 —— 先 resume（引擎线程上等 B 销毁完、还原状态、派发 onShow），
        // 再显示 A：SurfaceHolder.Callback2 让窗口等 A 的 Surface 交出第一帧再一起上屏，中间没有空白帧
        val t0 = android.os.SystemClock.uptimeMillis()
        generation++ // B 还在下载 / 准备的话，那边回来时作废
        cover?.let { root.removeView(it) }
        cover = null
        val b = mini
        b.close()
        root.removeView(b)
        mini = p.view
        opened = true
        currentLink = back.link
        currentAppId = p.appId
        currentStoreApp = p.storeApp
        currentApp = p.app
        currentRoute = p.route
        mini.unpark(JSONObject().put("path", p.route ?: "").put("query", JSONObject()).put("scene", 1038).put("referrerInfo", referrer))
        mini.visibility = android.view.View.VISIBLE
        Log.i(TAG, "[商店] 回到 ${p.storeApp?.id ?: "?"}（挂起的，原样恢复）${android.os.SystemClock.uptimeMillis() - t0}ms")
        return true
    }

    private fun summarize(e: JSONObject): String {
        val type = e.optString("type")
        if (type == "page") {
            // 视图几何（设备像素）：验收脚本按它把逻辑坐标换成点击位置、裁掉系统画的状态栏 / 导航条
            if (!geomLogged) {
                geomLogged = true
                Log.i(TAG, "MINI_GEOM " + geometry())
            }
            // 第一条 page 事件之后再等 2 秒存帧（冷启动要加载字体）
            if (dumpPending) {
                dumpPending = false
                main.postDelayed(::dump, 2000)
            }
            val title = e.optJSONObject("window")?.optString("navigationBarTitleText")
            currentRoute = e.optString("route").ifEmpty { null }
            return "page ${e.optString("route")} depth=${e.optInt("depth")} title=$title"
        }
        val copy = JSONObject(e.toString()).apply { remove("type") }
        return "$type $copy"
    }

    /**
     * 引擎当前这一帧 → PNG（引擎像素尺寸，在视图里 1:1 摆在左上角）。画面在 SurfaceView 上，
     * `View.draw(canvas)` 截不到（截出来是背景色），所以走 [MiniProgramView.dumpFrame]
     */
    private fun dump() {
        val out = File(getExternalFilesDir(null), "engine_frame.png")
        // w / h 报画面（SurfaceView）的尺寸 = 引擎像素尺寸：视图宽 1080、引擎 1079 时画面只占左边 1079 列
        mini.dumpFrame(out) { ok ->
            Log.i(TAG, "MINI_DUMP ok=$ok " + geometry(mini.pixelWidth, mini.pixelHeight) + " backend=${mini.activeBackend}")
        }
    }

    @Suppress("DEPRECATION")
    private fun geometry(w: Int = mini.width, h: Int = mini.height): String {
        val loc = IntArray(2).also { mini.getLocationOnScreen(it) }
        val i = mini.rootWindowInsets
        return "x=${loc[0]} y=${loc[1]} w=$w h=$h " +
            "statusBar=${i?.systemWindowInsetTop ?: 0} bottom=${i?.systemWindowInsetBottom ?: 0} " +
            "density=${resources.displayMetrics.density}"
    }

    /** 从入口页点开的（打开时是上推）：关掉时往下退，与打开成对 */
    override fun finish() {
        super.finish()
        if (intent.hasExtra(EXTRA_STORE_APP) || intent.hasExtra(EXTRA_LINK)) {
            @Suppress("DEPRECATION")
            overridePendingTransition(0, R.anim.store_slide_down)
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        // 开着的时候又来一条外部链接（LinkActivity 用 CLEAR_TOP | SINGLE_TOP 送到这里）：就地换小程序，来路作废
        intent.getStringExtra(EXTRA_LINK)?.let { clearBack(); return openStore(StoreLink.fromJson(it)) }
        intent.getStringExtra("route")?.let { mini.navigate("/$it") }
    }

    @Deprecated("Activity.onBackPressed：minSdk 24 上没有 OnBackInvokedCallback，两条都要能走")
    override fun onBackPressed() {
        // 页面栈空了：是被别的小程序打开的就回那个小程序，否则退出
        if (!mini.goBack() && !backToReferrer(null)) super.onBackPressed()
    }

    companion object {
        const val TAG = "mini-example"

        /** [StoreActivity] 点开的小程序（[MiniStore.App.toJson]）；不给就打开内置示例 */
        const val EXTRA_STORE_APP = "storeApp"
        const val EXTRA_STORE_URL = "storeUrl"
        /** 打开它的入口（微信场景值）：1001 列表、1005 搜索、1089 下拉最近使用、1104 下拉我的小程序 */
        const val EXTRA_SCENE = "scene"
        /** 入口页「诊断日志」开关存在这里（[StoreActivity.diagRow]） */
        const val PREFS = "example"
        const val PREF_DIAG = "diag"

        /** [LinkActivity] 解析好的外部链接（[StoreLink.toJson]） */
        const val EXTRA_LINK = "storeLink"
    }
}
