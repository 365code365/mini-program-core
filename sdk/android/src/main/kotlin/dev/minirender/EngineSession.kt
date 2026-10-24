package dev.minirender

import android.content.Context
import android.util.Log
import android.view.Surface
import java.io.File

/**
 * [MiniProgramView] 的引擎侧：原生视图那一层适配。出帧、上屏、事件都在 [EngineCore]（与 Flutter 插件同一份），
 * 这里只把它接到 SurfaceView 与视图的回调上（[Ui]），并补上原生 SDK 自己的启动步骤（诊断开关、报错）。
 *
 * 上屏规则（独立图层 + 引擎线程交帧、三条路逐级回退）见 EngineCore.kt 与 doc/滑动顿挫避坑清单.md。
 */
internal class EngineSession(context: Context, private val ui: Ui) {
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

    private val core = EngineCore(context, object : EngineCore.Hooks {
        override fun reveal() = ui.reveal()
        override fun newSurface() = ui.newSurface()
        override fun onEvents(list: List<String>) = ui.onEvents(list)
    })

    val pixelW get() = core.pixelW
    val pixelH get() = core.pixelH
    val activeBackend get() = core.activeBackend
    val backendReason get() = core.backendReason
    /** 引擎建好、首页已打开 */
    val ready get() = core.ready

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
    ) = core.start(
        appDir, dataDir, route, width, height, dpr, options, backend,
        EngineCore.DeviceInfo(device.totalMem, device.lowRam, device.api),
        // 诊断开关要在引擎读环境之前就位
        beforeWarmup = { MiniDiag.apply() },
    ) { error ->
        when (error) {
            null -> {
                val (pw, ph) = core.pixelW to core.pixelH
                core.main.post { if (!core.disposed) ui.onPixelSize(pw, ph) }
                // 首页的数据回调 / 定时器先跑起来；Surface 到了再上屏（attach 会补画当前帧）
                core.requestFrame()
            }
            "已销毁" -> {}
            else -> Log.e(TAG, "小程序打不开：$appDir（$error，看上面 mini-render 的报错）")
        }
    }

    // ───────────────────────── 命令（主线程进来） ─────────────────────────

    /** 在引擎线程上执行一条命令，之后挂一帧（输入之后引擎要出帧回应） */
    fun run(block: (Long) -> Unit) = core.run(block)

    /** 同步等引擎线程的结果（返回键要立刻决定 finish 还是不动）。引擎没建好返回 [fallback] */
    fun <T> call(fallback: T, block: (Long) -> T): T = core.call(fallback, block)

    /** what: 0 进前台 / 1 进后台 / 2 内存告警 */
    fun lifecycle(what: Int) = core.lifecycle(what)

    /** 视图改了尺寸（逻辑像素）：引擎重排，SurfaceView 跟着改缓冲区 */
    fun resize(width: Int, height: Int, dpr: Float) = core.onEngine {
        val h = core.handle
        if (h == 0L || !MiniEngine.nativeResize(h, width, height, dpr)) return@onEngine
        core.updatePixelSize()
        val (pw, ph) = core.pixelW to core.pixelH
        core.main.post { if (!core.disposed) ui.onPixelSize(pw, ph) }
        core.markNeedsPresent()
        core.requestFrame()
    }

    /** 把引擎当前这一帧存成 PNG（SurfaceView 的内容 `View.draw` 取不到）。[done] 在主线程回调 */
    fun dumpFrame(file: File, done: (Boolean) -> Unit) = core.dumpFrame(file, done)

    // ───────────────────────── Surface（主线程进来） ─────────────────────────

    /** SurfaceView 的 Surface 可用 / 改了尺寸 */
    fun onSurface(s: Surface) = core.attachSurface(s)

    /** SurfaceView 的 Surface 要没了：返回之后不能再碰它，同步等引擎线程放手 */
    fun onSurfaceGone() = core.surfaceGone()

    /** 窗口等这块 SurfaceView 的首帧（`Callback2.surfaceRedrawNeededAsync`），见 [EngineCore.redrawNeeded] */
    fun onRedrawNeeded(done: Runnable) = core.redrawNeeded(done)

    /** 主线程调：先让引擎线程放手 Surface 并销毁引擎，再停线程 */
    fun dispose() = core.dispose()

    private companion object {
        const val TAG = "mini-render"
    }
}
