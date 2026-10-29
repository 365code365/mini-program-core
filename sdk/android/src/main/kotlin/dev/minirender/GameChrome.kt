package dev.minirender

import android.app.Activity
import android.content.res.Configuration
import android.os.Build
import android.view.WindowInsets
import android.view.WindowManager
import android.widget.FrameLayout

/**
 * [MiniGameView] 的屏幕边缘：安全区（刘海、挖孔、横条、横屏时的侧边）与右上角胶囊（[GameCapsule]）。
 * iOS 对应 MiniGameView.swift 里的 safeAreaInsetsDidChange / GameCapsule.swift。
 *
 * - **铺满刘海**：游戏页沉浸式全屏，窗口要进刘海区（`LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES`），否则横屏时系统在刘海那一侧
 *   留一条黑边、游戏画面变窄。进来时设、[release] 时还原（Flutter 的页面与原生 Activity 共用一个窗口）；
 * - **安全区**：状态栏 / 导航栏按「不管显没显示都占着」算（沉浸式里它们是收着的，但从边缘一划就出来，盖住的地方游戏不该放按钮，
 *   微信的 statusBarHeight 也是这么报的），再并上刘海。横屏时顶上只算刘海：手机横过来状态栏收起，iPhone 横屏也没有状态栏；
 * - 报给引擎的是逻辑像素（`/ density`），引擎按方向算 safeArea 与胶囊位置（src/game/chrome.rs），胶囊按引擎给的矩形摆。
 */
internal class GameChrome(private val host: FrameLayout, capsuleOn: Boolean, onCapsule: (action: String) -> Unit) {
    private val density = host.resources.displayMetrics.density
    private val activity = host.context as? Activity
    private var savedCutout: Int? = null
    /** 最近一次的内缩（逻辑像素 top / right / bottom / left）；还没拿到是 null */
    var insets: FloatArray? = null
        private set
    val capsule: GameCapsule? = if (capsuleOn) GameCapsule(host.context, onCapsule) else null

    init {
        capsule?.let {
            it.visibility = android.view.View.GONE
            host.addView(it, FrameLayout.LayoutParams(0, 0))
        }
        if (Build.VERSION.SDK_INT >= 28) activity?.window?.let { w ->
            val a = w.attributes
            savedCutout = a.layoutInDisplayCutoutMode
            a.layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
            w.attributes = a
        }
    }

    /** 窗口内缩来了（主线程）：返回逻辑像素的 top / right / bottom / left；与上一次一样返回 null */
    fun onInsets(i: WindowInsets): FloatArray? {
        val landscape = host.resources.configuration.orientation == Configuration.ORIENTATION_LANDSCAPE
        val (t, r, b, l) = if (Build.VERSION.SDK_INT >= 30) {
            val bars = i.getInsetsIgnoringVisibility(WindowInsets.Type.systemBars())
            val cut = i.getInsets(WindowInsets.Type.displayCutout())
            listOf(if (landscape) cut.top else maxOf(bars.top, cut.top), maxOf(bars.right, cut.right), maxOf(bars.bottom, cut.bottom), maxOf(bars.left, cut.left))
        } else {
            @Suppress("DEPRECATION")
            val cut = if (Build.VERSION.SDK_INT >= 28) i.displayCutout else null
            @Suppress("DEPRECATION")
            listOf(
                if (landscape) cut?.safeInsetTop ?: 0 else maxOf(i.stableInsetTop, cut?.safeInsetTop ?: 0),
                maxOf(i.stableInsetRight, cut?.safeInsetRight ?: 0),
                maxOf(i.stableInsetBottom, cut?.safeInsetBottom ?: 0),
                maxOf(i.stableInsetLeft, cut?.safeInsetLeft ?: 0),
            )
        }
        val v = floatArrayOf(t / density, r / density, b / density, l / density)
        if (insets?.contentEquals(v) == true) return null
        insets = v
        return v
    }

    /** 创建选项里补上 `safeArea`（宿主自己给了就不动） */
    fun withSafeArea(o: org.json.JSONObject) {
        val v = insets ?: return
        if (o.has("safeArea")) return
        o.put("safeArea", org.json.JSONObject().put("top", v[0].toDouble()).put("right", v[1].toDouble())
            .put("bottom", v[2].toDouble()).put("left", v[3].toDouble()))
    }

    /** 引擎给的胶囊矩形（逻辑像素 x / y / w / h），主线程 */
    fun placeCapsule(r: FloatArray?) {
        val c = capsule ?: return
        if (r == null || r.size < 4) return
        c.place(r[0], r[1], r[2], r[3])
        c.bringToFront()
    }

    fun release() {
        val m = savedCutout ?: return
        if (Build.VERSION.SDK_INT >= 28) activity?.window?.let { w ->
            val a = w.attributes
            a.layoutInDisplayCutoutMode = m
            w.attributes = a
        }
    }
}
