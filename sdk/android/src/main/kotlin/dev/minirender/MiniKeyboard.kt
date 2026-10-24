package dev.minirender

import android.graphics.Rect
import android.os.Build
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.view.ViewTreeObserver
import android.view.WindowInsets
import org.json.JSONObject

/**
 * 键盘弹起时把画面顶上去（微信输入框的 `adjust-position`，默认开），与 Flutter 插件 view.dart 的 `_keyboardShift` 同一规则：
 * 键盘挡住了聚焦的输入框，就把整个画面往上移「刚好露出输入框底边 + `cursor-spacing`」，最多移键盘挡住的那么高。
 * 画面**不改尺寸**（不重排页面，与微信一致），宿主 Activity 用 `adjustNothing` 就行。
 *
 * 移的是视图里所有子视图（引擎的 SurfaceView 等）的 translationY，触摸坐标按同样的量加回去（[adjustTouch]），
 * 引擎看到的始终是没移之前的画面坐标。键盘高度另外推给小程序的 `wx.onKeyboardHeightChange`。
 *
 * 键盘高度：Android 11+ 读 `WindowInsets.Type.ime()`；更老的系统看窗口可见区域（`getWindowVisibleDisplayFrame`）
 */
internal class MiniKeyboard(
    private val view: ViewGroup,
    private val onKeyboardHeight: (Float) -> Unit,
) {
    private var focus: JSONObject? = null
    private var imePx = 0
    private var reportedPx = -1
    /** 当前上移量（设备像素，≥0） */
    var shiftPx = 0f
        private set

    private val layoutListener = ViewTreeObserver.OnGlobalLayoutListener {
        if (Build.VERSION.SDK_INT < 30) {
            val r = Rect()
            view.getWindowVisibleDisplayFrame(r)
            val root = view.rootView.height
            // 可见区下沿到窗口底：键盘 + 导航条；小于屏高的 15% 当没有键盘（只剩导航条）
            val gone = (root - r.bottom).coerceAtLeast(0)
            update(if (gone > root * 0.15f) gone else 0)
        }
    }

    fun attach() = view.viewTreeObserver.addOnGlobalLayoutListener(layoutListener)

    fun detach() = view.viewTreeObserver.removeOnGlobalLayoutListener(layoutListener)

    fun onInsets(insets: WindowInsets) {
        if (Build.VERSION.SDK_INT >= 30) update(insets.getInsets(WindowInsets.Type.ime()).bottom)
    }

    fun onFocus(ev: JSONObject) {
        focus = ev
        apply()
    }

    fun onBlur() {
        focus = null
        apply()
    }

    private fun update(px: Int) {
        if (px == imePx) return
        imePx = px
        if (px != reportedPx) {
            reportedPx = px
            onKeyboardHeight(px / view.resources.displayMetrics.density)
        }
        apply()
    }

    private fun apply() {
        val target = target()
        if (target == shiftPx) return
        shiftPx = target
        for (i in 0 until view.childCount) {
            view.getChildAt(i).animate().translationY(-target).setDuration(if (target > 0) 200L else 160L).start()
        }
    }

    /** 该上移多少（设备像素）：键盘挡住输入框底边（+ cursor-spacing）的那一截，最多键盘挡住视图的高度 */
    private fun target(): Float {
        val f = focus ?: return 0f
        if (imePx <= 0 || !f.optBoolean("adjustPosition", true)) return 0f
        val dpr = view.resources.displayMetrics.density
        val loc = IntArray(2).also { view.getLocationInWindow(it) }
        val windowH = view.rootView.height
        // 视图下沿离窗口底边多远（下面还有别的东西时，键盘先盖住它们）
        val below = windowH - (loc[1] + view.height)
        val overlap = (imePx - below).coerceAtLeast(0)
        if (overlap <= 0) return 0f
        val r = f.optJSONObject("rect") ?: return 0f
        val bottom = (r.optDouble("y") + r.optDouble("height") + f.optDouble("cursorSpacing", 0.0)).toFloat() * dpr
        // 输入框与键盘之间至少留一点（微信实测约 10px）
        val need = bottom + 10f * dpr - (view.height - overlap)
        return need.coerceIn(0f, overlap.toFloat())
    }

    /** 触摸坐标加回上移量：引擎看的是没移之前的画面 */
    fun adjustTouch(e: MotionEvent) {
        if (shiftPx != 0f) e.offsetLocation(0f, shiftPx)
    }
}
