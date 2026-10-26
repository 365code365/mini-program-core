package dev.minirender.flutter

import android.content.Context
import android.util.Log
import android.view.Surface
import dev.minirender.EngineCore
import dev.minirender.MiniEngine
import dev.minirender.dumpFrame
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.view.TextureRegistry

/**
 * 一个小程序实例在 Android 上的 Flutter 宿主：MethodChannel 与 SurfaceProducer / 平台视图那一层适配。
 *
 * 引擎线程、vsync 出帧闸门、三条上屏路（GPU → CPU 直写 → Bitmap）、事件与同层视频都在
 * [EngineCore]（sdk/android，与原生视图 `MiniProgramView` 同一份，规则只写一次）。这里只管：
 * - Dart 的命令（[handle]）→ 引擎线程；
 * - 上屏目标：独立图层（[EngineView]，默认）或 Flutter 外部纹理（SurfaceProducer）。
 *   外部纹理每交一帧要让 Flutter 出一帧来取（[EngineCore.Hooks.onPresented] → [scheduleFlutterFrame]）；
 *   SurfaceProducer 在 API 29+ 底层是 `ImageReader(PRIVATE, GPU_SAMPLED)`，CPU 直写不保证锁得上，
 *   锁失败时换一块新的 SurfaceProducer（纹理 id 会变，通知 Dart）。
 *
 * Flutter 3.29 起 Dart 就跑在主线程上，引擎的一帧放主线程会拖住整个 App —— 所以一切都在引擎线程。
 */
internal class EngineHost(
    /** 只用来开 ADPF 性能提示会话（FrameBoost） */
    context: Context,
    private val textures: TextureRegistry,
    private val onEvents: (List<String>) -> Unit,
    private val onTexture: (Map<String, Any>) -> Unit,
    /**
     * true：画到自己的 SurfaceView（[EngineView]，平台视图），SurfaceFlinger 直接合成，不经过 Flutter 的帧；
     * false：画到 Flutter 的外部纹理（SurfaceProducer）。为什么默认前者见 EngineView.kt
     */
    private val viewMode: Boolean,
) {
    private val core = EngineCore(
        context,
        object : EngineCore.Hooks {
            override fun reveal() {
                view?.reveal()
            }
            override fun newSurface() = replaceProducer()
            override fun onEvents(list: List<String>) = this@EngineHost.onEvents(list)
            override fun onPresented() = scheduleFlutterFrame()
            // 纹理没换，但实际后端变了：告诉 Dart（create 的返回早于挂载）
            override fun onGpuAttached() {
                main.post { if (!disposed && (viewMode || producer != null)) onTexture(textureInfo(producer)) }
            }
        },
        EngineCore.Options(revealsView = viewMode, pace = DiagProps.pace, blitStats = DiagProps.fps),
    )
    private val engine: android.os.Handler get() = core.engine
    private val main: android.os.Handler get() = core.main
    private val disposed: Boolean get() = core.disposed

    // ── 引擎线程（只在 engine.post / run 的块里用） ──
    private val handle get() = core.handle
    private fun requestFrame() = core.requestFrame()
    private fun updatePixelSize() = core.updatePixelSize()
    private fun attach(s: Surface) = core.attach(s)
    private fun run(block: (Long) -> Unit) = core.run(block)

    // ── 两条线程都读：像素尺寸（引擎线程写） ──
    private val pixelW get() = core.pixelW
    private val pixelH get() = core.pixelH

    /** 同层组件交帧用的直接缓冲区（主线程，`viewFrame`），按最大尺寸复用 */
    private var viewFrameBuf: java.nio.ByteBuffer? = null

    // ── 只在主线程读写 ──
    private var producer: TextureRegistry.SurfaceProducer? = null
    /** 独立图层模式下的那块视图（Dart 建好平台视图之后才有） */
    private var view: EngineView? = null

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
        core.start(
            appDir, dataDir, route, width, height, dpr, options, backend,
            EngineCore.DeviceInfo(device.totalMem, device.lowRam, device.api),
        ) { error ->
            if (error != null) {
                main.post {
                    p?.release()
                    producer = null
                    onFailed()
                    result.error("CREATE_FAILED", "小程序打不开：$appDir（$error，看 logcat 里的 mini-render 报错）", null)
                }
                return@start
            }
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

    private fun textureInfo(p: TextureRegistry.SurfaceProducer?): Map<String, Any> = listOfNotNull(
        p?.let { "textureId" to it.id() },
        // 独立图层：Dart 建平台视图（viewType [MiniRenderPlugin.VIEW_TYPE]）而不是 Texture
        ("view" to true).takeIf { p == null },
        "pixelWidth" to pixelW,
        "pixelHeight" to pixelH,
        "backend" to core.activeBackend,
        "backendReason" to core.backendReason,
        // 出帧节拍诊断（DiagProps.pace）：Dart 侧据此打 Flutter 每一帧的取纹理时刻
        "pace" to DiagProps.pace,
        // 原生组件比引擎画面晚上屏几个 vsync 的校准值（DiagProps.nativeLag，没设就不带）
        DiagProps.nativeLag?.let { "nativeLag" to it },
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
                core.surfaceGone()
            }
        })
        producer = p
        return p
    }

    /** 主线程调（[EngineCore.Hooks.newSurface]）：这块 Surface 被另一种生产者连过，换一块新的 */
    private fun replaceProducer() {
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

    /** 外部纹理：交了新的一帧要让 Flutter 出一帧来取（FlutterJNI 的调用必须在主线程）。独立图层不用 */
    private fun scheduleFlutterFrame() {
        if (viewMode) return
        main.post { if (!disposed) producer?.scheduleFrame() }
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
            // 第二根手指（view.dart 只在两指同时按着时送）：引擎只给图片预览用（双指缩放）
            "pointer2" -> {
                val phase = call.argument<Int>("phase") ?: 3
                run { MiniEngine.nativePointer2(it, phase, f("x"), f("y")) }
                result.success(null)
            }
            "keyboardShift" -> {
                // 键盘顶起画面：引擎合成时导航栏以下整体上移、自己缓动（与原生 SDK 的 MiniKeyboard 同一个接口）
                run { MiniEngine.nativeSetKeyboardShift(it, f("shift")) }
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
                            engine.post { core.markNeedsPresent(); requestFrame() }
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
                            if (!core.isGpu) core.present(notify = false)
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
                // 0 进前台（补画当前帧）/ 1 进后台（停止出帧、视频停下）/ 2 内存告警：与原生视图同一份（EngineCore.lifecycle）
                core.lifecycle(i("what"))
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
            "viewFrame" -> {
                // 同层渲染的组件（Dart 离屏画好的一帧，RGBA）：交给引擎（任何线程都行），引擎线程出一帧只重画那一块
                val key = call.argument<String>("key") ?: ""
                val (w, h) = i("width") to i("height")
                val data = call.argument<ByteArray>("data")
                if (data != null && w > 0 && h > 0 && data.size >= w * h * 4) {
                    // 直接缓冲区复用（只在主线程用；引擎在 nativeViewFrameRgba 里就拷走了）：
                    // 从前每帧 allocateDirect 一块几 MB 的堆外内存，回收要等 GC 的 Cleaner
                    val need = w * h * 4
                    val buf = viewFrameBuf?.takeIf { it.capacity() >= need }
                        ?: java.nio.ByteBuffer.allocateDirect(need).also { viewFrameBuf = it }
                    buf.clear()
                    buf.put(data, 0, need).rewind()
                    if (MiniEngine.nativeViewFrameRgba(key, w, h, w * 4, buf)) engine.post { requestFrame() }
                }
                result.success(null)
            }
            // 宿主推事件给 wx.onXxx（传感器、定位、网络、主题……）：引擎线程上派发，回监听者个数
            "dispatchEvent" -> {
                val name = call.argument<String>("name") ?: ""
                val payload = call.argument<String>("payload")
                engine.post {
                    val n = if (handle != 0L) MiniEngine.nativeDispatchEvent(handle, name, payload) else 0
                    if (handle != 0L) requestFrame()
                    main.post { result.success(n) }
                }
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
                core.dumpFrame(java.io.File(path)) { result.success(it) }
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
    fun onViewSurface(s: Surface) = core.attachSurface(s)

    /** SurfaceView 要窗口等它这一帧（Callback2，见 EngineView 与 [EngineCore.redrawNeeded]） */
    fun onViewRedrawNeeded(done: Runnable) = core.redrawNeeded(done)

    /** SurfaceView 的 Surface 要没了：同步等引擎线程放手 */
    fun onViewSurfaceGone() = core.surfaceGone()

    /** 主线程调：先让引擎线程放手 Surface 并销毁引擎，再放 SurfaceProducer */
    fun dispose() {
        if (disposed) return
        core.dispose()
        producer?.release()
        producer = null
    }

    private companion object {
        const val TAG = "mini-render"
    }
}
