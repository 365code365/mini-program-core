package dev.minirender

import android.content.Context
import android.graphics.Bitmap
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.util.Log
import android.view.Choreographer
import android.view.Surface
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

/**
 * 两份 Android 宿主共用的引擎内核：专用引擎线程 + vsync 出帧 + 往 Surface 上屏 + 事件。
 *
 * 原生视图（[EngineSession] → [MiniProgramView]）与 Flutter 插件（`dev.minirender.flutter.EngineHost`）
 * 从前各有一份几乎逐行相同的出帧 / 上屏代码，改了一边另一边常常没跟上（踩过：预约 vsync、
 * 后台事件的先后、诊断日志只有一边有）。现在两边都只是适配层：原生那边接 SurfaceView，
 * Flutter 那边接 MethodChannel 与 SurfaceProducer / 平台视图；出帧的规则只在这里写一次。
 * 每一条为什么要这么做见 doc/滑动顿挫避坑清单.md。
 *
 * ## 线程
 * - **引擎线程**（HandlerThread）：所有 `MiniEngine.native*` 都在这里调（逻辑层是 QuickJS，不是线程安全的）。
 *   出帧节拍用这条线程上的 Choreographer；上屏也在这条线程上，**不经过 UI 线程**。
 *   标了「引擎线程」的成员只能在 [onEngine] / [run] 的块里碰。
 * - **主线程**：Surface 的建 / 改尺寸 / 显示，[Hooks] 里除了标明的都在主线程回调。
 *
 * ## 出帧闸门
 * 引擎每帧回报 `FRAME_WANTS_MORE`（动画、惯性、定时器、请求在飞、按住的手指…）时才挂下一个 vsync，
 * 否则停下等输入 —— 静止页面零耗电。不能只认「这一帧有新画面」：`setTimeout` / 网络回包会停住。
 *
 * ## 上屏（三条路，按顺序试）
 * 0. GPU（偏好 auto / gpu）：Surface 交给引擎的 GPU 合成器，引擎自己上屏（`FRAME_PRESENTED`）。
 *    没选上或运行中回退（`FRAME_BACKEND_CHANGED`）就降到第 1 条；
 * 1. WINDOW：引擎 `ANativeWindow_lock` 直接写进 Surface，一次拷贝；锁不上降到第 2 条；
 * 2. BITMAP：写进 Bitmap 再 `lockHardwareCanvas` + `drawBitmap`。
 * 一块 Surface 被一种生产者（GPU / CPU）连过之后不能再给另一种用：降级时让宿主换一块新的（[Hooks.newSurface]）。
 */
internal class EngineCore(
    /** 只用来开 ADPF 性能提示会话（[FrameBoost]） */
    private val context: Context,
    private val hooks: Hooks,
    private val opts: Options = Options(),
) {
    interface Hooks {
        /** 引擎在当前这块 Surface 上交出了第一帧：把藏着的视图显示出来（[Options.revealsView] 为 false 时不调） */
        fun reveal()
        /** 当前这块 Surface 已被另一种生产者连过：换一块新的，建好后再 [attachSurface] */
        fun newSurface()
        fun onEvents(list: List<String>)
        /** **引擎线程**：一帧已经交给 Surface（GPU 自己上屏，或 CPU 写完）。外部纹理要据此让 Flutter 出一帧来取 */
        fun onPresented() {}
        /** **引擎线程**：Surface 交给 GPU 合成器成功（实际后端变了） */
        fun onGpuAttached() {}
    }

    data class Options(
        /** 宿主的 Surface 是独立图层、首帧前藏着（SurfaceView）：交出第一帧时 [Hooks.reveal] */
        val revealsView: Boolean = true,
        /** 每交出一帧打一行 `🎞 <vsync µs> <交出 µs> <tick 开始 µs>`（`debug.mini.pace`，tools/pace-check.py 读） */
        val pace: Boolean = false,
        /** 每秒一行 CPU 直写上屏的耗时（`debug.mini.fps`）：`ANativeWindow_lock` 等空闲缓冲 + 写像素 */
        val blitStats: Boolean = false,
    )

    /** 机型分级用（`nativeSetDeviceInfo`，auto 后端据此决定开不开 GPU） */
    data class DeviceInfo(val totalMem: Long, val lowRam: Boolean, val api: Int)

    private enum class Present { GPU, WINDOW, BITMAP }

    private val thread = HandlerThread("mini-render").apply { start() }
    /** 引擎线程的 Handler（宿主的命令块 `engine.post { … }` 直接往这里投） */
    val engine = Handler(thread.looper)
    val main = Handler(Looper.getMainLooper())

    // ── 只在引擎线程读写 ──
    /** **引擎线程**：引擎句柄，0 = 还没建好 / 已销毁 */
    var handle = 0L
        private set
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
    /** 上一帧回报了 `FRAME_WANTS_MORE`，见 [tick] */
    private var wantedMore = false

    /** 引擎已在当前这块 Surface 上交过帧（换新 Surface 时清掉） */
    @Volatile private var revealed = false
    @Volatile var disposed = false
        private set

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

    /** **引擎线程**：当前走 GPU 上屏 */
    val isGpu get() = mode == Present.GPU

    private val frameCallback = Choreographer.FrameCallback { frameTimeNanos ->
        frameScheduled = false
        tick(frameTimeNanos)
    }

    /**
     * 引擎线程上建引擎、打开首页。[beforeWarmup] 在引擎读环境之前跑（诊断开关要先就位）。
     * [done] 在引擎线程上回调：null 成功，否则是失败原因（引擎已销毁）
     */
    fun start(
        appDir: String,
        dataDir: String,
        route: String?,
        width: Int,
        height: Int,
        dpr: Float,
        options: String,
        backend: Int,
        device: DeviceInfo,
        beforeWarmup: () -> Unit = {},
        done: (error: String?) -> Unit,
    ) = engine.post {
        if (disposed) return@post done("已销毁")
        choreographer = Choreographer.getInstance()
        boost = FrameBoost.forCurrentThread(context)
        beforeWarmup()
        // 字体（约 0.35s）与 GPU（约 0.5s）跟建引擎并行
        MiniEngine.nativeWarmup(width, height, dpr, backend, device.totalMem, device.lowRam, device.api)
        val h = MiniEngine.nativeCreateEx(appDir, dataDir, width, height, dpr, options)
        if (h == 0L) return@post done("建引擎失败")
        // 机型信息要在挂 Surface 之前给（auto 的机型分级要用）
        MiniEngine.nativeSetDeviceInfo(h, device.totalMem, device.lowRam, device.api)
        MiniEngine.nativeSetBackend(h, backend)
        mode = if (backend != MiniEngine.BACKEND_CPU) {
            // 开 GPU 与下面的首页启动并行：Surface 建出来时合成器多半已经好了
            MiniEngine.nativePrewarmGpu(h)
            Present.GPU
        } else {
            Present.WINDOW
        }
        if (!MiniEngine.nativeLaunch(h, route)) {
            MiniEngine.nativeDestroy(h)
            return@post done("首页没打开")
        }
        handle = h
        ready = true
        updatePixelSize()
        refreshBackend()
        done(null)
    }

    // ───────────────────────── 命令（主线程进来） ─────────────────────────

    /** 在引擎线程上跑一段（不挂帧）。块里可以碰标了「引擎线程」的成员 */
    fun onEngine(block: () -> Unit) = engine.post(block)

    /** 在引擎线程上执行一条命令，之后挂一帧（输入之后引擎要出帧回应） */
    fun run(block: (Long) -> Unit) = engine.post {
        if (handle != 0L) {
            block(handle)
            requestFrame()
        }
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

    // ───────────────────────── Surface（主线程进来） ─────────────────────────

    /** Surface 可用 / 改了尺寸 */
    fun attachSurface(s: Surface) {
        if (!disposed) engine.post { attach(s) }
    }

    /** Surface 要没了：返回之后不能再碰它，同步等引擎线程放手 */
    fun surfaceGone() {
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
    fun redrawNeeded(done: Runnable) {
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

    /** 主线程调：先让引擎线程放手 Surface 并销毁引擎，再停线程。[onEngineThread] 在引擎线程上最后跑 */
    fun dispose(onEngineThread: () -> Unit = {}) {
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
            onEngineThread()
        }
        thread.quitSafely()
    }

    // ───────────────────────── 引擎线程 ─────────────────────────

    /** **引擎线程**：从引擎取一次像素尺寸（建好 / 改尺寸之后） */
    fun updatePixelSize() {
        val size = MiniEngine.nativePixelSize(handle)
        pixelW = (size ushr 32).toInt()
        pixelH = (size and 0xFFFFFFFFL).toInt()
    }

    private fun refreshBackend() {
        activeBackend = MiniEngine.nativeBackend(handle)
        backendReason = MiniEngine.nativeBackendReason(handle) ?: ""
    }

    /** **引擎线程**：挂上一块 Surface（[attachSurface] 的引擎线程那一半，改尺寸时宿主要先挂再画） */
    fun attach(s: Surface) {
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
            hooks.onGpuAttached()
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
        hooks.newSurface()
    }

    private fun detach() {
        if (handle != 0L) {
            if (gpuAttached) MiniEngine.nativeDetachSurface(handle) else MiniEngine.nativeDetachCpuWindow(handle)
        }
        gpuAttached = false
        surface = null
    }

    /** 当前这块 Surface 上交出了第一帧：让藏着的视图显示出来（只发一次） */
    private fun revealView() {
        if (!opts.revealsView || revealed) return
        revealed = true
        main.post { if (!disposed) hooks.reveal() }
    }

    /** **引擎线程**：这一帧要重新上屏（改了尺寸、回前台） */
    fun markNeedsPresent() {
        needsPresent = true
    }

    /** **引擎线程**：挂到下一个 vsync（同一帧只挂一次） */
    fun requestFrame() {
        if (frameScheduled || !running || handle == 0L) return
        frameScheduled = true
        choreographer?.postFrameCallback(frameCallback)
    }

    /**
     * 一帧：pump → 上屏 → 取事件。引擎说还有活（`FRAME_WANTS_MORE`）就挂下一个 vsync，否则停下等输入。
     *
     * 上一帧回报了 `WANTS_MORE`（拖动 / 惯性 / 动画里）就在 tick **开头**先挂下一个 vsync：从前在最后才挂，
     * 这一帧一超过一个周期，下一个 vsync 已经过去，Choreographer 只能排到再下一个 —— 白白丢一帧。
     * 一加 PLY110 `query-slow`（90Hz）实测：屏上 21 次重复里 20 次正是「上一帧 vsync→交出 11~13ms、
     * 下一次 tick 隔了 22ms」。先挂上的话回调就在消息队列里，这一帧一画完马上接着画下一帧，
     * 而 BufferQueue 本来就有一两帧的余量（入队到上屏约 25ms），赶得上它的那一格
     */
    private fun tick(frameTimeNanos: Long) {
        if (handle == 0L) return
        val speculative = wantedMore
        if (speculative) requestFrame()
        val t0 = System.nanoTime()
        val f = MiniEngine.nativePumpEx(handle, frameTimeNanos / 1_000_000)
        if (opts.pace && f and (MiniEngine.FRAME_PRESENTED or MiniEngine.FRAME_NEW) != 0) {
            // 第三列是 tick 开始的时刻：「vsync→交出」长，是 pump 慢还是这条线程上前面有别的活压着，靠它分
            Log.i(TAG, "🎞 ${frameTimeNanos / 1000} ${System.nanoTime() / 1000} ${t0 / 1000}")
        }
        if (f and MiniEngine.FRAME_BACKEND_CHANGED != 0 && mode == Present.GPU) {
            // 运行中回退（设备丢失 / Surface 丢失…）：引擎这一帧已经改走 CPU 合成
            refreshBackend()
            Log.w(TAG, "⚠️ GPU 回退 CPU：$backendReason")
            leaveGpu()
        }
        if (mode == Present.GPU) {
            // 引擎已经画到 Surface 上了
            needsPresent = false
            if (f and MiniEngine.FRAME_PRESENTED != 0) {
                hooks.onPresented()
                revealView()
            }
        } else {
            if (f and MiniEngine.FRAME_NEW != 0) needsPresent = true
            if (needsPresent) present()
        }
        if (f and (MiniEngine.FRAME_NEW or MiniEngine.FRAME_PRESENTED) != 0) boost?.report(System.nanoTime() - t0)
        if (f and MiniEngine.FRAME_EVENTS != 0) drainEvents()
        wantedMore = f and MiniEngine.FRAME_WANTS_MORE != 0
        if (wantedMore) {
            requestFrame()
        } else if (speculative && frameScheduled) {
            // 预约落空（动画 / 惯性这一帧刚停）：撤掉，静止页面照旧零耗电。
            // 这一帧里不会有别的请求（输入 / 命令都排在这条线程的消息队列后面，之后自己会再挂）
            choreographer?.removeFrameCallback(frameCallback)
            frameScheduled = false
        }
    }

    /**
     * **引擎线程**：CPU 合成好的一帧写进 Surface（GPU 模式不走这里）。
     * `notify = false`：写完不调 [Hooks.onPresented]（Flutter 外部纹理改尺寸时，先写好、回了尺寸再通知）
     */
    fun present(notify: Boolean = true) {
        val s = surface ?: return
        if (!s.isValid) return
        when (mode) {
            Present.GPU -> needsPresent = false
            Present.WINDOW -> {
                val t0 = if (opts.blitStats) System.nanoTime() else 0L
                val r = MiniEngine.nativeBlitToWindow(handle)
                if (t0 != 0L) blitStats.record((System.nanoTime() - t0) / 1e6f)
                if (r == 1) {
                    needsPresent = false
                    if (notify) hooks.onPresented()
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
                if (notify) hooks.onPresented()
                revealView()
            }
        }
    }

    private val blitStats = BlitStats()

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
        // 同层渲染的视频（`videos` / `videoCommand`）由 SDK 自带的播放器接（VideoHost），
        // 照样也交给宿主（`videoFullscreen` 由宿主收起状态栏）
        for (s in list) if (s.contains("\"type\":\"video")) video().onEvent(s)
        if (list.isNotEmpty()) main.post { if (!disposed) hooks.onEvents(list) }
    }

    /** 同层渲染的视频播放器（第一次有 `videos` 事件时才建，没有视频的小程序不多开一条线程）。只在引擎线程上取 */
    private var videoHost: VideoHost? = null

    private fun video(): VideoHost = videoHost ?: VideoHost(
        // 播放器事件送回引擎线程派发给 bind*（也更新引擎画的控制条）
        emit = { key, name, detail -> run { MiniEngine.nativeNativeViewEvent(it, key, name, detail.toString()) } },
        // 新帧到了：引擎只重画那个视频框（全屏时只重新合成）
        onFrame = { engine.post { requestFrame() } },
    ).also { videoHost = it }

    /** 主线程调：同步等引擎线程跑完 [block]（最多 1 秒） */
    fun onEngineSync(block: () -> Unit) {
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
