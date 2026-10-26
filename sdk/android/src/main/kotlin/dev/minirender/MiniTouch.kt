package dev.minirender

import android.view.MotionEvent

/**
 * 触摸 → 引擎（从 MiniProgramView 搬出来：那个文件过了 500 行）。坐标换成逻辑像素。
 *
 * 页面的触摸状态机是单指的：第一根手指走 `nativePointer*`。第二根手指单独走 `nativePointer2`
 * （只有图片预览认它：双指缩放）；第三根及以后不送。
 * 第一根先抬起、第二根还按着：先报第二根抬起，之后把它当第一根接着送（引擎那边接住位置的跳变）——
 * 不报第一根抬起，否则页面 / 预览会把它当成一次点击。
 */
internal class MiniTouch {
    private var primary = -1
    private var secondary = -1

    fun forward(session: EngineSession, event: MotionEvent, dpr: Float): Boolean {
        fun send(phase: Int, x: Float, y: Float) = session.run { MiniEngine.nativePointer(it, phase, x / dpr, y / dpr) }
        fun send2(phase: Int, x: Float, y: Float) = session.run { MiniEngine.nativePointer2(it, phase, x / dpr, y / dpr) }
        // 移动带上事件时间（uptimeMillis，与 pump 用的 Choreographer 帧时间同一个单调钟）：
        // 引擎按它重采样页面拖动，post 到哪一帧不再影响每帧挪多少（慢拖不一顿一顿）
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                primary = event.getPointerId(0)
                secondary = -1
                send(0, event.x, event.y)
            }
            MotionEvent.ACTION_POINTER_DOWN -> {
                val i = event.actionIndex
                if (secondary == -1 && event.getPointerId(i) != primary) {
                    secondary = event.getPointerId(i)
                    send2(0, event.getX(i), event.getY(i))
                }
            }
            MotionEvent.ACTION_MOVE -> {
                val pi = event.findPointerIndex(primary)
                if (pi < 0) return true
                // 历史点也要送：抽稀会让滑动手感变差，也会漏掉快速滑动的方向判定。
                // 一个 MotionEvent 的全部采样打成一包、一次 post（与 iOS 的 coalescedTouches 一致）：
                // 从前每个历史点各 post 一个 Runnable、各挂一次帧，120Hz 触控上一帧三四个
                val n = event.historySize + 1
                val xs = FloatArray(n)
                val ys = FloatArray(n)
                val ts = LongArray(n)
                for (i in 0 until n - 1) {
                    xs[i] = event.getHistoricalX(pi, i) / dpr
                    ys[i] = event.getHistoricalY(pi, i) / dpr
                    ts[i] = event.getHistoricalEventTime(i)
                }
                xs[n - 1] = event.getX(pi) / dpr
                ys[n - 1] = event.getY(pi) / dpr
                ts[n - 1] = event.eventTime
                val si = if (secondary == -1) -1 else event.findPointerIndex(secondary)
                val sx = if (si >= 0) event.getX(si) / dpr else 0f
                val sy = if (si >= 0) event.getY(si) / dpr else 0f
                session.run { h ->
                    for (i in 0 until n) MiniEngine.nativePointerMoveAt(h, xs[i], ys[i], ts[i].toDouble())
                    if (si >= 0) MiniEngine.nativePointer2(h, 1, sx, sy)
                }
            }
            MotionEvent.ACTION_POINTER_UP -> {
                val i = event.actionIndex
                val id = event.getPointerId(i)
                when (id) {
                    secondary -> {
                        send2(2, event.getX(i), event.getY(i))
                        secondary = -1
                    }
                    primary -> if (secondary != -1) {
                        // 第一根先抬：第二根接班当第一根（见类注释）
                        val j = event.findPointerIndex(secondary)
                        if (j >= 0) send2(2, event.getX(j), event.getY(j))
                        primary = secondary
                        secondary = -1
                    }
                }
            }
            MotionEvent.ACTION_UP -> {
                if (secondary != -1) send2(2, event.x, event.y)
                primary = -1
                secondary = -1
                send(2, event.x, event.y)
            }
            MotionEvent.ACTION_CANCEL -> {
                if (secondary != -1) send2(3, 0f, 0f)
                primary = -1
                secondary = -1
                send(3, 0f, 0f)
            }
            else -> return false
        }
        return true
    }
}
