package dev.minirender.flutter

import android.content.Context
import android.graphics.Bitmap
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.util.Log
import android.view.Choreographer
import android.view.Surface
import dev.minirender.FrameBoost
import dev.minirender.MiniEngine
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.view.TextureRegistry
import java.io.FileOutputStream
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/**
 * 一个小程序实例在 Android 上的宿主：专用引擎线程 + vsync 出帧 + SurfaceProducer 上屏。
 *
 * ## 线程
 * - **引擎线程**（HandlerThread）：所有 `MiniEngine.native*` 都在这里调 —— 逻辑层是 QuickJS，
 *   不是线程安全的；而且 Flutter 3.29 起 Dart 就跑在主线程上，引擎的一帧放主线程会拖住整个 App。
 *   出帧节拍用这条线程上的 Choreographer，跟着显示器的 vsync 走。
 * - **主线程**：SurfaceProducer 的建 / 改尺寸 / 释放、MethodChannel 回调。
 *
 * ## 出帧闸门
 * 引擎每帧回报 `FRAME_WANTS_MORE`（动画、惯性、定时器、请求在飞、按住的手指…）时才挂下一个
 * vsync；否则停下，等下一条输入 / 命令再 [requestFrame] —— 静止页面零耗电。
 * 从前的原生 View 只在「有新画面」时续帧，`setTimeout` / 网络回包在静止页面上会停住。
 *
 * ## 上屏（三条路，按顺序试）
 * 0. [Present.GPU]（只在宿主选了 auto / gpu 时）：Surface 交给引擎的 GPU 合成器
 *    （`nativeAttachSurface`，wgpu Vulkan/GLES），引擎自己上屏、带 `FRAME_PRESENTED`。
 *    没选上（门槛 / 自检没过）或运行中回退（`FRAME_BACKEND_CHANGED`）：那块 Surface 可能已被
 *    GPU 连过，换一块新的 SurfaceProducer 走第 1 条；
 * 1. [Present.WINDOW]：引擎直接 `ANativeWindow_lock` 写进 Surface，零中间拷贝；
 * 2. [Present.BITMAP]：写进 Bitmap 再 `lockHardwareCanvas` + `drawBitmap`（Flutter 自己的平台视图
 *    就这么往 SurfaceProducer 上画）。SurfaceProducer 在 API 29+ 底层是
 *    `ImageReader(PRIVATE, GPU_SAMPLED)`，第 1 条不保证锁得上；锁失败时那块 Surface 已经被 CPU
 *    连过、不能再给 GPU 生产者用，所以换一块新的 SurfaceProducer（纹理 id 会变，通知 Dart）。
 */
internal class EngineHost(
    /** 只用来开 ADPF 性能提示会话（[FrameBoost]） */
    private val context: Context,
    private val textures: TextureRegistry,
    private val onEvents: (List<String>) -> Unit,
    private val onTexture: (Map<String, Any>) -> Unit,
    /**
     * true：画到自己的 SurfaceView（[EngineView]，平台视图），SurfaceFlinger 直接合成，不经过 Flutter 的帧；
     * false：画到 Flutter 的外部纹理（SurfaceProducer）。为什么默认前者见 EngineView.kt
     */
    private val viewMode: Boolean,
) {
    private enum class Present { GPU, WINDOW, BITMAP }

    private val thread = HandlerThread("mini-render").apply { start() }
    private val engine = Handler(thread.looper)
    private val main = Handler(Looper.getMainLooper())

    // ── 只在引擎线程读写 ──
    private var handle = 0L
    private var choreographer: Choreographer? = null
    /** 引擎线程的性能提示（为什么要见 FrameBoost.kt：复杂页面的补条带在小核上要 5~11ms） */
    private var boost: FrameBoost? = null
    private var frameScheduled = false
    private var running = true
    private var surface: Surface? = null
    private var mode = Present.WINDOW
    /** GPU 模式下 [surface] 已交给引擎的合成器（同一块 Surface 改尺寸不用重挂：合成器自己 reconfigure） */
    private var gpuAttached = false
    private var bitmap: Bitmap? = null
    private var needsPresent = false

    // ── 两条线程都读：像素尺寸（引擎线程写） ──
    @Volatile private var pixelW = 0
    @Volatile private var pixelH = 0

    // ── 只在主线程读写 ──
    private var producer: TextureRegistry.SurfaceProducer? = null
    /** 独立图层模式下的那块视图（Dart 建好平台视图之后才有） */
    private var view: EngineView? = null
    @Volatile private var disposed = false

    private val frameCallback = Choreographer.FrameCallback { frameTimeNanos ->
        frameScheduled = false
        tick(frameTimeNanos)
    }

    /** 主线程调：建 SurfaceProducer，引擎线程上建引擎、打开首页 */
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
        result: MethodChannel.Result,
        onFailed: () -> Unit,
    ) {
        val p = if (viewMode) null else newProducer()
        engine.post {
            choreographer = Choreographer.getInstance()
            boost = FrameBoost.forCurrentThread(context)
            val h = MiniEngine.nativeCreateEx(appDir, dataDir, width, height, dpr, options)
            if (h != 0L) {
                // 机型信息要在挂 surface 之前给（Auto 的机型分级要用）
                MiniEngine.nativeSetDeviceInfo(h, device.totalMem, device.lowRam, device.api)
                MiniEngine.nativeSetBackend(h, backend)
                if (backend != MiniEngine.BACKEND_CPU) mode = Present.GPU
            }
            if (h == 0L || !MiniEngine.nativeLaunch(h, route)) {
                if (h != 0L) MiniEngine.nativeDestroy(h)
                main.post {
                    p?.release()
                    producer = null
                    onFailed()
                    result.error("CREATE_FAILED", "小程序打不开：$appDir（看 logcat 里的 mini-render 报错）", null)
                }
                return@post
            }
            handle = h
            updatePixelSize()
            refreshBackend()
            val (pw, ph) = pixelW to pixelH
            main.post {
                if (disposed) return@post
                if (p == null) {
                    // 独立图层：Surface 等 Dart 建好平台视图（EngineView.surfaceChanged → onViewSurface）
                    view?.setPixelSize(pw, ph)
                    result.success(textureInfo(null))
                    return@post
                }
                p.setSize(pw, ph)
                val s = p.surface
                engine.post { attach(s) }
                result.success(textureInfo(p))
            }
        }
    }

    /** 机型分级用（`nativeSetDeviceInfo`） */
    data class DeviceInfo(val totalMem: Long, val lowRam: Boolean, val api: Int)

    // ── 实际后端与原因（引擎线程写，主线程读） ──
    @Volatile private var activeBackend = MiniEngine.BACKEND_CPU
    @Volatile private var backendReason = ""

    private fun textureInfo(p: TextureRegistry.SurfaceProducer?): Map<String, Any> = listOfNotNull(
        p?.let { "textureId" to it.id() },
        // 独立图层：Dart 建平台视图（viewType [MiniRenderPlugin.VIEW_TYPE]）而不是 Texture
        ("view" to true).takeIf { p == null },
        "pixelWidth" to pixelW,
        "pixelHeight" to pixelH,
        "backend" to activeBackend,
        "backendReason" to backendReason,
        // 出帧节拍诊断（DiagProps.pace）：Dart 侧据此打 Flutter 每一帧的取纹理时刻
        "pace" to DiagProps.pace,
    ).toMap()

    /** 主线程调 */
    private fun newProducer(): TextureRegistry.SurfaceProducer {
        val p = textures.createSurfaceProducer()
        p.setCallback(object : TextureRegistry.SurfaceProducer.Callback {
            override fun onSurfaceAvailable() {
                if (producer !== p || disposed) return
                // 进后台被回收后重建：重新挂上，并把当前帧补画上去
                val s = p.surface
                engine.post { attach(s) }
            }

            override fun onSurfaceCleanup() {
                if (producer !== p) return
                // 返回之后不能再碰这块 Surface：同步等引擎线程放手
                onEngineSync { detach() }
            }
        })
        producer = p
        return p
    }

    /** 主线程调：直写失败后换一块从没被 CPU 连过的 Surface */
    private fun replaceProducer() {
        if (disposed) return
        if (viewMode) {
            // 换一块新的 SurfaceView（surfaceChanged 会再挂上），后端变了告诉 Dart
            view?.newSurface()
            onTexture(textureInfo(null))
            return
        }
        producer?.release()
        val p = newProducer()
        p.setSize(pixelW, pixelH)
        val s = p.surface
        onTexture(textureInfo(p))
        engine.post { attach(s) }
    }

    // ───────────────────────── 引擎线程 ─────────────────────────

    private fun updatePixelSize() {
        val size = MiniEngine.nativePixelSize(handle)
        pixelW = (size ushr 32).toInt()
        pixelH = (size and 0xFFFFFFFFL).toInt()
    }

    private fun attach(s: Surface) {
        if (handle == 0L) return
        if (mode == Present.GPU) {
            if (!(gpuAttached && s === surface) && !attachGpu(s)) return
        } else {
            surface = s
            if (mode == Present.WINDOW && !MiniEngine.nativeAttachCpuWindow(handle, s)) {
                mode = Present.BITMAP
            }
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
            // 纹理没换，但实际后端变了：告诉 Dart（create 的返回早于挂载）
            main.post { if (!disposed && (viewMode || producer != null)) onTexture(textureInfo(producer)) }
            return true
        }
        Log.i(TAG, "GPU 没选上（$backendReason），改走 CPU")
        leaveGpu()
        return false
    }

    /** 离开 GPU 模式：放掉合成器与 Surface，换一块从没被 GPU 连过的 Surface 走 CPU */
    private fun leaveGpu() {
        MiniEngine.nativeDetachSurface(handle)
        gpuAttached = false
        surface = null
        mode = Present.WINDOW
        needsPresent = true
        main.post { replaceProducer() }
    }

    private fun refreshBackend() {
        activeBackend = MiniEngine.nativeBackend(handle)
        backendReason = MiniEngine.nativeBackendReason(handle) ?: ""
    }

    private fun detach() {
        if (handle != 0L) {
            if (gpuAttached) MiniEngine.nativeDetachSurface(handle) else MiniEngine.nativeDetachCpuWindow(handle)
        }
        gpuAttached = false
        surface = null
    }

    private fun requestFrame() {
        if (frameScheduled || !running || handle == 0L) return
        frameScheduled = true
        choreographer?.postFrameCallback(frameCallback)
    }

    private fun tick(frameTimeNanos: Long) {
        if (handle == 0L) return
        val t0 = System.nanoTime()
        val f = MiniEngine.nativePumpEx(handle, frameTimeNanos / 1_000_000)
        if (DiagProps.pace && f and (MiniEngine.FRAME_PRESENTED or MiniEngine.FRAME_NEW) != 0) {
            Log.i(TAG, "🎞 ${frameTimeNanos / 1000} ${System.nanoTime() / 1000}")
        }
        if (f and MiniEngine.FRAME_BACKEND_CHANGED != 0 && mode == Present.GPU) {
            // 运行中回退（设备丢失 / surface 丢失…）：引擎这一帧已经改走 CPU 合成
            refreshBackend()
            Log.w(TAG, "⚠️ GPU 回退 CPU：$backendReason")
            leaveGpu()
        }
        if (mode == Present.GPU) {
            // 引擎已经画到 Surface 上了：只要通知 Flutter 取新的一帧
            needsPresent = false
            if (f and MiniEngine.FRAME_PRESENTED != 0) scheduleFlutterFrame()
        } else {
            if (f and MiniEngine.FRAME_NEW != 0) needsPresent = true
            if (needsPresent) present()
        }
        if (f and (MiniEngine.FRAME_NEW or MiniEngine.FRAME_PRESENTED) != 0) boost?.report(System.nanoTime() - t0)
        if (f and MiniEngine.FRAME_EVENTS != 0) drainEvents()
        if (f and MiniEngine.FRAME_WANTS_MORE != 0) requestFrame()
    }

    /** `notifyFlutter = false`：只写进 Surface，等别处触发的 Flutter 帧来取（改尺寸时用，见 resize） */
    private fun present(notifyFlutter: Boolean = true) {
        val s = surface ?: return
        if (!s.isValid) return
        when (mode) {
            // 引擎自己画到 Surface 上了（tick 只在 FRAME_PRESENTED 时通知 Flutter）
            Present.GPU -> needsPresent = false
            Present.WINDOW -> {
                val t0 = if (DiagProps.fps) System.nanoTime() else 0L
                val r = MiniEngine.nativeBlitToWindow(handle)
                if (t0 != 0L) blitStats.record((System.nanoTime() - t0) / 1e6f)
                if (r == 1) {
                    needsPresent = false
                    if (notifyFlutter) scheduleFlutterFrame()
                    return
                }
                Log.w(TAG, "CPU 直写 Surface 失败（$r），改走 Bitmap + lockHardwareCanvas")
                MiniEngine.nativeDetachCpuWindow(handle)
                surface = null
                mode = Present.BITMAP
                main.post { replaceProducer() }
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
                if (notifyFlutter) scheduleFlutterFrame()
            }
        }
    }

    /** `debug.mini.fps`：每秒一行「上屏」耗时（`ANativeWindow_lock` 等空闲缓冲 + 写像素），引擎的 📊 行不含它 */
    private val blitStats = object {
        var start = 0L
        var n = 0
        var max = 0f
        var sum = 0f
        fun record(ms: Float) {
            val now = System.nanoTime()
            if (start == 0L) start = now
            n++; sum += ms; if (ms > max) max = ms
            if (now - start >= 1_000_000_000L) {
                Log.i(TAG, "📤 上屏 $n 次，平均 ${"%.1f".format(sum / n)}ms 最慢 ${"%.1f".format(max)}ms")
                start = now; n = 0; sum = 0f; max = 0f
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

    /** FlutterJNI 的调用必须在主线程 */
    /** 外部纹理：交了新的一帧要让 Flutter 出一帧来取。独立图层不用（SurfaceFlinger 自己取） */
    private fun scheduleFlutterFrame() {
        if (viewMode) return
        main.post { if (!disposed) producer?.scheduleFrame() }
    }

    private fun drainEvents() {
        val list = ArrayList<String>()
        while (true) list.add(MiniEngine.nativePollEvent(handle) ?: break)
        if (list.isNotEmpty()) main.post { if (!disposed) onEvents(list) }
    }

    /** 在引擎线程上执行一条命令，之后挂一帧（输入之后引擎要出帧回应） */
    private fun run(block: (Long) -> Unit) = engine.post {
        if (handle != 0L) {
            block(handle)
            requestFrame()
        }
    }

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

    // ───────────────────────── 命令（主线程进来） ─────────────────────────

    fun handle(call: MethodCall, result: MethodChannel.Result) {
        fun f(k: String) = (call.argument<Number>(k) ?: 0).toFloat()
        fun i(k: String) = (call.argument<Number>(k) ?: 0).toInt()
        when (call.method) {
            "pointer" -> {
                val phase = i("phase")
                // 移动带事件时间（Dart PointerEvent.timeStamp，安卓上就是 MotionEvent.getEventTime()）：
                // 引擎按它重采样页面拖动。经 MethodChannel 再转引擎线程，一帧到几个移动是碰运气，
                // 按到达时刻走就是慢拖时的一顿一顿（engine_resample.rs）
                val t = call.argument<Number>("t")?.toDouble()
                if (phase == 1 && t != null) {
                    run { MiniEngine.nativePointerMoveAt(it, f("x"), f("y"), t) }
                } else {
                    run { MiniEngine.nativePointer(it, phase, f("x"), f("y")) }
                }
                result.success(null)
            }
            "wheel" -> {
                run { MiniEngine.nativeWheel(it, f("dy"), call.argument<Boolean>("precise") ?: true) }
                result.success(null)
            }
            "text" -> {
                run { MiniEngine.nativeTextInput(it, call.argument<String>("text") ?: "") }
                result.success(null)
            }
            "key" -> {
                run { MiniEngine.nativeKey(it, call.argument<String>("name") ?: "") }
                result.success(null)
            }
            "navigate" -> {
                run { MiniEngine.nativeNavigate(it, call.argument<String>("url") ?: "") }
                result.success(null)
            }
            "back" -> engine.post {
                val ok = handle != 0L && MiniEngine.nativeBack(handle)
                requestFrame()
                main.post { result.success(ok) }
            }
            "resize" -> {
                val (w, h, dpr) = Triple(i("width"), i("height"), f("dpr"))
                val asked = System.nanoTime()
                engine.post {
                    val t0 = System.nanoTime()
                    if (handle == 0L || !MiniEngine.nativeResize(handle, w, h, dpr)) {
                        main.post { result.success(null) }
                        return@post
                    }
                    val t1 = System.nanoTime()
                    updatePixelSize()
                    Log.i(TAG, "resize ${w}x$h：排队 ${(t0 - asked) / 1_000_000}ms，重排 ${(t1 - t0) / 1_000_000}ms")
                    main.post {
                        if (viewMode) {
                            // 缓冲区改尺寸 → surfaceChanged → onViewSurface 再挂一次（同一块 Surface 不重建合成器）
                            view?.setPixelSize(pixelW, pixelH)
                            engine.post { needsPresent = true; requestFrame() }
                            result.success(textureInfo(null))
                            return@post
                        }
                        val p = producer ?: return@post result.success(null)
                        p.setSize(pixelW, pixelH)
                        val s = p.surface
                        engine.post {
                            attach(s)
                            // 新尺寸的那一帧**先**写进 Surface（不通知 Flutter），再把新尺寸回给 Dart：
                            // Dart 按新尺寸重建 Texture 的那一帧取到的就是新画面。
                            // 从前是先回尺寸、下一个 vsync 才画 —— 中间那一帧旧画面被拉伸到新尺寸，
                            // 切换导航栏样式（AppBar 出现 / 消失）时整页先放大一下再弹回来。
                            // GPU 模式：引擎下一帧整帧合成，合成器按新尺寸自己 reconfigure（照旧）
                            if (mode != Present.GPU) present(notifyFlutter = false)
                            main.post { result.success(textureInfo(p)) }
                        }
                    }
                }
            }
            "safeArea" -> {
                run {
                    val t0 = System.nanoTime()
                    if (MiniEngine.nativeSetSafeArea(it, f("top"), f("right"), f("bottom"), f("left"))) {
                        Log.i(TAG, "safeArea top=${f("top")}：重排 ${(System.nanoTime() - t0) / 1_000_000}ms")
                    }
                }
                result.success(null)
            }
            "lifecycle" -> {
                val what = i("what")
                engine.post {
                    if (handle == 0L) return@post
                    MiniEngine.nativeLifecycle(handle, what)
                    when (what) {
                        0 -> {
                            running = true
                            needsPresent = true
                            requestFrame()
                        }
                        1 -> running = false // 进后台：停止出帧（继续出帧纯属耗电）
                    }
                }
                result.success(null)
            }
            "hostReply" -> {
                val id = i("id")
                val ok = call.argument<Boolean>("ok") ?: false
                val res = call.argument<String>("result")
                run { MiniEngine.nativeHostReply(it, id, ok, res) }
                result.success(null)
            }
            "nativeViewEvent" -> {
                val key = call.argument<String>("key") ?: ""
                val name = call.argument<String>("name") ?: ""
                val detail = call.argument<String>("detail")
                run { MiniEngine.nativeNativeViewEvent(it, key, name, detail) }
                result.success(null)
            }
            "eval" -> {
                val code = call.argument<String>("code") ?: ""
                engine.post {
                    val r = if (handle != 0L) MiniEngine.nativeEval(handle, code) else null
                    main.post { result.success(r) }
                }
            }
            "dumpFrame" -> {
                val path = call.argument<String>("path") ?: ""
                engine.post {
                    var ok = false
                    if (handle != 0L && pixelW > 0 && pixelH > 0) {
                        val bmp = Bitmap.createBitmap(pixelW, pixelH, Bitmap.Config.ARGB_8888)
                        if (MiniEngine.nativePixelsToBitmap(handle, bmp) > 0) {
                            ok = FileOutputStream(path).use { bmp.compress(Bitmap.CompressFormat.PNG, 100, it) }
                        }
                        bmp.recycle()
                    }
                    main.post { result.success(ok) }
                }
            }
            else -> result.notImplemented()
        }
    }

    // ───────────────────────── 独立图层（EngineView，主线程调） ─────────────────────────

    fun bindView(v: EngineView) {
        view = v
        if (pixelW > 0 && pixelH > 0) v.setPixelSize(pixelW, pixelH)
    }

    fun unbindView(v: EngineView) {
        if (view === v) view = null
    }

    /** SurfaceView 的 Surface 可用 / 改了尺寸 */
    fun onViewSurface(s: Surface) {
        if (disposed) return
        engine.post { attach(s) }
    }

    /** SurfaceView 的 Surface 要没了：同步等引擎线程放手 */
    fun onViewSurfaceGone() {
        if (disposed) return
        onEngineSync { detach() }
    }

    /** 主线程调：先让引擎线程放手 Surface 并销毁引擎，再放 SurfaceProducer */
    fun dispose() {
        if (disposed) return
        disposed = true
        onEngineSync {
            choreographer?.removeFrameCallback(frameCallback)
            detach()
            if (handle != 0L) MiniEngine.nativeDestroy(handle)
            handle = 0L
            boost?.close()
            boost = null
            bitmap?.recycle()
            bitmap = null
        }
        producer?.release()
        producer = null
        thread.quitSafely()
    }

    private companion object {
        const val TAG = "mini-render"
    }
}
