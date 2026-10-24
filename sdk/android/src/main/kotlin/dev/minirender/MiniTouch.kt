package dev.minirender

import android.view.MotionEvent

/** 触摸 → 引擎（从 MiniProgramView 搬出来：那个文件过了 500 行）。坐标换成逻辑像素 */
internal object MiniTouch {
    fun forward(session: EngineSession, event: MotionEvent, dpr: Float): Boolean {
        fun send(phase: Int, x: Float, y: Float) = session.run { MiniEngine.nativePointer(it, phase, x / dpr, y / dpr) }
        // 移动带上事件时间（uptimeMillis，与 pump 用的 Choreographer 帧时间同一个单调钟）：
        // 引擎按它重采样页面拖动，post 到哪一帧不再影响每帧挪多少（慢拖不一顿一顿）
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> send(0, event.x, event.y)
            MotionEvent.ACTION_MOVE -> {
                // 历史点也要送：抽稀会让滑动手感变差，也会漏掉快速滑动的方向判定。
                // 一个 MotionEvent 的全部采样打成一包、一次 post（与 iOS 的 coalescedTouches 一致）：
                // 从前每个历史点各 post 一个 Runnable、各挂一次帧，120Hz 触控上一帧三四个
                val n = event.historySize + 1
                val xs = FloatArray(n)
                val ys = FloatArray(n)
                val ts = LongArray(n)
                for (i in 0 until n - 1) {
                    xs[i] = event.getHistoricalX(i) / dpr
                    ys[i] = event.getHistoricalY(i) / dpr
                    ts[i] = event.getHistoricalEventTime(i)
                }
                xs[n - 1] = event.x / dpr
                ys[n - 1] = event.y / dpr
                ts[n - 1] = event.eventTime
                session.run { h -> for (i in 0 until n) MiniEngine.nativePointerMoveAt(h, xs[i], ys[i], ts[i].toDouble()) }
            }
            MotionEvent.ACTION_UP -> send(2, event.x, event.y)
            MotionEvent.ACTION_CANCEL -> send(3, 0f, 0f)
            else -> return false
        }
        return true
    }
}
