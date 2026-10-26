package dev.minirender

import android.content.Context
import android.provider.Settings
import android.view.OrientationEventListener

/**
 * 全屏视频跟着手机转：只在全屏期间开方向传感器，把朝向换成引擎的 0 / 90 / -90 报上去
 * （`nativeDeviceOrientation`）。引擎转的是全屏那一层（不转 Activity、页面不重排），见 video/fullscreen.rs。
 *
 * - 带回差：离开竖屏要转过 60°、回竖屏要回到 30° 以内，拿在 45° 附近不会来回跳；倒拿（180°）不理；
 * - 系统关了「自动旋转」就不跟（与系统里别的 App 一致），停在进全屏时的方向（竖屏）；
 * - 关掉时报一次 0：下一次进全屏从竖屏开始（传感器开起来那一下再按实际朝向转）。
 */
internal class FullscreenOrientation(private val context: Context, private val report: (Int) -> Unit) {
    private var current = 0
    private var listener: OrientationEventListener? = null

    fun enable(on: Boolean) {
        if (on == (listener != null)) return
        if (!on) {
            listener?.disable()
            listener = null
            if (current != 0) report(0)
            current = 0
            return
        }
        val l = object : OrientationEventListener(context) {
            override fun onOrientationChanged(angle: Int) {
                if (angle == ORIENTATION_UNKNOWN || !autoRotate()) return
                val next = map(angle, current)
                if (next != current) {
                    current = next
                    report(next)
                }
            }
        }
        if (!l.canDetectOrientation()) return
        l.enable()
        listener = l
    }

    private fun autoRotate() =
        runCatching { Settings.System.getInt(context.contentResolver, Settings.System.ACCELEROMETER_ROTATION) == 1 }.getOrDefault(true)

    companion object {
        /**
         * 传感器角度（0 = 竖着，90 = 左边朝上 / 右侧朝下，270 = 右边朝上 / 左侧朝下）→ 引擎方向。
         * 引擎的 90 是「手机左侧朝下」，也就是传感器的 270
         */
        fun map(angle: Int, current: Int): Int = when {
            angle in 240..300 -> 90
            angle in 60..120 -> -90
            angle <= 30 || angle >= 330 -> 0
            else -> current
        }
    }
}
