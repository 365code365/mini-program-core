package dev.minirender.flutter

import android.content.Context
import android.os.Build
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.View
import android.widget.FrameLayout
import io.flutter.plugin.common.StandardMessageCodec
import io.flutter.plugin.platform.PlatformView
import io.flutter.plugin.platform.PlatformViewFactory

/**
 * 引擎画面的「独立图层」上屏方式（Android 默认）：一块自己的 SurfaceView，嵌成 Flutter 的平台视图
 * （hybrid composition），由 SurfaceFlinger 直接合成，**不经过 Flutter 的帧**。
 *
 * ## 为什么不用外部纹理
 * 纹理那条路上，屏上那一帧是「Flutter 光栅线程取纹理那一刻队列里的那张」：引擎每个 vsync 都按时
 * 交出了一帧，可 Flutter 自己的帧跑在主线程上，主线程一忙（一加 PLY110 实测 vsync 后 10~13ms 才开始光栅），
 * Flutter 这个 vsync 就没出帧，屏上重复上一张 —— 慢拖时 3% 的 vsync 是这样丢的（SurfaceFlinger
 * `--latency` 量的，不靠 App 内日志），肉眼就是「一顿一顿」。引擎侧的帧耗时、位移再均匀也救不回来。
 *
 * 独立图层上，引擎线程每个 vsync 交给 SurfaceFlinger 的那一张就是屏上的那一张：Flutter 卡不卡与它无关。
 * 盖在小程序上面的 Flutter 控件（宿主承载的视频、输入框）照样在上面（hybrid composition 的覆盖层）。
 *
 * 一个进程同时只有一个小程序，所以视图直接找插件当前的 [EngineHost]（[hostOf]）。
 */
internal class EngineViewFactory(private val hostOf: () -> EngineHost?) :
    PlatformViewFactory(StandardMessageCodec.INSTANCE) {
    override fun create(context: Context, viewId: Int, args: Any?): PlatformView = EngineView(context, hostOf)
}

internal class EngineView(context: Context, private val hostOf: () -> EngineHost?) : PlatformView {
    private val frame = FrameLayout(context)
    private var surfaceView: SurfaceView? = null
    private var fixed: Pair<Int, Int>? = null

    private val callback = object : SurfaceHolder.Callback {
        override fun surfaceCreated(holder: SurfaceHolder) {}

        override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
            val s = holder.surface
            requestHighRefresh(s)
            hostOf()?.onViewSurface(s)
        }

        override fun surfaceDestroyed(holder: SurfaceHolder) {
            // 返回之后不能再碰这块 Surface：EngineHost 同步等引擎线程放手
            hostOf()?.onViewSurfaceGone()
        }
    }

    init {
        newSurface()
        hostOf()?.bindView(this)
    }

    /**
     * 换一块从没被连过的 Surface（GPU 运行中回退 CPU、直写失败改走 Bitmap 时：那块 Surface 已经被
     * 另一种生产者连过）。主线程调
     */
    fun newSurface() {
        surfaceView?.holder?.removeCallback(callback)
        frame.removeAllViews()
        val sv = SurfaceView(frame.context)
        sv.holder.addCallback(callback)
        fixed?.let { (w, h) -> sv.holder.setFixedSize(w, h) }
        frame.addView(sv, FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT))
        surfaceView = sv
    }

    /** 缓冲区按引擎的像素尺寸（一个引擎像素对一个屏幕像素，不缩放）。主线程调 */
    fun setPixelSize(w: Int, h: Int) {
        if (w <= 0 || h <= 0 || fixed == w to h) return
        fixed = w to h
        surfaceView?.holder?.setFixedSize(w, h)
    }

    /**
     * 这一层在动的时候要高刷：没人要的话系统（ColorOS 实测）会把整个屏降到 60 / 90Hz。
     * 只是「希望」，静止时系统照样可以降（静止页面本来也不出帧）
     */
    private fun requestHighRefresh(s: Surface) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) return
        val display = frame.display ?: return
        val max = display.supportedModes.maxOfOrNull { it.refreshRate } ?: return
        try {
            s.setFrameRate(max, Surface.FRAME_RATE_COMPATIBILITY_DEFAULT)
        } catch (_: Exception) {
        }
    }

    override fun getView(): View = frame

    override fun dispose() {
        hostOf()?.unbindView(this)
        surfaceView?.holder?.removeCallback(callback)
        frame.removeAllViews()
        surfaceView = null
    }
}
