package dev.minirender

import android.app.Activity
import android.app.ActivityManager
import android.app.AlertDialog
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.ContextWrapper
import android.os.Build
import android.util.AttributeSet
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.WindowInsets
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.widget.FrameLayout
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

/**
 * 小程序视图 —— 集成方**只需要用这一个类**。
 *
 * ```kotlin
 * val view = MiniProgramView(this)
 * setContentView(view)
 * view.open(File(filesDir, "my-mini-app"))      // 解包后的小程序目录
 *
 * override fun onBackPressed() {                // 返回键交给小程序
 *     if (!view.goBack()) super.onBackPressed() // 页面栈空了才退出
 * }
 * ```
 *
 * ## 它替你做了什么
 * - 起一条**专用渲染线程**（引擎不是线程安全的，所有调用都排到这条线程上），出帧跟着 vsync 走；
 * - 画面画在自己的一块 **SurfaceView** 上：引擎线程直接交给 SurfaceFlinger 合成，不经过 UI 线程的
 *   `onDraw`，主线程忙不忙与滑动无关（为什么要这样见 doc/滑动顿挫避坑清单.md）；
 * - 后端默认 **auto**：机型够格就 GPU 合成（滚动帧只合成不重画），否则 / 失败时回退 CPU，见 [backend]；
 * - 按 `View` 的实际尺寸与 `density` 算逻辑尺寸和 dpr；
 * - `onTouchEvent` → 引擎的**指针三段**（含 `getHistoricalX/Y` 的历史点与事件时间，不抽稀）；
 * - 只在引擎还有活时才出帧，静止时不出帧（不耗电）；
 * - 生命周期与内存告警转发；`adb shell setprop debug.mini.<开关>` 的诊断开关（[MiniDiag]）。
 *
 * ## 为什么触摸要送三段而不是"点击"
 * 引擎内部有触摸状态机，`touchstart/touchmove/touchend/longpress/tap` 都由它产出。
 * 宿主自己合成"点击"会绕过状态机 —— 那类 bug 在自动化测试里全绿、真机上却点不动。
 *
 * ## 右上角胶囊
 * `navigationStyle: custom` 的页面由引擎画胶囊（位置 = `wx.getMenuButtonBoundingClientRect()`，
 * 所以视图要铺到状态栏下面，窗口 insets 会自动交给引擎）。两个口子留给宿主：
 * ```kotlin
 * view.onAbout = { about -> /* 「•••」查看关于：打开自己的关于页 */ }
 * view.onClose = { /* 「⊙」返回宿主：finish() / 弹回上一个 Fragment */ }
 * ```
 * 不设时的默认：`onAbout` 弹一个展示小程序信息的 `AlertDialog`，`onClose` 结束所在的 Activity。
 * 普通页面的导航栏默认也是引擎画的（[navigationBar]，胶囊在里面）；关掉之后导航栏与胶囊由宿主自己画。
 *
 * ## SurfaceView 的约束
 * 画面在独立图层上：`View.draw(canvas)` 截不到它（截帧用 [dumpFrame]）；Android 10 以下 SurfaceView
 * 不跟父容器的 alpha / 变换动画走。第一帧交出来之前它是藏着的，露出的是这个视图的背景色。
 */
class MiniProgramView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
) : FrameLayout(context, attrs) {

    private val session = EngineSession(context.applicationContext, object : EngineSession.Ui {
        override fun onPixelSize(w: Int, h: Int) = setPixelSize(w, h)
        override fun reveal() = revealSurface()
        override fun newSurface() = newSurfaceView()
        override fun onEvents(list: List<String>) = list.forEach(::dispatchEvent)
    })
    private var appDir: File? = null
    private var startRoute: String? = null
    private var created = false
    private val hostCalls = HostCallRouter(context, session)
    /** 软键盘：输入框聚焦弹出、输入法的字交给引擎（MiniImeConnection.kt） */
    private val ime = MiniIme(this, ::sendText, ::sendKey)
    private val sameLayer = SameLayerViews(session) { resources.displayMetrics.density }

    // ── SurfaceView（只在主线程读写） ──
    private var surfaceView: SurfaceView? = null
    private var fixed: Pair<Int, Int>? = null
    private var surfaceRevealed = false

    /** 引擎在 custom 导航栏的页面上画右上角胶囊（`open` 之前设） */
    var menuButton = true

    /**
     * 导航栏由引擎画（默认，与 Flutter 插件一致；`open` 之前设）：画面铺满整屏、顶到状态栏，普通页面顶上是
     * 引擎画的导航栏（标题、返回箭头、胶囊、`wx.setNavigationBar*`）。关掉的话普通页面从状态栏下面直接开始，
     * 导航栏得宿主自己画（从前的默认 —— 原生示例因此整页顶到状态栏里、也没有返回箭头）
     */
    var navigationBar = true

    /** 页面转场动画（默认开，与 Flutter 插件一致；`open` 之前设）：navigateTo 从右边滑入、返回向右滑出，300ms */
    var routeAnimation = true

    /**
     * `<video>` 走同层渲染（默认开，与 Flutter 插件一致；`open` 之前设）：SDK 自带的系统播放器（MediaPlayer，硬解、
     * 带声音）解码，帧交给引擎画进页面，滚动时与页面一起动；控制条、全屏（横 / 竖）由引擎画。
     * false = 引擎自己软解（openh264，手机上没有声音、耗 CPU）
     */
    var sameLayerVideo = true

    /** 视频全屏时收起状态栏与导航条（默认开；[onEvent] 里也能收到 `videoFullscreen` 自己处理） */
    var immersiveVideoFullscreen = true

    /**
     * 渲染后端偏好（`open` 之前设）：[MiniEngine.BACKEND_AUTO]（默认：机型够格走 GPU，不够格 / 初始化失败 /
     * 运行中丢设备一律回退 CPU）、[MiniEngine.BACKEND_CPU]（强制软件光栅化）、[MiniEngine.BACKEND_GPU]。
     * 调试时也可以 `adb shell setprop debug.mini.backend cpu` 强制，实际用的是哪个看 [activeBackend]
     */
    var backend = MiniEngine.BACKEND_AUTO

    /** 实际后端（[MiniEngine.BACKEND_CPU] / [MiniEngine.BACKEND_GPU]），挂上 Surface 之后才定 */
    val activeBackend: Int get() = session.activeBackend

    /** 为什么是这个后端（机型分级 / 自检结果 / 回退原因），logcat 里 `🎨 GPU 上屏：…` 也是这一句 */
    val backendReason: String get() = session.backendReason

    /** 引擎画面的像素尺寸（= SurfaceView 的缓冲区与布局尺寸，摆在视图左上角）；引擎建好之前是 0 */
    val pixelWidth: Int get() = session.pixelW
    val pixelHeight: Int get() = session.pixelH

    /**
     * 宿主实现的 `wx.*`（`open` 之前登记）：登录、支付、定位、扫码……引擎做不了的那些。登记了名字的 API
     * 调用时交给它（主线程），回包走 [MiniHostReply]；没登记的照旧立刻 `fail`（errMsg 写明「宿主可接管」）。
     * ```kotlin
     * view.hostApis["login"] = MiniHostApi { call, reply -> reply.success(JSONObject().put("code", myCode())) }
     * ```
     * 与 [builtinHostApis] 同名时以这里为准。
     */
    val hostApis: MutableMap<String, MiniHostApi> get() = hostCalls.apis

    /**
     * SDK 自带的宿主能力（默认开，`open` 之前设）：`chooseImage` / `chooseMedia` / `chooseVideo` / `chooseMessageFile` /
     * `saveImageToPhotosAlbum` / `saveVideoToPhotosAlbum` / `openDocument`，走系统的照片选择器、相机、文件选择器
     * （透明的 [MiniMediaActivity] 代办，宿主不用转发 `onActivityResult`）。与 Flutter 插件的 `builtinHostApis` 同一组。
     * 清单要有 [MiniMediaActivity] 与 [MiniFileProvider]（AAR 自带；直接编源码的照 `sdk/android/example` 抄）
     */
    var builtinHostApis: Boolean
        get() = hostCalls.builtin
        set(v) {
            hostCalls.builtin = v
        }

    /**
     * 同层渲染的原生组件（`open` 之前登记）：标签 → 工厂。登记了的标签（如 `map`）由宿主**画**
     * （[MiniSameLayerView.onDraw]），引擎把画好的那一帧画进页面 —— 滚动时与页面在同一张图里走，
     * 不像盖在画面上的控件那样一滑就错开、一顿一顿。点击转给 [MiniSameLayerView.onTap]。
     * ```kotlin
     * view.sameLayerViews["map"] = { MyMapView() }
     * ```
     */
    val sameLayerViews: MutableMap<String, () -> MiniSameLayerView> get() = sameLayer.factories

    /** 胶囊「•••」：查看关于（主线程回调）。null = 默认弹 AlertDialog */
    var onAbout: ((MiniProgramAbout) -> Unit)? = null

    /** 胶囊「⊙」：返回宿主（主线程回调）。null = 默认结束所在的 Activity */
    var onClose: (() -> Unit)? = null

    /** 引擎发来的每一条事件（主线程回调；`page` / `navigationBar` / `inputFocus` / `app` / `menuButton` …） */
    var onEvent: ((JSONObject) -> Unit)? = null

    /** 小程序的关于信息（启动后第一帧到；主线程读） */
    var about: MiniProgramAbout? = null
        private set

    /** 打开一个解包后的小程序目录。可指定起始页（深链）。一个视图只开一个小程序 */
    fun open(dir: File, route: String? = null) {
        appDir = dir
        startRoute = route
        if (width > 0 && height > 0) createEngine()
    }

    /** 跳到某个页面（`/pages/x/x?id=1`） */
    fun navigate(url: String) = session.run { MiniEngine.nativeNavigate(it, url) }

    /**
     * 返回键。**返回 false 表示页面栈只剩一页**，宿主该退出小程序。
     *
     * 注意这是同步等结果的（要立刻决定 `finish()` 还是不动），所以内部做了一次
     * 跨线程等待；返回键是低频操作，代价可以忽略。
     */
    fun goBack(): Boolean = session.call(false) { MiniEngine.nativeBack(it) }

    fun onHostResume() {
        session.lifecycle(0)
    }

    /** 进后台：派发 `App.onHide` 并**停止出帧**（继续出帧纯属耗电） */
    fun onHostPause() {
        session.lifecycle(1)
    }

    /** 对应 Activity/Application 的 `onTrimMemory` */
    fun onHostTrimMemory() = session.lifecycle(2)

    fun close() {
        session.dispose()
    }

    /**
     * 把引擎当前这一帧存成 PNG（引擎像素尺寸，RGBA 原样）。画面在 SurfaceView 上，`View.draw` 截不到，
     * 截帧对比 / 自动化验收用这个。[done] 在主线程回调
     */
    fun dumpFrame(file: File, done: (Boolean) -> Unit) {
        session.dumpFrame(file, done)
    }

    /** 软键盘提交的文字（宿主可以自己接 InputConnection，简单场景直接调这个） */
    fun sendText(text: String) {
        session.run { MiniEngine.nativeTextInput(it, text) }
    }

    /**
     * 功能键：enter / newline / backspace / delete / left / right / up / down / home / end、
     * shift+… 扩展选区、word-left / word-right、doc-start / doc-end、delete-word-back、
     * selectall / copy / cut / undo / redo / escape。回车也可以 sendText("\n")（textarea 换行）
     */
    fun sendKey(name: String) {
        session.run { MiniEngine.nativeKey(it, name) }
    }

    // ─────────────────────────── 内部 ───────────────────────────

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        if (!created) {
            createEngine()
            return
        }
        // 转屏 / 分屏：引擎重排，SurfaceView 跟着引擎的新像素尺寸改（EngineSession.resize → setPixelSize）
        val dpr = resources.displayMetrics.density
        session.resize((w / dpr).toInt().coerceAtLeast(1), (h / dpr).toInt().coerceAtLeast(1), dpr)
    }

    private fun createEngine() {
        val dir = appDir ?: return
        if (created || width <= 0 || height <= 0) return
        created = true
        val dpr = resources.displayMetrics.density
        // 逻辑尺寸 = 像素 / density（引擎按逻辑坐标工作，rpx 由它自己换算）
        val lw = (width / dpr).toInt().coerceAtLeast(1)
        val lh = (height / dpr).toInt().coerceAtLeast(1)
        // 数据目录必须给：storage 与图片磁盘缓存都落在这里，不给会静默失效
        val data = File(context.filesDir, "mini-render").apply { mkdirs() }
        // 选项要在 app.js 执行前就位：onLaunch 里读 getSystemInfoSync（状态栏高度、胶囊位置）是常态
        val options = JSONObject()
            .put("systemInfo", JSONObject()
                .put("platform", "android")
                .put("brand", Build.BRAND)
                .put("model", Build.MODEL)
                .put("system", "Android ${Build.VERSION.RELEASE}"))
            .put("menuButton", menuButton)
            .put("navigationBar", navigationBar)
            .put("routeAnimation", routeAnimation)
            // 点了名的 wx.* 才交给宿主（hostCall 事件）；没点名的照旧立刻 fail
            .put("hostApis", JSONArray(hostCalls.names()))
        // 同层渲染：宿主画 / 解码，引擎画进页面，滚动时与页面在同一张图里走。
        // - 视频：帧由 VideoHost 交（EngineSession 接 `videos` / `videoCommand`）；
        // - 登记了的组件（[sameLayerViews]，如 map）：宿主画（`sameLayerViews` / `nativeViewTap` 事件）
        val same = JSONArray()
        if (sameLayerVideo) same.put("video")
        sameLayer.tags().filter { it != "video" }.forEach { same.put(it) }
        if (same.length() > 0) options.put("sameLayer", same)
        rootWindowInsets?.let { options.put("safeArea", insetsJson(it, dpr)) }
        session.create(dir.absolutePath, data.absolutePath, startRoute, lw, lh, dpr, options.toString(), backend, deviceInfo())
    }

    /** auto 后端的机型分级：内存、低内存机型标记、API level（与 Flutter 插件同一组） */
    private fun deviceInfo(): EngineSession.DeviceInfo {
        val am = context.getSystemService(Context.ACTIVITY_SERVICE) as? ActivityManager
        val mem = ActivityManager.MemoryInfo().also { am?.getMemoryInfo(it) }
        return EngineSession.DeviceInfo(mem.totalMem, am?.isLowRamDevice ?: false, Build.VERSION.SDK_INT)
    }

    /** 刘海 / 状态栏 / 导航条（逻辑像素）：`statusBarHeight`、胶囊位置与 `env(safe-area-inset-*)` 都按它算 */
    override fun onApplyWindowInsets(insets: WindowInsets): WindowInsets {
        val o = insetsJson(insets, resources.displayMetrics.density)
        session.run {
            MiniEngine.nativeSetSafeArea(
                it,
                o.getDouble("top").toFloat(), o.getDouble("right").toFloat(),
                o.getDouble("bottom").toFloat(), o.getDouble("left").toFloat(),
            )
        }
        return super.onApplyWindowInsets(insets)
    }

    @Suppress("DEPRECATION")
    private fun insetsJson(i: WindowInsets, dpr: Float): JSONObject {
        val (t, r, b, l) = if (Build.VERSION.SDK_INT >= 30) {
            val s = i.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout())
            listOf(s.top, s.right, s.bottom, s.left)
        } else {
            listOf(i.systemWindowInsetTop, i.systemWindowInsetRight, i.systemWindowInsetBottom, i.systemWindowInsetLeft)
        }
        return JSONObject().put("top", t / dpr).put("right", r / dpr).put("bottom", b / dpr).put("left", l / dpr)
    }

    // ─────────────────────────── SurfaceView ───────────────────────────

    /**
     * Callback2：窗口这一帧要等这块 SurfaceView 有了内容再一起上屏（[EngineSession.onRedrawNeeded]）。
     * 只实现 Callback 的话，回桌面再回来时窗口不等它、直接上屏，SurfaceView 还没有缓冲 —— 露出底下那层，闪一下
     */
    private val callback = object : SurfaceHolder.Callback2 {
        override fun surfaceCreated(holder: SurfaceHolder) {}

        override fun surfaceRedrawNeeded(holder: SurfaceHolder) {}

        override fun surfaceRedrawNeededAsync(holder: SurfaceHolder, drawingFinished: Runnable) =
            session.onRedrawNeeded(drawingFinished)

        override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
            val s = holder.surface
            requestHighRefresh(s)
            session.onSurface(s)
        }

        // 返回之后不能再碰这块 Surface：同步等引擎线程放手
        override fun surfaceDestroyed(holder: SurfaceHolder) = session.onSurfaceGone()
    }

    /** 引擎的像素尺寸到了 / 变了。第一次到的时候才建 SurfaceView：缓冲区一建出来就是对的尺寸 */
    private fun setPixelSize(w: Int, h: Int) {
        if (w <= 0 || h <= 0) return
        val first = fixed == null
        if (fixed == w to h) return
        fixed = w to h
        if (first || surfaceView == null) return newSurfaceView()
        surfaceView?.let { sv ->
            sv.holder.setFixedSize(w, h)
            sv.layoutParams = LayoutParams(w, h)
        }
    }

    /**
     * 换一块从没被连过的 SurfaceView（第一次建、GPU 回退 CPU、直写失败改走 Bitmap：
     * 那块 Surface 已经被另一种生产者连过）。主线程调
     */
    private fun newSurfaceView() {
        surfaceView?.holder?.removeCallback(callback)
        removeAllViews()
        val sv = SurfaceView(context)
        // 第一帧之前先藏起来（Android 10+ 的 SurfaceView 才支持 alpha）：没有缓冲的 SurfaceView 露的是黑底，
        // 第一次打开时引擎要先开 GPU（一加 PLY110 约 0.5s）。藏着的时候露出这个视图的背景色
        surfaceRevealed = false
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) sv.alpha = 0f
        sv.holder.addCallback(callback)
        // 一个引擎像素对一个屏幕像素：布局尺寸 = 缓冲区尺寸 = 引擎像素尺寸。视图宽 1080、引擎 1079 时
        // MATCH_PARENT 会让 SurfaceFlinger 把整帧横向拉伸 1 像素（全屏发虚），所以按像素尺寸摆在左上角
        val (w, h) = fixed ?: (LayoutParams.MATCH_PARENT to LayoutParams.MATCH_PARENT)
        fixed?.let { (fw, fh) -> sv.holder.setFixedSize(fw, fh) }
        addView(sv, LayoutParams(w, h))
        surfaceView = sv
    }

    private fun revealSurface() {
        if (surfaceRevealed) return
        surfaceRevealed = true
        surfaceView?.alpha = 1f
    }

    /**
     * 这一层在动的时候要高刷：没人要的话系统（ColorOS 实测）会把整个屏降到 60Hz。
     * 只是「希望」，静止时系统照样可以降（静止页面本来也不出帧）
     */
    private fun requestHighRefresh(s: Surface) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) return
        val max = display?.supportedModes?.maxOfOrNull { it.refreshRate } ?: return
        try {
            s.setFrameRate(max, Surface.FRAME_RATE_COMPATIBILITY_DEFAULT)
        } catch (_: Exception) {
        }
    }

    // ─────────────────────────── 事件 ───────────────────────────

    /** 主线程：胶囊的两个动作交给宿主（没设回调就走默认） */
    private fun dispatchEvent(raw: String) {
        val ev = runCatching { JSONObject(raw) }.getOrNull() ?: return
        onEvent?.invoke(ev)
        when (ev.optString("type")) {
            "app" -> about = MiniProgramAbout.fromJson(ev.optJSONObject("about"))
            "menuButton" -> when (ev.optString("action")) {
                "menu" -> showAbout()
                "close" -> requestClose()
            }
            "hostCall" -> hostCalls.dispatch(ev, activity())
            "inputFocus" -> ime.onFocus(ev)
            "inputBlur" -> ime.onBlur()
            "sameLayerViews" -> sameLayer.onViews(ev)
            "nativeViewTap" -> sameLayer.onTap(ev)
            // 视频进 / 出全屏（横屏由引擎转画面）：收起 / 放出系统栏
            "videoFullscreen" -> if (immersiveVideoFullscreen) SystemBars.immersive(activity(), ev.optBoolean("fullScreen"))
            // wx.setClipboardData、输入框里的 copy / cut：引擎自己存一份并弹 Toast，系统剪贴板由宿主写
            "clipboard" -> (context.getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager)
                ?.setPrimaryClip(ClipData.newPlainText("mini-program", ev.optString("data")))
        }
    }

    /** 视图所在的 Activity（顺着 ContextWrapper 找） */
    private fun activity(): Activity? {
        var c: Context? = context
        while (c is ContextWrapper) {
            if (c is Activity) return c
            c = c.baseContext
        }
        return null
    }

    private fun showAbout() {
        val a = about ?: MiniProgramAbout.fromJson(null)
        onAbout?.let { it(a); return }
        AlertDialog.Builder(context)
            .setTitle(a.dialogTitle)
            .setMessage(a.dialogMessage)
            .setPositiveButton("确定", null)
            .show()
    }

    private fun requestClose() {
        onClose?.let { it(); return }
        activity()?.finish()
    }

    override fun onCheckIsTextEditor(): Boolean = ime.active

    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean = ime.onKey(event) || super.onKeyDown(keyCode, event)

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection? = ime.create(outAttrs)

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (!session.ready) return false
        val dpr = resources.displayMetrics.density
        fun send(phase: Int, x: Float, y: Float) = session.run { MiniEngine.nativePointer(it, phase, x / dpr, y / dpr) }
        // 移动带上事件时间（uptimeMillis，与 pump 用的 Choreographer 帧时间同一个单调钟）：
        // 引擎按它重采样页面拖动，post 到哪一帧不再影响每帧挪多少（慢拖不一顿一顿）
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> send(0, event.x, event.y)
            MotionEvent.ACTION_MOVE -> {
                // 历史点也要送：抽稀会让滑动手感变差，也会漏掉快速滑动的方向判定。
                // 一个 MotionEvent 的全部采样打成一包、一次 post（与 iOS 的 coalescedTouches 一致）：
                // 从前每个历史点各 post 一个 Runnable、各挂一次帧，120Hz 触控上一帧三四个
                val n = event.historySize + 1
                val xs = FloatArray(n)
                val ys = FloatArray(n)
                val ts = LongArray(n)
                for (i in 0 until n - 1) {
                    xs[i] = event.getHistoricalX(i) / dpr
                    ys[i] = event.getHistoricalY(i) / dpr
                    ts[i] = event.getHistoricalEventTime(i)
                }
                xs[n - 1] = event.x / dpr
                ys[n - 1] = event.y / dpr
                ts[n - 1] = event.eventTime
                session.run { h -> for (i in 0 until n) MiniEngine.nativePointerMoveAt(h, xs[i], ys[i], ts[i].toDouble()) }
            }
            MotionEvent.ACTION_UP -> send(2, event.x, event.y)
            MotionEvent.ACTION_CANCEL -> send(3, 0f, 0f)
            else -> return false
        }
        return true
    }
}
