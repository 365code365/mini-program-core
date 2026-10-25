package dev.minirender

import android.os.Build
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.widget.FrameLayout

/**
 * [MiniProgramView] 里那块 SurfaceView 的建 / 换 / 定尺寸 / 第一帧前藏起来（只在主线程读写）。
 *
 * 从 MiniProgramView.kt 原样搬出（那边超了 500 行）：逻辑一行没改，只把 `this` 换成 [parent]。
 */
internal class MiniSurfaceHost(private val parent: FrameLayout, private val session: EngineSession) {
    private var surfaceView: SurfaceView? = null
    private var fixed: Pair<Int, Int>? = null
    private var surfaceRevealed = false

    /**
     * Callback2：窗口这一帧要等这块 SurfaceView 有了内容再一起上屏（[EngineSession.onRedrawNeeded]）。
     * 只实现 Callback 的话，回桌面再回来时窗口不等它、直接上屏，SurfaceView 还没有缓冲 —— 露出底下那层，闪一下
     */
    private val callback = object : SurfaceHolder.Callback2 {
        override fun surfaceCreated(holder: SurfaceHolder) {}

        override fun surfaceRedrawNeeded(holder: SurfaceHolder) {}

        override fun surfaceRedrawNeededAsync(holder: SurfaceHolder, drawingFinished: Runnable) =
            session.onRedrawNeeded(drawingFinished)

        override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
            val s = holder.surface
            requestHighRefresh(s)
            session.onSurface(s)
        }

        // 返回之后不能再碰这块 Surface：同步等引擎线程放手
        override fun surfaceDestroyed(holder: SurfaceHolder) = session.onSurfaceGone()
    }

    /** 引擎的像素尺寸到了 / 变了。第一次到的时候才建 SurfaceView：缓冲区一建出来就是对的尺寸 */
    fun setPixelSize(w: Int, h: Int) {
        if (w <= 0 || h <= 0) return
        val first = fixed == null
        if (fixed == w to h) return
        fixed = w to h
        if (first || surfaceView == null) return newSurfaceView()
        surfaceView?.let { sv ->
            sv.holder.setFixedSize(w, h)
            sv.layoutParams = FrameLayout.LayoutParams(w, h)
        }
    }

    /**
     * 换一块从没被连过的 SurfaceView（第一次建、GPU 回退 CPU、直写失败改走 Bitmap：
     * 那块 Surface 已经被另一种生产者连过）。主线程调
     */
    fun newSurfaceView() {
        surfaceView?.holder?.removeCallback(callback)
        parent.removeAllViews()
        val sv = SurfaceView(parent.context)
        // 第一帧之前先藏起来（Android 10+ 的 SurfaceView 才支持 alpha）：没有缓冲的 SurfaceView 露的是黑底，
        // 第一次打开时引擎要先开 GPU（一加 PLY110 约 0.5s）。藏着的时候露出这个视图的背景色
        surfaceRevealed = false
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) sv.alpha = 0f
        sv.holder.addCallback(callback)
        // 一个引擎像素对一个屏幕像素：布局尺寸 = 缓冲区尺寸 = 引擎像素尺寸。视图宽 1080、引擎 1079 时
        // MATCH_PARENT 会让 SurfaceFlinger 把整帧横向拉伸 1 像素（全屏发虚），所以按像素尺寸摆在左上角
        val (w, h) = fixed ?: (FrameLayout.LayoutParams.MATCH_PARENT to FrameLayout.LayoutParams.MATCH_PARENT)
        fixed?.let { (fw, fh) -> sv.holder.setFixedSize(fw, fh) }
        parent.addView(sv, FrameLayout.LayoutParams(w, h))
        surfaceView = sv
    }

    fun revealSurface() {
        if (surfaceRevealed) return
        surfaceRevealed = true
        surfaceView?.alpha = 1f
    }

    /**
     * 这一层在动的时候要高刷：没人要的话系统（ColorOS 实测）会把整个屏降到 60Hz。
     * 只是「希望」，静止时系统照样可以降（静止页面本来也不出帧）
     */
    private fun requestHighRefresh(s: Surface) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) return
        val max = parent.display?.supportedModes?.maxOfOrNull { it.refreshRate } ?: return
        try {
            s.setFrameRate(max, Surface.FRAME_RATE_COMPATIBILITY_DEFAULT)
        } catch (_: Exception) {
        }
    }
}
