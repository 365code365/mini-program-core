package dev.minirender

import android.os.Build
import android.view.View
import android.view.WindowInsets

/**
 * 底部留给 Android 系统导航栏（三键 / 手势条）的那一截，不交给引擎画（与 Flutter 插件 view.dart `_reservedBottom` 同一规则）。
 *
 * 微信安卓的小程序窗口不伸到虚拟导航键下面（`windowHeight` 不含它）。SDK 替宿主铺满整屏（[HostChrome.edgeToEdge]）之后，
 * 视图一直铺到屏幕底边：不留的话页面最底下一截、吸底的输入条 / 操作栏都压在导航键下面 —— 模拟器三键导航实测轻聊聊天页的
 * 输入条被盖住 126px 里的 86px，只露出一条边；而样例页大多没写 `env(safe-area-inset-bottom)`。
 *
 * 留出来的那一截画所在页的底色（tabBar 页接 tabBar 的底色，见 [color]），触摸不交给引擎（手势导航时这一条收得到触摸）。
 */
internal class NavBarReserve(private val view: View) {
    /** 当前留出的高度（设备像素） */
    var px = 0
        private set

    /** 按新的窗口 insets 重算；变了返回 true（调用方要把引擎的高度改成 `view.height - px`） */
    @Suppress("DEPRECATION")
    fun update(insets: WindowInsets): Boolean {
        // stable：不含软键盘（键盘弹着时 systemWindowInsetBottom 会把键盘也算进去）
        val nav = if (Build.VERSION.SDK_INT >= 30) {
            insets.getInsets(WindowInsets.Type.navigationBars()).bottom
        } else {
            insets.stableInsetBottom
        }
        val next = overlap(nav)
        if (next == px) return false
        px = next
        return true
    }

    /** 视图与导航栏真正重叠的那一截（视图放在别的东西上面、没伸到屏幕底时是 0） */
    private fun overlap(nav: Int): Int {
        if (nav <= 0 || view.height <= 0) return 0
        val loc = IntArray(2).also { view.getLocationInWindow(it) }
        val windowH = view.rootView.height
        return ((loc[1] + view.height) - (windowH - nav)).coerceIn(0, view.height / 2)
    }

    /** 交给引擎的高度（设备像素） */
    fun engineHeight(viewHeight: Int): Int = (viewHeight - px).coerceAtLeast(1)

    /** 按在留出的那一截上：不属于小程序 */
    fun swallows(y: Float, viewHeight: Int): Boolean = px > 0 && y >= viewHeight - px

    companion object {
        /** `page` 事件里那一截该画的底色：tabBar 页用 tabBar 的底色，否则用页面的 `backgroundColor`，都没有就白 */
        fun color(ev: org.json.JSONObject): Int {
            val raw = ev.optString("tabBarBackgroundColor").ifEmpty { ev.optJSONObject("window")?.optString("backgroundColor").orEmpty() }
            return runCatching { android.graphics.Color.parseColor(raw.trim()) }.getOrDefault(android.graphics.Color.WHITE)
        }
    }
}
