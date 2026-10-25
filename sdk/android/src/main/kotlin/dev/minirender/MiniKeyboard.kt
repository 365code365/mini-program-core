package dev.minirender

import android.graphics.Rect
import android.os.Build
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
 * 顶起交给引擎（[onShift]，逻辑像素 → `MiniEngine.nativeSetKeyboardShift`）：引擎合成时把导航栏以下整体上移、
 * 自己按帧缓动、触摸坐标自己换回去（src/host/engine_keyboard.rs）。从前这里给子视图（SurfaceView）做 200ms 的
 * `translationY` 动画：引擎画的导航栏跟着被推出屏幕、页面顶到状态栏底下；SurfaceView 的位置更新与引擎出帧不同步，
 * 顶起那一下一顿一顿（招财银行转账页「附言」）。键盘高度另外推给小程序的 `wx.onKeyboardHeightChange`。
 *
 * 键盘高度：Android 11+ 读 `WindowInsets.Type.ime()`；更老的系统看窗口可见区域（`getWindowVisibleDisplayFrame`）
 */
internal class MiniKeyboard(
    private val view: ViewGroup,
    private val onKeyboardHeight: (Float) -> Unit,
    /** 目标上移量（逻辑像素，0 = 收回），交给引擎 */
    private val onShift: (Float) -> Unit,
    /**
     * 输入框还聚焦着、键盘却被用户收掉了（返回键、输入法自己的收起键、下滑）：与微信一致，输入框失焦
     * （宿主送 `escape`，引擎派发 `bindblur`、收光标）。不失焦的话引擎以为键盘还在：光标一直闪，
     * 再点输入框只是「挪光标」，键盘就弹不出来了（轻聊聊天页）
     */
    private val onUserHide: () -> Unit,
    /** 视图底下留给系统导航栏、不归引擎画的那一截（设备像素，NavBarReserve.kt）：键盘先盖住它 */
    private val reserved: () -> Int,
) {
    private var focus: JSONObject? = null
    /** 最近一次聚焦的时刻：换输入框时有的输入法会先收再弹，这一下不算用户收起 */
    private var focusAt = 0L
    private var imePx = 0
    private var reportedPx = -1
    /** 当前目标上移量（设备像素，≥0；画面怎么动由引擎缓动） */
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
        focusAt = android.os.SystemClock.uptimeMillis()
        apply()
    }

    fun onBlur() {
        focus = null
        apply()
    }

    private fun update(px: Int) {
        if (px == imePx) return
        val wasShown = imePx > 0
        imePx = px
        // 报给小程序的是键盘挡住视图的那一截（视图下面另有东西时先扣掉）：页面拿 windowHeight - height
        // 就是键盘上方看得见的高度（聊天页据此把列表改矮，`bindkeyboardheightchange` 也是这个数）
        val covered = overlap()
        if (covered != reportedPx) {
            reportedPx = covered
            onKeyboardHeight(covered / view.resources.displayMetrics.density)
        }
        apply()
        if (wasShown && px == 0 && focus != null && android.os.SystemClock.uptimeMillis() - focusAt > 500) onUserHide()
    }

    /** 键盘挡住视图的高度（设备像素） */
    private fun overlap(): Int {
        if (imePx <= 0) return 0
        val loc = IntArray(2).also { view.getLocationInWindow(it) }
        val below = view.rootView.height - (loc[1] + view.height)
        return (imePx - below - reserved()).coerceAtLeast(0)
    }

    private fun apply() {
        val target = target()
        if (target == shiftPx) return
        shiftPx = target
        onShift(target / view.resources.displayMetrics.density)
    }

    /** 该上移多少（设备像素）：键盘挡住输入框底边（+ cursor-spacing）的那一截，最多键盘挡住视图的高度 */
    private fun target(): Float {
        val f = focus ?: return 0f
        if (imePx <= 0 || !f.optBoolean("adjustPosition", true)) return 0f
        val dpr = view.resources.displayMetrics.density
        // 键盘挡住引擎画面的那一截（视图下面另有东西、底下留了导航栏时，键盘先盖住它们）
        val overlap = overlap()
        if (overlap <= 0) return 0f
        val r = f.optJSONObject("rect") ?: return 0f
        val bottom = (r.optDouble("y") + r.optDouble("height") + f.optDouble("cursorSpacing", 0.0)).toFloat() * dpr
        // 输入框与键盘之间至少留一点（微信实测约 10px）。引擎画面高 = 视图高 - 导航栏那一截
        val need = bottom + 10f * dpr - (view.height - reserved() - overlap)
        return need.coerceIn(0f, overlap.toFloat())
    }
}
