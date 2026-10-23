package dev.minirender

import android.content.Context
import android.graphics.Bitmap
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.util.Log
import android.view.Choreographer
import android.view.Surface
import java.io.File
import java.io.FileOutputStream
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

/**
 * [MiniProgramView] 的引擎侧：专用引擎线程 + vsync 出帧 + 往 SurfaceView 上屏。
 *
 * 规则与 Flutter 插件的 `EngineHost`（独立图层模式）一条一条对齐 —— 那边在一加 PLY110 上用
 * SurfaceFlinger `--latency` 逐条量过。从前这个视图是「像素拷进 ByteArray → copyPixelsFromBuffer →
 * postInvalidate → onDraw 里 drawBitmap」，每帧三次整帧拷贝 + 一次纹理上传、晚一个 vsync、还只能走 CPU，
 * 原生 App 一滑就顿。为什么每一条要这么做见 doc/滑动顿挫避坑清单.md。
 *
 * ## 线程
 * - **引擎线程**（HandlerThread）：所有 `MiniEngine.native*` 都在这里调（逻辑层是 QuickJS，不是线程安全的）。
 *   出帧节拍用这条线程上的 Choreographer；上屏也在这条线程上，**不经过 UI 线程**。
 * - **主线程**：SurfaceView 的建 / 改尺寸 / 显示（[Ui]），Surface 回调进来。
 *
 * ## 上屏（三条路，按顺序试）
 * 0. GPU（偏好 auto / gpu）：Surface 交给引擎的 GPU 合成器，引擎自己上屏（`FRAME_PRESENTED`）。
 *    滚动帧只合成、不重画；没选上或运行中回退（`FRAME_BACKEND_CHANGED`）就降到第 1 条；
 * 1. WINDOW：引擎 `ANativeWindow_lock` 直接写进 Surface，一次拷贝；锁不上降到第 2 条；
 * 2. BITMAP：写进 Bitmap 再 `lockHardwareCanvas` + `drawBitmap`。
 * 一块 Surface 被一种生产者（GPU / CPU）连过之后不能再给另一种用：降级时让视图换一块新的 SurfaceView。
 */
internal class EngineSession(private val context: Context, private val ui: Ui) {
    /** 主线程回调 */
    interface Ui {
        /** 引擎的像素尺寸定了 / 变了：SurfaceView 按它定缓冲区与布局尺寸（一个引擎像素对一个屏幕像素） */
        fun onPixelSize(w: Int, h: Int)
        /** 引擎在当前这块 SurfaceView 上交出了第一帧：显示出来 */
        fun reveal()
        /** 当前这块 Surface 已被另一种生产者连过：换一块新的 SurfaceView */
        fun newSurface()
        fun onEvents(list: List<String>)
    }

    /** 机型分级用（`nativeSetDeviceInfo`，auto 后端据此决定开不开 GPU） */
    data class DeviceInfo(val totalMem: Long, val lowRam: Boolean, val api: Int)

    private enum class Present { GPU, WINDOW, BITMAP }

    private val thread = HandlerThread("mini-render").apply { start() }
    private val engine = Handler(thread.looper)
    private val main = Handler(Looper.getMainLooper())

    // ── 只在引擎线程读写 ──
    private var handle = 0L
    private var choreographer: Choreographer? = null
    /** 引擎线程的性能提示（复杂页面的补条带在小核上要 5~11ms，见 FrameBoost.kt） */
    private var boost: FrameBoost? = null
    private var frameScheduled = false
    private var running = true
    private var surface: Surface? = null
    private var mode = Present.WINDOW
    /** GPU 模式下 [surface] 已交给合成器（同一块 Surface 改尺寸不用重挂：合成器自己 reconfigure） */
    private var gpuAttached = false
    private var bitmap: Bitmap? = null
    private var needsPresent = false

    /** 引擎已在当前这块 SurfaceView 上交过帧（主线程换新 SurfaceView 时清掉） */
    @Volatile private var revealed = false
    @Volatile private var disposed = false

    // ── 引擎线程写，哪条线程都能读 ──
    @Volatile var pixelW = 0
        private set
    @Volatile var pixelH = 0
        private set
    @Volatile var activeBackend = MiniEngine.BACKEND_CPU
        private set
    @Volatile var backendReason = ""
        private set
    /** 引擎建好、首页已打开 */
    @Volatile var ready = false
        private set

    private val frameCallback = Choreographer.FrameCallback { frameTimeNanos ->
        frameScheduled = false
        tick(frameTimeNanos)
    }

    /** 主线程调：引擎线程上建引擎、打开首页。Surface 由视图在拿到像素尺寸之后建出来再交过来 */
    fun create(
        appDir: String,
        dataDir: String,
        route: String?,
        width: Int,
        height: Int,
        dpr: Float,
        options: String,
        backend: Int,
        device: DeviceInfo,
    ) = engine.post {
        if (disposed) return@post
        choreographer = Choreographer.getInstance()
        boost = FrameBoost.forCurrentThread(context)
        // 诊断开关要在引擎读环境之前就位
        MiniDiag.apply()
        // 字体（约 0.35s）与 GPU（约 0.5s）跟建引擎并行
        MiniEngine.nativeWarmup(width, height, dpr, backend, device.totalMem, device.lowRam, device.api)
        val h = MiniEngine.nativeCreateEx(appDir, dataDir, width, height, dpr, options)
        if (h == 0L) {
            Log.e(TAG, "小程序打不开：$appDir（建引擎失败，看上面 mini-render 的报错）")
            return@post
        }
        // 机型信息要在挂 Surface 之前给（auto 的机型分级要用）
        MiniEngine.nativeSetDeviceInfo(h, device.totalMem, device.lowRam, device.api)
        MiniEngine.nativeSetBackend(h, backend)
        mode = if (backend != MiniEngine.BACKEND_CPU) {
            // 开 GPU 与下面的首页启动并行：SurfaceView 建出来时合成器多半已经好了
            MiniEngine.nativePrewarmGpu(h)
            Present.GPU
        } else {
            Present.WINDOW
        }
        if (!MiniEngine.nativeLaunch(h, route)) {
            MiniEngine.nativeDestroy(h)
            Log.e(TAG, "小程序打不开：$appDir（首页没打开）")
            return@post
        }
        handle = h
        ready = true
        updatePixelSize()
        refreshBackend()
        val (pw, ph) = pixelW to pixelH
        main.post { if (!disposed) ui.onPixelSize(pw, ph) }
        // 首页的数据回调 / 定时器先跑起来；Surface 到了再上屏（attach 会补画当前帧）
        requestFrame()
    }

    // ───────────────────────── 命令（主线程进来） ─────────────────────────

    /** 在引擎线程上执行一条命令，之后挂一帧（输入之后引擎要出帧回应） */
    fun run(block: (Long) -> Unit) = engine.post {
        if (handle != 0L) {
            block(handle)
            requestFrame()
        }
    }

    /**
     * 同步等引擎线程的结果（返回键要立刻决定 finish 还是不动）。引擎没建好返回 [fallback]。
     * 等到做完为止（与从前一样）：超时就当「没消费」的话，JS 卡一下用户按返回就整个退出了
     */
    fun <T> call(fallback: T, block: (Long) -> T): T {
        if (disposed || !ready) return fallback
        val latch = CountDownLatch(1)
        val box = AtomicReference(fallback)
        engine.post {
            try {
                if (handle != 0L) {
                    box.set(block(handle))
                    requestFrame()
                }
            } finally {
                latch.countDown()
            }
        }
        while (!latch.await(500, TimeUnit.MILLISECONDS)) if (disposed) break
        return box.get()
    }

    /** what: 0 进前台 / 1 进后台 / 2 内存告警 */
    fun lifecycle(what: Int) = engine.post {
        if (what == 0) running = true
        // 进后台：停止出帧（继续出帧纯属耗电），视频也停下（与微信一致，回前台不自动接着播）
        if (what == 1) {
            running = false
            videoHost?.pauseAll()
        }
        if (handle == 0L) return@post
        MiniEngine.nativeLifecycle(handle, what)
        if (what == 0) {
            needsPresent = true
            requestFrame()
        }
    }

    /** 视图改了尺寸（逻辑像素）：引擎重排，SurfaceView 跟着改缓冲区 */
    fun resize(width: Int, height: Int, dpr: Float) = engine.post {
        if (handle == 0L || !MiniEngine.nativeResize(handle, width, height, dpr)) return@post
        updatePixelSize()
        val (pw, ph) = pixelW to pixelH
        main.post { if (!disposed) ui.onPixelSize(pw, ph) }
        needsPresent = true
        requestFrame()
    }

    /** 把引擎当前这一帧存成 PNG（SurfaceView 的内容 `View.draw` 取不到）。[done] 在主线程回调 */
    fun dumpFrame(file: File, done: (Boolean) -> Unit) = engine.post {
        var ok = false
        if (handle != 0L && pixelW > 0 && pixelH > 0) {
            val bmp = Bitmap.createBitmap(pixelW, pixelH, Bitmap.Config.ARGB_8888)
            if (MiniEngine.nativePixelsToBitmap(handle, bmp) > 0) {
                ok = runCatching { FileOutputStream(file).use { bmp.compress(Bitmap.CompressFormat.PNG, 100, it) } }
                    .getOrDefault(false)
            }
            bmp.recycle()
        }
        main.post { done(ok) }
    }

    // ───────────────────────── Surface（主线程进来） ─────────────────────────

    /** SurfaceView 的 Surface 可用 / 改了尺寸 */
    fun onSurface(s: Surface) {
        if (!disposed) engine.post { attach(s) }
    }

    /** SurfaceView 的 Surface 要没了：返回之后不能再碰它，同步等引擎线程放手 */
    fun onSurfaceGone() {
        if (!disposed) onEngineSync { detach() }
    }

    /**
     * SurfaceView 要窗口等它这一帧（`SurfaceHolder.Callback2.surfaceRedrawNeededAsync`）：引擎线程马上在刚挂上的
     * Surface 上出一帧（不等下一个 vsync 回调），交上去之后再告诉系统。不这样做，回桌面再回来时窗口先上屏、
     * SurfaceView 还没有缓冲，露出底下那层（约 50ms 的一闪）。
     *
     * 这块 SurfaceView 还没交过帧（第一次打开，引擎在建 / 开 GPU，要几百毫秒）就不等：窗口不能停那么久，
     * 那时 SurfaceView 是藏着的（alpha 0）。超时兜底 [REDRAW_WAIT_MS]：引擎线程被 JS 卡住时窗口照常上屏
     */
    fun onRedrawNeeded(done: Runnable) {
        val once = AtomicBoolean(false)
        val t0 = System.nanoTime()
        fun finish(why: String) = Runnable {
            if (!once.compareAndSet(false, true)) return@Runnable
            done.run()
            Log.i(TAG, "🪟 窗口等引擎首帧 ${(System.nanoTime() - t0) / 100_000 / 10.0}ms（$why）")
        }
        if (disposed) return finish("已销毁").run()
        if (pixelW == 0 || !revealed) return finish("还没交过帧（第一次打开），不等").run()
        main.postDelayed(finish("超时"), REDRAW_WAIT_MS)
        engine.post {
            var why = "还没有引擎 / Surface"
            if (handle != 0L && surface != null) {
                choreographer?.removeFrameCallback(frameCallback)
                frameScheduled = false
                tick(System.nanoTime())
                why = "已交帧"
            }
            main.post(finish(why))
        }
    }

    /** 主线程调：先让引擎线程放手 Surface 并销毁引擎，再停线程 */
    fun dispose() {
        if (disposed) return
        disposed = true
        onEngineSync {
            choreographer?.removeFrameCallback(frameCallback)
            videoHost?.release()
            videoHost = null
            detach()
            if (handle != 0L) MiniEngine.nativeDestroy(handle)
            handle = 0L
            boost?.close()
            boost = null
            bitmap?.recycle()
            bitmap = null
        }
        thread.quitSafely()
    }

    // ───────────────────────── 引擎线程 ─────────────────────────

    private fun updatePixelSize() {
        val size = MiniEngine.nativePixelSize(handle)
        pixelW = (size ushr 32).toInt()
        pixelH = (size and 0xFFFFFFFFL).toInt()
    }

    private fun refreshBackend() {
        activeBackend = MiniEngine.nativeBackend(handle)
        backendReason = MiniEngine.nativeBackendReason(handle) ?: ""
    }

    private fun attach(s: Surface) {
        if (handle == 0L) return
        if (mode == Present.GPU) {
            if (!(gpuAttached && s === surface) && !attachGpu(s)) return
        } else {
            surface = s
            if (mode == Present.WINDOW && !MiniEngine.nativeAttachCpuWindow(handle, s)) mode = Present.BITMAP
        }
        needsPresent = true
        requestFrame()
    }

    /** 把 Surface 交给 GPU 合成器。没选上就改走 CPU（换一块新 Surface），返回 false */
    private fun attachGpu(s: Surface): Boolean {
        if (gpuAttached) MiniEngine.nativeDetachSurface(handle)
        surface = s
        gpuAttached = MiniEngine.nativeAttachSurface(handle, s) == MiniEngine.BACKEND_GPU
        refreshBackend()
        if (gpuAttached) {
            Log.i(TAG, "🎨 GPU 上屏：$backendReason")
            return true
        }
        Log.i(TAG, "GPU 没选上（$backendReason），改走 CPU")
        leaveGpu()
        return false
    }

    /** 离开 GPU：放掉合成器与 Surface，换一块从没被 GPU 连过的 Surface 走 CPU */
    private fun leaveGpu() {
        MiniEngine.nativeDetachSurface(handle)
        gpuAttached = false
        surface = null
        mode = Present.WINDOW
        needsPresent = true
        replaceSurface()
    }

    private fun replaceSurface() = main.post {
        if (disposed) return@post
        revealed = false
        ui.newSurface()
    }

    private fun detach() {
        if (handle != 0L) {
            if (gpuAttached) MiniEngine.nativeDetachSurface(handle) else MiniEngine.nativeDetachCpuWindow(handle)
        }
        gpuAttached = false
        surface = null
    }

    /** 当前这块 SurfaceView 上交出了第一帧：让藏着的 SurfaceView 显示出来（只发一次） */
    private fun revealView() {
        if (revealed) return
        revealed = true
        main.post { if (!disposed) ui.reveal() }
    }

    /** 挂到下一个 vsync（同一帧只挂一次） */
    private fun requestFrame() {
        if (frameScheduled || !running || handle == 0L) return
        frameScheduled = true
        choreographer?.postFrameCallback(frameCallback)
    }

    /**
     * 一帧：pump → 上屏 → 取事件。引擎说还有活（`FRAME_WANTS_MORE`：动画 / 惯性 / 定时器 / 请求在飞 /
     * 按住的手指）就挂下一个 vsync，否则停下等输入 —— 静止页面零耗电。
     * 不能只认「这一帧有新画面」：setTimeout / 网络回包在静止页面上会停住
     */
    private fun tick(frameTimeNanos: Long) {
        if (handle == 0L) return
        val t0 = System.nanoTime()
        val f = MiniEngine.nativePumpEx(handle, frameTimeNanos / 1_000_000)
        if (f and MiniEngine.FRAME_BACKEND_CHANGED != 0 && mode == Present.GPU) {
            // 运行中回退（设备丢失 / Surface 丢失…）：引擎这一帧已经改走 CPU 合成
            refreshBackend()
            Log.w(TAG, "⚠️ GPU 回退 CPU：$backendReason")
            leaveGpu()
        }
        if (mode == Present.GPU) {
            // 引擎已经画到 Surface 上了
            needsPresent = false
            if (f and MiniEngine.FRAME_PRESENTED != 0) revealView()
        } else {
            if (f and MiniEngine.FRAME_NEW != 0) needsPresent = true
            if (needsPresent) present()
        }
        if (f and (MiniEngine.FRAME_NEW or MiniEngine.FRAME_PRESENTED) != 0) boost?.report(System.nanoTime() - t0)
        if (f and MiniEngine.FRAME_EVENTS != 0) drainEvents()
        if (f and MiniEngine.FRAME_WANTS_MORE != 0) requestFrame()
    }

    /** CPU 合成好的一帧写进 Surface（GPU 模式不走这里） */
    private fun present() {
        val s = surface ?: return
        if (!s.isValid) return
        when (mode) {
            Present.GPU -> needsPresent = false
            Present.WINDOW -> {
                val r = MiniEngine.nativeBlitToWindow(handle)
                if (r == 1) {
                    needsPresent = false
                    revealView()
                    return
                }
                Log.w(TAG, "CPU 直写 Surface 失败（$r），改走 Bitmap + lockHardwareCanvas")
                MiniEngine.nativeDetachCpuWindow(handle)
                surface = null
                mode = Present.BITMAP
                replaceSurface()
            }
            Present.BITMAP -> {
                val bmp = ensureBitmap()
                if (MiniEngine.nativePixelsToBitmap(handle, bmp) <= 0) return
                val canvas = try {
                    s.lockHardwareCanvas()
                } catch (e: Exception) {
                    Log.w(TAG, "lockHardwareCanvas 失败：$e")
                    return
                }
                try {
                    canvas.drawBitmap(bmp, 0f, 0f, null)
                } finally {
                    s.unlockCanvasAndPost(canvas)
                }
                needsPresent = false
                revealView()
            }
        }
    }

    private fun ensureBitmap(): Bitmap {
        val b = bitmap
        if (b != null && b.width == pixelW && b.height == pixelH) return b
        b?.recycle()
        return Bitmap.createBitmap(pixelW.coerceAtLeast(1), pixelH.coerceAtLeast(1), Bitmap.Config.ARGB_8888)
            .also { bitmap = it }
    }

    /** 取完事件队列，交到主线程分发 */
    private fun drainEvents() {
        val list = ArrayList<String>()
        while (true) list.add(MiniEngine.nativePollEvent(handle) ?: break)
        // 同层渲染的视频（`videos` / `videoCommand`）由 SDK 自带的播放器接（VideoHost，与 Flutter 插件同一份），
        // 照样也交给视图（`videoFullscreen` 由视图收起状态栏）
        for (s in list) if (s.contains("\"type\":\"video")) video().onEvent(s)
        if (list.isNotEmpty()) main.post { if (!disposed) ui.onEvents(list) }
    }

    /** 同层渲染的视频播放器（第一次有 `videos` 事件时才建，没有视频的小程序不多开一条线程）。只在引擎线程上取 */
    private var videoHost: VideoHost? = null

    private fun video(): VideoHost = videoHost ?: VideoHost(
        // 播放器事件送回引擎线程派发给 bind*（也更新引擎画的控制条）
        emit = { key, name, detail -> run { MiniEngine.nativeNativeViewEvent(it, key, name, detail.toString()) } },
        // 新帧到了：引擎只重画那个视频框（全屏时只重新合成）
        onFrame = { engine.post { requestFrame() } },
    ).also { videoHost = it }

    private fun onEngineSync(block: () -> Unit) {
        val latch = CountDownLatch(1)
        engine.post {
            try {
                block()
            } finally {
                latch.countDown()
            }
        }
        latch.await(1, TimeUnit.SECONDS)
    }

    private companion object {
        /** 窗口最多等引擎第一帧这么久（一加 PLY110 上回前台实测 35ms 左右交出来） */
        const val REDRAW_WAIT_MS = 250L
        const val TAG = "mini-render"
    }
}
