package dev.minirender

import android.app.Activity
import android.app.AlertDialog
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.ContextWrapper
import android.os.Build
import android.util.AttributeSet
import android.view.KeyEvent
import android.view.MotionEvent
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
        override fun onPixelSize(w: Int, h: Int) = surfaces.setPixelSize(w, h)
        override fun reveal() = surfaces.revealSurface()
        override fun newSurface() = surfaces.newSurfaceView()
        override fun onEvents(list: List<String>) = list.forEach(::onEngineEvent)
    })
    private var appDir: File? = null
    private var startRoute: String? = null
    private var created = false
    private val hostCalls = HostCallRouter(context, session)
    /** 软键盘：输入框聚焦弹出、输入法的字交给引擎（MiniImeConnection.kt） */
    private val ime = MiniIme(this, ::sendText, ::sendKey)
    // adjust-position：键盘高度推给 wx.onKeyboardHeightChange；顶起量交给引擎（合成时上移，导航栏不动）
    private val keyboard = MiniKeyboard(
        this,
        { h -> dispatchEvent("keyboardHeightChange", JSONObject().put("height", h)) },
        { shift -> session.run { MiniEngine.nativeSetKeyboardShift(it, shift) } },
        { sendKey("escape") }, // 用户自己收了键盘：输入框失焦（引擎派发 bindblur、回 inputBlur）
        { navReserve.px }, // 留给导航栏的那一截：键盘先盖住它
    )
    /** 底部留给系统导航栏的那一截（NavBarReserve.kt）：引擎只画上面那部分 */
    private val navReserve = NavBarReserve(this)
    private val sameLayer = SameLayerViews(session) { resources.displayMetrics.density }

    /** SurfaceView 的建 / 换 / 定尺寸（MiniSurfaceHost.kt，只在主线程读写） */
    private val surfaces: MiniSurfaceHost by lazy { MiniSurfaceHost(this, session) }

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
     * 启动参数（`open` 之前设）：外部 App / H5 / 别的小程序打开时给，小程序在 `App.onLaunch(options)` 与
     * `wx.getLaunchOptionsSync()` 里读到。形状与微信一致：
     * `{"path":"pages/x/x","query":{"id":"1"},"scene":1065,"referrerInfo":{"appId":"wx…","extraData":{…}}}`。
     * 不给就是默认（scene 1001）
     */
    var launchOptions: JSONObject? = null

    /**
     * `<video>` 走同层渲染（默认开，与 Flutter 插件一致；`open` 之前设）：SDK 自带的系统播放器（MediaPlayer，硬解、
     * 带声音）解码，帧交给引擎画进页面，滚动时与页面一起动；控制条、全屏（横 / 竖）由引擎画。
     * false = 引擎自己软解（openh264，手机上没有声音、耗 CPU）
     */
    var sameLayerVideo = true

    /**
     * 替宿主把窗口铺满整屏（透明状态栏 / 导航条、刘海区也画；默认开，挂到窗口时生效）：引擎按 insets 算状态栏高度、
     * 胶囊位置与安全区，画面不铺到状态栏下面就对不上。宿主自己管系统栏样式的话关掉
     */
    var autoEdgeToEdge = true

    /** 跟着所在 Activity 的 resume / pause / destroy 与系统内存告警自动转给引擎（默认开；关掉的话宿主自己调 onHost*） */
    var autoLifecycle = true

    /** `wx.vibrateShort` / `vibrateLong` 由 SDK 做触感反馈（默认开；关掉的话宿主在 [onEvent] 里接 `vibrate`） */
    var hapticFeedback = true

    /** 视频全屏时收起状态栏与导航条（默认开；[onEvent] 里也能收到 `videoFullscreen` 自己处理） */
    var immersiveVideoFullscreen = true

    /**
     * 渲染后端偏好（`open` 之前设）：[MiniEngine.BACKEND_AUTO]（默认：机型够格走 GPU，不够格 / 初始化失败 /
     * 运行中丢设备一律回退 CPU）、[MiniEngine.BACKEND_CPU]（强制软件光栅化）、[MiniEngine.BACKEND_GPU]。
     * 调试时也可以 `adb shell setprop debug.mini.backend cpu` 强制，实际用的是哪个看 [activeBackend]
     */
    var backend = MiniEngine.BACKEND_AUTO
    /**
     * 诊断开关（`open` 之前设；测试包里打开，不用 `adb setprop`）：名字与 `debug.mini.<名字>` 同一组，
     * 如 `setOf("damage_log", "fps")`。日志在 `adb logcat -s mini-render`：每帧慢在哪（〔慢：解码 / 降采样 / 缩放 / 字形〕）、
     * 为什么整帧（`整帧·数据 ← 原因`）、关闭耗时（`[生命周期]`）。见 [MiniDiag]
     */
    var diagnostics: Set<String> = emptySet()

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
     * 宿主推事件给小程序的 `wx.onXxx`（`accelerometerChange` / `locationChange` / `networkStatusChange` /
     * `contextEvent` …，名字写 `onLocationChange` 也认）。与 Flutter 插件的 `dispatchEvent` 同一组名字。
     * 异步投到引擎线程；[done]（可选，主线程）拿到收到它的监听者个数，0 = 小程序没在听，可以停掉对应的传感器
     */
    fun dispatchEvent(name: String, payload: JSONObject? = null, done: ((Int) -> Unit)? = null) = session.run {
        val n = MiniEngine.nativeDispatchEvent(it, name, payload?.toString())
        if (done != null) post { done(n) }
    }

    /**
     * 返回键。**返回 false 表示页面栈只剩一页**，宿主该退出小程序。
     *
     * 注意这是同步等结果的（要立刻决定 `finish()` 还是不动），所以内部做了一次
     * 跨线程等待；返回键是低频操作，代价可以忽略。
     */
    fun goBack(): Boolean = session.call(false) { MiniEngine.nativeBack(it) }

    /**
     * 回前台。所在 Activity 的 resume / pause / destroy 与内存告警 SDK 已经自己跟着转了（[autoLifecycle]），
     * 宿主不用再调；调了也没关系：前后台状态没变的那次不转
     */
    fun onHostResume() {
        if (!background) return
        background = false
        session.lifecycle(0)
    }

    /** 进后台：派发 `App.onHide` 并**停止出帧**（继续出帧纯属耗电） */
    fun onHostPause() {
        if (background) return
        background = true
        session.lifecycle(1)
    }

    /** 对应 Activity/Application 的 `onTrimMemory` */
    fun onHostTrimMemory() = session.lifecycle(2)

    /**
     * 销毁引擎：返回时引擎线程已经放手 Surface；实例在引擎线程上异步销毁（不卡主线程，退出时回 App 的动画不顿）。
     * 之后可以在**新的**视图上 `open` 另一个小程序（新引擎会先等上一个销毁完 —— 一个进程同一时刻只有一个
     * 引擎实例，小程序根目录、数据目录是进程级的）
     */
    fun close() {
        lifecycle?.unregister()
        lifecycle = null
        session.dispose()
    }

    private var background = false
    private var lifecycle: HostLifecycle? = null

    /** 挂到窗口上：按开关替宿主铺满整屏、接上 Activity 生命周期（HostChrome.kt） */
    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        keyboard.attach()
        val a = activity() ?: return
        if (autoEdgeToEdge) HostChrome.edgeToEdge(a)
        if (autoLifecycle && lifecycle == null) {
            lifecycle = HostLifecycle(a, ::onHostResume, ::onHostPause, ::onHostTrimMemory, ::close).also { it.register() }
        }
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
        resizeEngine()
    }

    private fun resizeEngine() {
        val dpr = resources.displayMetrics.density
        session.resize((width / dpr).toInt().coerceAtLeast(1), (navReserve.engineHeight(height) / dpr).toInt().coerceAtLeast(1), dpr)
    }

    /** 安全区（逻辑像素）：底下留了导航栏那一截时，底部安全区已在画面之外，给 0（与 Flutter 插件一致） */
    private fun safeArea(insets: WindowInsets): JSONObject =
        HostChrome.insetsJson(insets, resources.displayMetrics.density).also { if (navReserve.px > 0) it.put("bottom", 0.0) }

    private fun createEngine() {
        val dir = appDir ?: return
        if (created || width <= 0 || height <= 0) return
        created = true
        val dpr = resources.displayMetrics.density
        // 逻辑尺寸 = 像素 / density（引擎按逻辑坐标工作，rpx 由它自己换算）
        val lw = (width / dpr).toInt().coerceAtLeast(1)
        rootWindowInsets?.let { navReserve.update(it) }
        val lh = (navReserve.engineHeight(height) / dpr).toInt().coerceAtLeast(1)
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
            .put("launchOptions", launchOptions ?: JSONObject.NULL)
            // 点了名的 wx.* 才交给宿主（hostCall 事件）；没点名的照旧立刻 fail
            .put("hostApis", JSONArray(hostCalls.names()))
        // 同层渲染：宿主画 / 解码，引擎画进页面，滚动时与页面在同一张图里走。
        // - 视频：帧由 VideoHost 交（EngineSession 接 `videos` / `videoCommand`）；
        // - 登记了的组件（[sameLayerViews]，如 map）：宿主画（`sameLayerViews` / `nativeViewTap` 事件）
        val same = JSONArray()
        if (sameLayerVideo) same.put("video")
        sameLayer.tags().filter { it != "video" }.forEach { same.put(it) }
        if (same.length() > 0) options.put("sameLayer", same)
        rootWindowInsets?.let { options.put("safeArea", safeArea(it)) }
        session.create(dir.absolutePath, data.absolutePath, startRoute, lw, lh, dpr, options.toString(), backend, HostChrome.deviceInfo(context), diagnostics)
    }

    /** 刘海 / 状态栏 / 导航条（逻辑像素）：`statusBarHeight`、胶囊位置与 `env(safe-area-inset-*)` 都按它算 */
    override fun onApplyWindowInsets(insets: WindowInsets): WindowInsets {
        if (navReserve.update(insets) && created) resizeEngine() // 三键 ↔ 手势导航切换、转屏
        keyboard.onInsets(insets)
        val o = safeArea(insets)
        session.run {
            MiniEngine.nativeSetSafeArea(
                it,
                o.getDouble("top").toFloat(), o.getDouble("right").toFloat(),
                o.getDouble("bottom").toFloat(), o.getDouble("left").toFloat(),
            )
        }
        return super.onApplyWindowInsets(insets)
    }

    // ─────────────────────────── 事件 ───────────────────────────

    /** 主线程：胶囊的两个动作交给宿主（没设回调就走默认） */
    private fun onEngineEvent(raw: String) {
        val ev = runCatching { JSONObject(raw) }.getOrNull() ?: return
        onEvent?.invoke(ev)
        when (ev.optString("type")) {
            "app" -> about = MiniProgramAbout.fromJson(ev.optJSONObject("about"))
            "page" -> setBackgroundColor(NavBarReserve.color(ev)) // 导航栏那一截与 SurfaceView 第一帧之前露出的底色
            "menuButton" -> when (ev.optString("action")) {
                "menu" -> showAbout()
                "close" -> requestClose()
            }
            "hostCall" -> hostCalls.dispatch(ev, activity())
            "vibrate" -> if (hapticFeedback) HostChrome.vibrate(this, ev)
            "inputFocus" -> { ime.onFocus(ev); keyboard.onFocus(ev) }
            "inputBlur" -> { ime.onBlur(); keyboard.onBlur() }
            "inputValue" -> ime.onValue(ev)
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

    override fun onDetachedFromWindow() = keyboard.detach().also { super.onDetachedFromWindow() }

    /** 第一根 / 第二根手指的跟踪（第二根只给图片预览的双指缩放用） */
    private val touch = MiniTouch()

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (!session.ready) return false
        if (event.actionMasked == MotionEvent.ACTION_DOWN && navReserve.swallows(event.y, height)) return false
        return touch.forward(session, event, resources.displayMetrics.density)
    }

}
