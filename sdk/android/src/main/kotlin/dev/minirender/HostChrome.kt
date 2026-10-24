package dev.minirender

import android.app.Activity
import android.app.ActivityManager
import android.content.Context
import android.view.WindowInsets
import android.app.Application
import android.content.ComponentCallbacks2
import android.content.res.Configuration
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.view.HapticFeedbackConstants
import android.view.View
import android.view.WindowManager
import org.json.JSONObject

/**
 * 每个宿主都要写、写法又都一样的那几件事，SDK 自己做（从前在示例的 MainActivity 里，接入方得照抄）：
 *
 * - **铺满整屏**（[edgeToEdge]）：引擎按窗口 insets 算 `statusBarHeight`、胶囊位置与 `env(safe-area-inset-*)`，
 *   画面必须铺到状态栏 / 导航条下面才对得上。targetSdk 35 在 Android 15 上本来就是 edge-to-edge，低版本要补；
 * - **触感**（[vibrate]）：`wx.vibrateShort` / `vibrateLong` 的 `vibrate` 事件。走 `performHapticFeedback`，不要 VIBRATE 权限；
 * - **生命周期**（[HostLifecycle]）：所在 Activity 的 resume / pause / destroy 与系统内存告警自动转给引擎
 *   （`App.onShow` / `onHide`、停帧、销毁）。宿主照旧手动调 `onHostResume` 之类也没关系：状态没变的那次不转。
 *
 * 返回键不在这里：minSdk 24 上一个 View 截不住 Activity 的返回（没有 AndroidX 的 OnBackPressedDispatcher，
 * Android 13+ 的 OnBackInvokedCallback 还要宿主在清单里开 `enableOnBackInvokedCallback`），仍是宿主一行
 * `if (!mini.goBack()) super.onBackPressed()`。
 */
internal object HostChrome {
    /** 透明系统栏、内容铺到状态栏 / 导航条 / 刘海下面（浅色状态栏图标：引擎画的默认导航栏是白底黑字） */
    @Suppress("DEPRECATION")
    fun edgeToEdge(activity: Activity) {
        val w = activity.window
        w.addFlags(WindowManager.LayoutParams.FLAG_DRAWS_SYSTEM_BAR_BACKGROUNDS)
        w.statusBarColor = Color.TRANSPARENT
        w.navigationBarColor = Color.TRANSPARENT
        w.decorView.systemUiVisibility = w.decorView.systemUiVisibility or
            View.SYSTEM_UI_FLAG_LAYOUT_STABLE or
            View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN or
            View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION or
            View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR
        if (Build.VERSION.SDK_INT >= 28) {
            w.attributes = w.attributes.apply {
                layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
            }
        }
    }

    /** 刘海 / 状态栏 / 导航条（逻辑像素）：`statusBarHeight`、胶囊位置与 `env(safe-area-inset-*)` 都按它算 */
    @Suppress("DEPRECATION")
    fun insetsJson(i: WindowInsets, dpr: Float): JSONObject {
        val (t, r, b, l) = if (Build.VERSION.SDK_INT >= 30) {
            val s = i.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout())
            listOf(s.top, s.right, s.bottom, s.left)
        } else {
            listOf(i.systemWindowInsetTop, i.systemWindowInsetRight, i.systemWindowInsetBottom, i.systemWindowInsetLeft)
        }
        return JSONObject().put("top", t / dpr).put("right", r / dpr).put("bottom", b / dpr).put("left", l / dpr)
    }

    /** auto 后端的机型分级：内存、低内存机型标记、API level（与 Flutter 插件同一组） */
    fun deviceInfo(context: Context): EngineSession.DeviceInfo {
        val am = context.getSystemService(Context.ACTIVITY_SERVICE) as? ActivityManager
        val mem = ActivityManager.MemoryInfo().also { am?.getMemoryInfo(it) }
        return EngineSession.DeviceInfo(mem.totalMem, am?.isLowRamDevice ?: false, Build.VERSION.SDK_INT)
    }

    /** `vibrate` 事件：长振动用 LONG_PRESS 那一档（没有 VIBRATE 权限时 Vibrator 振不了），短振动按 `style` 分轻重 */
    fun vibrate(view: View, ev: JSONObject) {
        val kind = when {
            ev.optString("duration") == "long" -> HapticFeedbackConstants.LONG_PRESS
            ev.optString("style") == "light" -> HapticFeedbackConstants.CLOCK_TICK
            else -> HapticFeedbackConstants.VIRTUAL_KEY
        }
        view.performHapticFeedback(kind)
    }
}

/**
 * 跟着 [activity] 的生命周期转给引擎。[resume] / [pause] 是幂等的（前台 / 后台状态没变就不转），
 * 宿主自己也调的话不会多派发一次 `App.onShow`
 */
internal class HostLifecycle(
    private val activity: Activity,
    private val resume: () -> Unit,
    private val pause: () -> Unit,
    private val trimMemory: () -> Unit,
    private val destroy: () -> Unit,
) : Application.ActivityLifecycleCallbacks, ComponentCallbacks2 {
    private var registered = false

    fun register() {
        if (registered) return
        registered = true
        activity.application.registerActivityLifecycleCallbacks(this)
        activity.application.registerComponentCallbacks(this)
    }

    fun unregister() {
        if (!registered) return
        registered = false
        activity.application.unregisterActivityLifecycleCallbacks(this)
        activity.application.unregisterComponentCallbacks(this)
    }

    override fun onActivityResumed(a: Activity) { if (a === activity) resume() }
    override fun onActivityPaused(a: Activity) { if (a === activity) pause() }
    override fun onActivityDestroyed(a: Activity) {
        if (a !== activity) return
        unregister()
        destroy()
    }
    override fun onActivityCreated(a: Activity, b: Bundle?) {}
    override fun onActivityStarted(a: Activity) {}
    override fun onActivityStopped(a: Activity) {}
    override fun onActivitySaveInstanceState(a: Activity, b: Bundle) {}

    /** 每一档都转（与从前宿主在 Activity.onTrimMemory 里无条件转的一致）：要不要丢缓存由引擎按自己的状态定 */
    override fun onTrimMemory(level: Int) = trimMemory()
    override fun onConfigurationChanged(c: Configuration) {}
    @Deprecated("ComponentCallbacks.onLowMemory") override fun onLowMemory() = trimMemory()
}
