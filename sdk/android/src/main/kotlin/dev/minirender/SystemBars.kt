package dev.minirender

import android.app.Activity
import android.os.Build
import android.view.View
import android.view.WindowInsets
import android.view.WindowInsetsController
import java.util.WeakHashMap

/**
 * 视频全屏时收起 / 放出状态栏与导航条（引擎的 `videoFullscreen` 事件，[MiniProgramView] 默认接）。
 *
 * 横屏全屏是**引擎把那一层转过来画**（不转 Activity，见引擎 `video/fullscreen.rs`），这里只管系统栏：
 * 不收起的话竖着的状态栏会压在横过来的画面一侧。收起后从屏幕边缘划一下能临时叫出来（与系统播放器一致）。
 * 收起 / 放出时系统会报新的安全区，引擎在全屏期间按住不用（不重排页面），见 `engine_video_feed.rs`。
 */
internal object SystemBars {
    /** API 30 以下：进全屏前的 systemUiVisibility，退出时原样还回去 */
    private val saved = WeakHashMap<Activity, Int>()

    @Suppress("DEPRECATION")
    private const val IMMERSIVE =
        View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY or View.SYSTEM_UI_FLAG_FULLSCREEN or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION

    @Suppress("DEPRECATION")
    fun immersive(activity: Activity?, on: Boolean) {
        val window = activity?.window ?: return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val c = window.insetsController ?: return
            if (on) {
                c.systemBarsBehavior = WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
                c.hide(WindowInsets.Type.systemBars())
            } else {
                c.show(WindowInsets.Type.systemBars())
            }
            return
        }
        val decor = window.decorView
        if (on) {
            if (!saved.containsKey(activity)) saved[activity] = decor.systemUiVisibility
            decor.systemUiVisibility = decor.systemUiVisibility or IMMERSIVE or
                View.SYSTEM_UI_FLAG_LAYOUT_STABLE or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
        } else {
            decor.systemUiVisibility = saved.remove(activity) ?: (decor.systemUiVisibility and IMMERSIVE.inv())
        }
    }
}
