package dev.minirender

import android.app.Activity
import android.app.AlertDialog
import android.content.Context
import android.content.ContextWrapper
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.util.AttributeSet
import android.view.Choreographer
import android.view.MotionEvent
import android.view.View
import android.view.WindowInsets
import org.json.JSONObject
import java.io.File
import java.nio.ByteBuffer

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
 * - 起一条**专用渲染线程**（引擎不是线程安全的，所有调用都排到这条线程上）；
 * - 按 `View` 的实际尺寸与 `density` 算逻辑尺寸和 dpr；
 * - `onTouchEvent` → 引擎的**指针三段**（含 `getHistoricalX/Y` 的历史点，不抽稀）；
 * - 只在引擎说「有新画面」时才上屏，静止时不出帧（不耗电）；
 * - 生命周期与内存告警转发。
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
 * 普通页面的导航栏是宿主画的，胶囊也由宿主放进自己的导航栏。
 */
class MiniProgramView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
) : View(context, attrs) {

    private var handle = 0L
    private var bitmap: Bitmap? = null
    private var pixels: ByteArray? = null
    private val paint = Paint(Paint.FILTER_BITMAP_FLAG)
    private val thread = HandlerThread("mini-render").apply { start() }
    private val engineHandler = Handler(thread.looper)
    /** 引擎线程的 ADPF 性能提示（只在引擎线程读写；为什么要见 FrameBoost.kt） */
    private var boost: FrameBoost? = null
    private var appDir: File? = null
    private var startRoute: String? = null
    private var running = false
    private val main = Handler(Looper.getMainLooper())
    // ── 只在引擎线程读写 ──
    private var choreographer: Choreographer? = null
    private var frameScheduled = false
    private val frameCallback = Choreographer.FrameCallback { t ->
        frameScheduled = false
        frame(t / 1_000_000)
    }

    /** 引擎在 custom 导航栏的页面上画右上角胶囊（`open` 之前设） */
    var menuButton = true

    /** 胶囊「•••」：查看关于（主线程回调）。null = 默认弹 AlertDialog */
    var onAbout: ((MiniProgramAbout) -> Unit)? = null

    /** 胶囊「⊙」：返回宿主（主线程回调）。null = 默认结束所在的 Activity */
    var onClose: (() -> Unit)? = null

    /** 引擎发来的每一条事件（主线程回调；`page` / `navigationBar` / `inputFocus` / `app` / `menuButton` …） */
    var onEvent: ((JSONObject) -> Unit)? = null

    /** 小程序的关于信息（启动后第一帧到；主线程读） */
    var about: MiniProgramAbout? = null
        private set

    /** 打开一个解包后的小程序目录。可指定起始页（深链）。 */
    fun open(dir: File, route: String? = null) {
        appDir = dir
        startRoute = route
        if (width > 0 && height > 0) createEngine()
    }

    /** 跳到某个页面（`/pages/x/x?id=1`） */
    fun navigate(url: String) = post { if (handle != 0L) MiniEngine.nativeNavigate(handle, url) }

    /**
     * 返回键。**返回 false 表示页面栈只剩一页**，宿主该退出小程序。
     *
     * 注意这是同步等结果的（要立刻决定 `finish()` 还是不动），所以内部做了一次
     * 跨线程等待；返回键是低频操作，代价可以忽略。
     */
    fun goBack(): Boolean {
        if (handle == 0L) return false
        var consumed = false
        val lock = Object()
        var done = false
        post {
            consumed = MiniEngine.nativeBack(handle)
            synchronized(lock) { done = true; lock.notifyAll() }
        }
        synchronized(lock) { while (!done) lock.wait(500); }
        if (consumed) requestFrame()
        return consumed
    }

    fun onHostResume() {
        running = true
        post { if (handle != 0L) MiniEngine.nativeLifecycle(handle, 0) }
        requestFrame()
    }

    /** 进后台：派发 `App.onHide` 并**停止出帧**（继续出帧纯属耗电） */
    fun onHostPause() {
        running = false
        post { if (handle != 0L) MiniEngine.nativeLifecycle(handle, 1) }
    }

    /** 对应 Activity/Application 的 `onTrimMemory` */
    fun onHostTrimMemory() = post { if (handle != 0L) MiniEngine.nativeLifecycle(handle, 2) }

    fun close() {
        post {
            if (handle != 0L) {
                MiniEngine.nativeDestroy(handle)
                handle = 0L
            }
            boost?.close()
            boost = null
        }
        thread.quitSafely()
    }

    // ─────────────────────────── 内部 ───────────────────────────

    private fun post(block: () -> Unit) = engineHandler.post(block)

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        if (handle == 0L) createEngine() else requestFrame()
    }

    private fun createEngine() {
        val dir = appDir ?: return
        if (width <= 0 || height <= 0) return
        val dpr = resources.displayMetrics.density
        // 逻辑尺寸 = 像素 / density（引擎按逻辑坐标工作，rpx 由它自己换算）
        val lw = (width / dpr).toInt().coerceAtLeast(1)
        val lh = (height / dpr).toInt().coerceAtLeast(1)
        // 数据目录必须给：storage 与图片磁盘缓存都落在这里，不给会静默失效
        val data = File(context.filesDir, "mini-render").apply { mkdirs() }
        val route = startRoute
        // 选项要在 app.js 执行前就位：onLaunch 里读 getSystemInfoSync（状态栏高度、胶囊位置）是常态
        val options = JSONObject()
            .put("systemInfo", JSONObject()
                .put("platform", "android")
                .put("brand", Build.BRAND)
                .put("model", Build.MODEL)
                .put("system", "Android ${Build.VERSION.RELEASE}"))
            .put("menuButton", menuButton)
        rootWindowInsets?.let { options.put("safeArea", insetsJson(it, dpr)) }
        post {
            choreographer = Choreographer.getInstance()
            if (boost == null) boost = FrameBoost.forCurrentThread(context)
            // 系统中文字体（约 0.35s）跟建引擎并行；这个视图走 Bitmap（CPU），不预热 GPU（pref 传 cpu）
            MiniEngine.nativeWarmup(lw, lh, dpr, MiniEngine.BACKEND_CPU, 0L, false, Build.VERSION.SDK_INT)
            handle = MiniEngine.nativeCreateEx(dir.absolutePath, data.absolutePath, lw, lh, dpr, options.toString())
            if (handle == 0L) return@post
            MiniEngine.nativeLaunch(handle, route)
            running = true
            frame(System.nanoTime() / 1_000_000)
        }
    }

    /** 刘海 / 状态栏 / 导航条（逻辑像素）：`statusBarHeight`、胶囊位置与 `env(safe-area-inset-*)` 都按它算 */
    override fun onApplyWindowInsets(insets: WindowInsets): WindowInsets {
        val o = insetsJson(insets, resources.displayMetrics.density)
        post {
            if (handle != 0L) {
                MiniEngine.nativeSetSafeArea(
                    handle,
                    o.getDouble("top").toFloat(), o.getDouble("right").toFloat(),
                    o.getDouble("bottom").toFloat(), o.getDouble("left").toFloat(),
                )
                scheduleFrame()
            }
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

    private fun requestFrame() = post { scheduleFrame() }

    /** 引擎线程：挂到下一个 vsync（同一帧只挂一次） */
    private fun scheduleFrame() {
        if (frameScheduled || !running || handle == 0L) return
        frameScheduled = true
        choreographer?.postFrameCallback(frameCallback)
    }

    /**
     * 一帧：pump → 有变化就把像素搬进 Bitmap → 请求 UI 线程重绘；有事件就取完；
     * 引擎说还有活（动画 / 定时器 / 请求在飞 / 按住的手指）就挂下一个 vsync，否则停下等输入。
     */
    private fun frame(nowMs: Long) {
        if (handle == 0L || !running) return
        val t0 = System.nanoTime()
        val flags = MiniEngine.nativePumpEx(handle, nowMs)
        if (flags and MiniEngine.FRAME_EVENTS != 0) drainEvents()
        if (flags and MiniEngine.FRAME_NEW != 0) {
            val size = MiniEngine.nativePixelSize(handle)
            val w = (size ushr 32).toInt()
            val h = (size and 0xFFFFFFFFL).toInt()
            if (w > 0 && h > 0) {
                var bmp = bitmap
                if (bmp == null || bmp.width != w || bmp.height != h) {
                    bmp = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
                    bitmap = bmp
                    pixels = ByteArray(w * h * 4)
                }
                val buf = pixels!!
                if (MiniEngine.nativePixels(handle, buf) == buf.size) {
                    bmp.copyPixelsFromBuffer(ByteBuffer.wrap(buf))
                    postInvalidate()
                }
            }
            boost?.report(System.nanoTime() - t0)
        }
        // 从前这里只在「这一帧有新画面」时续帧：setTimeout / 网络回包在静止页面上停住。
        // 改认 WANTS_MORE（与 Flutter 插件的 EngineHost 同一条规则）
        if (flags and MiniEngine.FRAME_WANTS_MORE != 0) scheduleFrame()
    }

    /** 引擎线程：取完事件队列，交到主线程分发 */
    private fun drainEvents() {
        val list = ArrayList<String>()
        while (true) list.add(MiniEngine.nativePollEvent(handle) ?: break)
        if (list.isNotEmpty()) main.post { list.forEach(::dispatchEvent) }
    }

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
        }
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
        var c: Context? = context
        while (c is ContextWrapper) {
            if (c is Activity) { c.finish(); return }
            c = c.baseContext
        }
    }

    override fun onDraw(canvas: Canvas) {
        bitmap?.let { canvas.drawBitmap(it, 0f, 0f, paint) }
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (handle == 0L) return false
        val dpr = resources.displayMetrics.density
        fun send(phase: Int, x: Float, y: Float) =
            post { if (handle != 0L) MiniEngine.nativePointer(handle, phase, x / dpr, y / dpr) }
        // 移动带上事件时间（uptimeMillis，与 pump 用的 nanoTime / Choreographer 帧时间同一个单调钟）：
        // 引擎按它重采样页面拖动，post 到哪一帧不再影响每帧挪多少（慢拖不一顿一顿）
        fun move(x: Float, y: Float, t: Long) =
            post { if (handle != 0L) MiniEngine.nativePointerMoveAt(handle, x / dpr, y / dpr, t.toDouble()) }
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> send(0, event.x, event.y)
            MotionEvent.ACTION_MOVE -> {
                // 历史点也要送：抽稀会让滑动手感变差，也会漏掉快速滑动的方向判定
                for (i in 0 until event.historySize) {
                    move(event.getHistoricalX(i), event.getHistoricalY(i), event.getHistoricalEventTime(i))
                }
                move(event.x, event.y, event.eventTime)
            }
            MotionEvent.ACTION_UP -> send(2, event.x, event.y)
            MotionEvent.ACTION_CANCEL -> send(3, 0f, 0f)
            else -> return false
        }
        requestFrame()
        return true
    }

    /** 软键盘提交的文字（宿主可以自己接 InputConnection，简单场景直接调这个） */
    fun sendText(text: String) {
        post { if (handle != 0L) MiniEngine.nativeTextInput(handle, text) }
        requestFrame()
    }

    /**
     * 功能键：enter / newline / backspace / delete / left / right / up / down / home / end、
     * shift+… 扩展选区、word-left / word-right、doc-start / doc-end、delete-word-back、
     * selectall / copy / cut / undo / redo / escape。回车也可以 sendText("\n")（textarea 换行）
     */
    fun sendKey(name: String) {
        post { if (handle != 0L) MiniEngine.nativeKey(handle, name) }
        requestFrame()
    }
}
