package dev.minirender

import android.content.Context
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.util.Log
import android.view.Choreographer
import android.view.MotionEvent
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.widget.FrameLayout
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/**
 * 小游戏的宿主视图（game-doc/02 §12）：一块全屏 SurfaceView + 专用引擎线程 + 引擎线程上的 Choreographer。
 *
 * 与 [EngineCore]（小程序）同一套规则（steering 30「宿主一律走独立图层 + 引擎线程」）：
 * - 上屏在引擎线程上直写 Surface（`ANativeWindow_lock`），不经 UI 线程的 onDraw；
 * - tick 开头就预约下一个 vsync（上一帧要更多的话）：最后才挂会让超时的那一帧白白丢掉一个 vsync（避坑清单第 16 条）；
 * - 主画布按自己的分辨率写进 Surface，由 SurfaceFlinger 的硬件缩放器拉到全屏，CPU 不做缩放；
 * - `Surface.setFrameRate(rAF 帧率, FIXED_SOURCE)`：系统选 60 的整数倍的显示模式，90Hz 屏上 60fps 不会一长一短。
 *
 * 用法：`MiniGameView(ctx, dir, dataDir, optionsJson)`，Activity 的 onResume / onPause 转给 [onResume] / [onPause]，
 * onDestroy 调 [dispose]。
 */
class MiniGameView(
    context: Context,
    private val dir: String,
    private val dataDir: String,
    private val options: String = "{}",
) : FrameLayout(context) {
    private val main = Handler(Looper.getMainLooper())
    private val thread = HandlerThread("mini-game").apply { start() }
    private val engine = Handler(thread.looper)

    // ── 引擎线程上的状态 ──
    private var handle = 0L
    private var choreographer: Choreographer? = null
    private var scheduled = false
    private var wantedMore = false
    private var running = true
    private var surface: Surface? = null
    private var appliedFps = 0
    private var boost: FrameBoost? = null
    private val slowLog = SystemProperties_get("debug.mini.game_log") == "1"
    /** `adb shell setprop debug.mini.touch_log 1`：每帧打「最早一个还没派发的触摸 → 这次 pump」隔了多久（A3.3：≤ 1 个 vsync） */
    private val touchLog = SystemProperties_get("debug.mini.touch_log") == "1"
    /** `adb shell setprop debug.mini.audio_log 1`：每 2 秒打一行音频输出统计（延迟、欠载、回调块） */
    private val audioLog = SystemProperties_get("debug.mini.audio_log") == "1"
    private var lastAudioLog = 0L
    /** 引擎线程：排队等派发的触摸里最早的那个的事件时刻（ns，CLOCK_MONOTONIC），0 = 没有 */
    private var pendingTouchNanos = 0L

    @Volatile private var disposed = false

    /**
     * 引擎交给宿主的事件（日志、audioSession 之外的：`exit` / `restart`（wx.exitMiniProgram / restartMiniProgram）、
     * 数据上报 `report` 等），在主线程上回调：type 与原始 JSON
     */
    var onHostEvent: ((type: String, event: org.json.JSONObject) -> Unit)? = null

    /** 音频焦点与来电（[GameAudioSession]），引擎线程上建、用、放 */
    private var audioSession: GameAudioSession? = null

    private val frame = Choreographer.FrameCallback { t ->
        scheduled = false
        tick(t)
    }

    private val sv = SurfaceView(context).also { v ->
        addView(v, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
        v.holder.addCallback(object : SurfaceHolder.Callback {
            override fun surfaceCreated(holder: SurfaceHolder) {}
            override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
                val s = holder.surface
                engine.post { attach(s) }
            }
            // 返回之后不能再碰这块 Surface：同步等引擎线程放手
            override fun surfaceDestroyed(holder: SurfaceHolder) = onEngineSync { detach() }
        })
    }

    init {
        // 建实例之前就按 game.json 锁方向：否则第一帧拿到竖屏尺寸，横屏包要先错一次再 resize（G6.3）
        val o = try { org.json.JSONObject(java.io.File(dir, "game.json").readText()).optString("deviceOrientation", "portrait") } catch (_: Exception) { "portrait" }
        // rotateInEngine：宿主不转（引擎自己把横屏画面转进竖屏，game/rotate.rs），锁竖屏
        val inEngine = try { org.json.JSONObject(options).optBoolean("rotateInEngine") } catch (_: Exception) { false }
        lockOrientation(if (inEngine) "portrait" else o)
        post { start() }
    }

    /** game.json / wx.setDeviceOrientation 的方向 → Activity 的 requestedOrientation（宿主不是 Activity 时不管） */
    private fun lockOrientation(value: String) {
        val a = context as? android.app.Activity ?: return
        a.requestedOrientation = when (value) {
            "landscape" -> android.content.pm.ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
            // 官方：landscapeLeft = home 键在左（设备顺时针转），landscapeRight = home 键在右
            "landscapeLeft" -> android.content.pm.ActivityInfo.SCREEN_ORIENTATION_REVERSE_LANDSCAPE
            "landscapeRight" -> android.content.pm.ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE
            else -> android.content.pm.ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
        }
    }

    /** 尺寸变了（转屏、分屏）：告诉引擎新的逻辑尺寸与方向（派发 wx.onWindowResize / onDeviceOrientationChange） */
    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        if (oldw == 0 && oldh == 0) return // 第一次布局：start() 按它建实例
        val d = resources.displayMetrics.density
        val rot = (if (Build.VERSION.SDK_INT >= 30) display?.rotation else null) ?: android.view.Surface.ROTATION_0
        val orient = when (rot) {
            android.view.Surface.ROTATION_90 -> "landscape"
            android.view.Surface.ROTATION_270 -> "landscapeReverse"
            else -> if (w > h) "landscape" else "portrait"
        }
        val lw = (w / d).toInt().coerceAtLeast(1)
        val lh = (h / d).toInt().coerceAtLeast(1)
        engine.post {
            if (handle == 0L) return@post
            MiniGame.nativeResize(handle, lw, lh, d, orient)
            drainEvents()
            forceBlit = true
            requestFrame()
        }
    }

    /** 尺寸要等第一次布局之后才知道：逻辑尺寸 = 视图像素 / density（与小程序同一口径） */
    private fun start() {
        if (disposed) return
        val dpr = resources.displayMetrics.density
        val w = (width / dpr).toInt().coerceAtLeast(1)
        val h = (height / dpr).toInt().coerceAtLeast(1)
        engine.post {
            choreographer = Choreographer.getInstance()
            boost = FrameBoost.forCurrentThread(context)
            val t0 = System.nanoTime()
            handle = MiniGame.nativeCreate(dir, dataDir, w, h, dpr, options)
            if (handle != 0L) audioSession = GameAudioSession(context, engine) { begin ->
                if (handle != 0L) {
                    MiniGame.nativeAudioInterruption(handle, begin)
                    drainEvents()
                    requestFrame()
                }
            }
            Log.i(TAG, "🎮 建实例 ${(System.nanoTime() - t0) / 1_000_000}ms（$w×$h@$dpr，$dir）${if (handle == 0L) " 失败" else ""}")
            drainEvents()
            requestFrame()
        }
    }

    fun onResume() = engine.post {
        running = true
        if (handle != 0L) MiniGame.nativeSetBackground(handle, false)
        audioSession?.setForeground(true)
        requestFrame()
    }

    fun onPause() = engine.post {
        running = false
        if (handle != 0L) MiniGame.nativeSetBackground(handle, true)
        audioSession?.setForeground(false)
        choreographer?.removeFrameCallback(frame)
        scheduled = false
    }

    fun dispose() {
        if (disposed) return
        disposed = true
        engine.post {
            choreographer?.removeFrameCallback(frame)
            audioSession?.release()
            audioSession = null
            if (handle != 0L) MiniGame.nativeDestroy(handle)
            handle = 0L
            boost?.close()
            thread.quitSafely()
        }
    }

    /**
     * 触摸（game-doc/02 §7.1）：一个 MotionEvent 拆成「每根指针一次」——POINTER_DOWN / UP 只报那一根，
     * MOVE 把全部历史采样按时间顺序、每根指针各报一次（游戏算手势速度要采样密度），CANCEL 报全部指针。
     * UI 线程上只打包，一次 post 到引擎线程、一次 JNI；派发在下一帧 pump 的 rAF 之前。
     */
    override fun onTouchEvent(e: MotionEvent): Boolean {
        val d = resources.displayMetrics.density
        val n = e.pointerCount
        val buf = FloatArray(6 * n * (e.historySize + 1))
        var k = 0
        val baseNanos = eventNanos(e)
        fun add(phase: Int, i: Int, x: Float, y: Float, p: Float, tNanos: Long) {
            buf[k++] = phase.toFloat(); buf[k++] = e.getPointerId(i).toFloat()
            buf[k++] = x / d; buf[k++] = y / d; buf[k++] = p.coerceIn(0f, 1f)
            buf[k++] = ((tNanos - baseNanos) / 1e6).toFloat()
        }
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                // 不要系统按 vsync 攒批的 MOVE：攒批要等到 UI 线程的下一个 vsync 才交出来，再 post 到引擎线程
                // 又错过引擎这一帧，实测事件 → 派发中位 25ms（1.5 个 vsync）。游戏自己按帧消费，来一个收一个
                if (e.actionMasked == MotionEvent.ACTION_DOWN && Build.VERSION.SDK_INT >= 30) requestUnbufferedDispatch(e)
                val i = e.actionIndex
                add(0, i, e.getX(i), e.getY(i), e.getPressure(i), baseNanos)
            }
            MotionEvent.ACTION_MOVE -> {
                for (h in 0 until e.historySize) {
                    val t = e.getHistoricalEventTime(h) * 1_000_000
                    for (i in 0 until n) add(1, i, e.getHistoricalX(i, h), e.getHistoricalY(i, h), e.getHistoricalPressure(i, h), t)
                }
                for (i in 0 until n) add(1, i, e.getX(i), e.getY(i), e.getPressure(i), baseNanos)
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                val i = e.actionIndex
                add(2, i, e.getX(i), e.getY(i), 0f, baseNanos)
            }
            MotionEvent.ACTION_CANCEL -> for (i in 0 until n) add(3, i, e.getX(i), e.getY(i), 0f, baseNanos)
            else -> return true
        }
        val count = k / 6
        engine.post {
            if (handle == 0L) return@post
            MiniGame.nativeTouchBatch(handle, buf, count, baseNanos / 1000)
            if (pendingTouchNanos == 0L) pendingTouchNanos = baseNanos
            requestFrame()
        }
        return true
    }

    /** 事件时刻（ns）：API 34 起有纳秒精度的版本 */
    private fun eventNanos(e: MotionEvent): Long =
        if (Build.VERSION.SDK_INT >= 34) e.eventTimeNanos else e.eventTime * 1_000_000

    /** 诊断：在逻辑层执行 JS（结果回到主线程） */
    fun eval(js: String, cb: (String?) -> Unit) = engine.post {
        val r = if (handle != 0L) MiniGame.nativeEval(handle, js) else null
        main.post { cb(r) }
    }

    // ───────────────────────── 引擎线程 ─────────────────────────

    private fun attach(s: Surface) {
        if (handle == 0L) {
            // 实例还在建（第一次布局之后才建）：等一下再挂
            engine.postDelayed({ attach(s) }, 16)
            return
        }
        if (surface !== s) {
            MiniGame.nativeAttachWindow(handle, s)
            surface = s
        }
        applyFrameRate(true)
        // 新 surface 要立刻有内容：下一帧强制上屏
        forceBlit = true
        requestFrame()
    }

    private fun detach() {
        if (handle != 0L) MiniGame.nativeDetachWindow(handle)
        surface = null
    }

    private var forceBlit = false

    /** rAF 帧率变了（setPreferredFramesPerSecond）就重新投票：FIXED_SOURCE 让系统选它的整数倍 */
    private fun applyFrameRate(force: Boolean) {
        val s = surface ?: return
        val fps = MiniGame.nativePreferredFps(handle)
        if (!force && fps == appliedFps) return
        appliedFps = fps
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            try {
                s.setFrameRate(fps.toFloat(), Surface.FRAME_RATE_COMPATIBILITY_FIXED_SOURCE)
            } catch (e: Exception) {
                Log.w(TAG, "setFrameRate($fps) 失败：$e")
            }
        }
    }

    private fun requestFrame() {
        if (scheduled || !running || handle == 0L || disposed) return
        scheduled = true
        choreographer?.postFrameCallback(frame)
    }

    private val wake = Runnable { requestFrame() }

    private fun tick(frameTimeNanos: Long) {
        if (handle == 0L) return
        val speculative = wantedMore
        if (speculative) requestFrame()
        val t0 = System.nanoTime()
        if (pendingTouchNanos != 0L) {
            if (touchLog) Log.i("mini-render", "👆 [GAME] 触摸 → 派发 %.1fms".format((t0 - pendingTouchNanos) / 1e6))
            pendingTouchNanos = 0L
        }
        val f = MiniGame.nativePump(handle, frameTimeNanos / 1000)
        val t1 = System.nanoTime()
        var blit = 0
        if (surface != null && (f and MiniGame.FRAME_NEW != 0 || forceBlit)) {
            blit = MiniGame.nativeBlit(handle)
            if (blit == 1) forceBlit = false else if (blit < 0) Log.w(TAG, "上屏失败（$blit）")
        }
        val t2 = System.nanoTime()
        if (f and MiniGame.FRAME_NEW != 0) boost?.report(t2 - t0)
        // 与小程序 🩹 / ⏱ 同一道线：8ms。等缓冲（lock 阻塞到上一块缓冲被释放）是 vsync 背压，不是工作量，扣掉再判
        val st = if (blit == 1) MiniGame.nativeBlitStats(handle) else 0L
        val waitMs = (st ushr 32) / 1000.0
        val workMs = (t2 - t0) / 1e6 - waitMs
        if (workMs > 8.0 || slowLog) {
            val postMs = if (blit == 1) MiniGame.nativeBlitPostUs(handle) / 1000.0 else 0.0
            Log.i("mini-render", "⏱ [GAME] 工作 %.1fms（逻辑+rAF %.1f，上屏拷贝 %.1f 其中提交 %.1f）等缓冲 %.1f".format(workMs, (t1 - t0) / 1e6, (st and 0xffffffffL) / 1000.0, postMs, waitMs))
        }
        if (f and MiniGame.FRAME_EVENTS != 0) drainEvents()
        if (audioLog && t2 - lastAudioLog > 2_000_000_000L) {
            lastAudioLog = t2
            Log.i("mini-render", "🔊 [GAME] 音频 ${MiniGame.nativeAudioStats(handle)}")
        }
        applyFrameRate(false)
        wantedMore = f and MiniGame.FRAME_WANTS_MORE != 0
        engine.removeCallbacks(wake)
        if (wantedMore) {
            requestFrame()
        } else {
            if (speculative && scheduled) {
                choreographer?.removeFrameCallback(frame)
                scheduled = false
            }
            if (f and MiniGame.FRAME_WAKE_AT != 0) {
                val us = MiniGame.nativeNextWakeUs(handle)
                if (us >= 0) engine.postDelayed(wake, (us / 1000).coerceAtLeast(1))
            }
        }
    }

    private fun drainEvents() {
        while (true) {
            val e = MiniGame.nativePollEvent(handle) ?: break
            // 日志行原样进 logcat（tag mini-game）：真机验收脚本按 T:* 前缀解析
            val o = try { org.json.JSONObject(e) } catch (_: Exception) { null }
            when (o?.optString("type")) {
                "log" -> o.optString("text").lineSequence().forEach { Log.i("mini-game", it) }
                "audioSession" -> { Log.i("mini-game", e); audioSession?.apply(o) }
                // initial 的那条建实例之前已经按 game.json 锁过了；wx.setDeviceOrientation 的才要转
                "deviceOrientation" -> if (!o.optBoolean("initial")) { val v = o.optString("value"); main.post { lockOrientation(v) } }
                "error" -> Log.w("mini-game", e)
                else -> {
                    Log.i("mini-game", e)
                    val type = o?.optString("type") ?: ""
                    val cb = onHostEvent
                    if (cb != null && o != null) main.post { cb(type, o) }
                }
            }
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

    private companion object {
        const val TAG = "mini-game"

        @Suppress("PrivateApi")
        fun SystemProperties_get(key: String): String = try {
            Class.forName("android.os.SystemProperties").getMethod("get", String::class.java).invoke(null, key) as String
        } catch (_: Exception) {
            ""
        }
    }
}
