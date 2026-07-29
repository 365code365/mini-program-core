package dev.minirender

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.os.Handler
import android.os.HandlerThread
import android.util.AttributeSet
import android.view.MotionEvent
import android.view.View
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
    private var appDir: File? = null
    private var startRoute: String? = null
    private var running = false

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
        post {
            handle = MiniEngine.nativeCreate(dir.absolutePath, data.absolutePath, lw, lh, dpr)
            if (handle == 0L) return@post
            MiniEngine.nativeLaunch(handle, route)
            running = true
            frame()
        }
    }

    private fun requestFrame() = post { frame() }

    /** 一帧：pump → 有变化就把像素搬进 Bitmap → 请求 UI 线程重绘 */
    private fun frame() {
        if (handle == 0L || !running) return
        val changed = MiniEngine.nativePump(handle, System.nanoTime() / 1_000_000)
        if (changed) {
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
        }
        // 还有动画/滚动/定时器时继续按刷新率出帧；静止时下一帧由事件触发
        if (changed) engineHandler.post { frame() }
    }

    override fun onDraw(canvas: Canvas) {
        bitmap?.let { canvas.drawBitmap(it, 0f, 0f, paint) }
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (handle == 0L) return false
        val dpr = resources.displayMetrics.density
        fun send(phase: Int, x: Float, y: Float) =
            post { if (handle != 0L) MiniEngine.nativePointer(handle, phase, x / dpr, y / dpr) }
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> send(0, event.x, event.y)
            MotionEvent.ACTION_MOVE -> {
                // 历史点也要送：抽稀会让滑动手感变差，也会漏掉快速滑动的方向判定
                for (i in 0 until event.historySize) {
                    send(1, event.getHistoricalX(i), event.getHistoricalY(i))
                }
                send(1, event.x, event.y)
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

    /** 功能键：enter / backspace / delete / left / right / home / end / selectall */
    fun sendKey(name: String) {
        post { if (handle != 0L) MiniEngine.nativeKey(handle, name) }
        requestFrame()
    }
}
