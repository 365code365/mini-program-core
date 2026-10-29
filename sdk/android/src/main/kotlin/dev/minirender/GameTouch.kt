package dev.minirender

import android.os.Build
import android.view.MotionEvent

/**
 * 小游戏的触摸打包（game-doc/02 §7.1；从 MiniGameView.kt 拆出来的，行为不变）：一个 MotionEvent 拆成「每根指针一次」——
 * POINTER_DOWN / UP 只报那一根，MOVE 把全部历史采样按时间顺序、每根指针各报一次（游戏算手势速度要采样密度），CANCEL 报全部指针。
 * UI 线程上只打包，一次 post 到引擎线程、一次 JNI（`nativeTouchBatch`）；派发在下一帧 pump 的 rAF 之前。
 */
internal object GameTouch {
    /** 一批：每个 6 个 float `[phase, pointerId, x, y, force, 相对 baseNanos 的毫秒]`，坐标是逻辑像素 */
    class Batch(val data: FloatArray, val count: Int, val baseNanos: Long)

    /** 事件时刻（ns）：API 34 起有纳秒精度的版本 */
    private fun eventNanos(e: MotionEvent): Long =
        if (Build.VERSION.SDK_INT >= 34) e.eventTimeNanos else e.eventTime * 1_000_000

    /** 不认识的动作返回 null */
    fun pack(e: MotionEvent, d: Float): Batch? {
        val n = e.pointerCount
        val buf = FloatArray(6 * n * (e.historySize + 1))
        var k = 0
        val baseNanos = eventNanos(e)
        fun add(phase: Int, i: Int, x: Float, y: Float, p: Float, tNanos: Long) {
            buf[k++] = phase.toFloat(); buf[k++] = e.getPointerId(i).toFloat()
            buf[k++] = x / d; buf[k++] = y / d; buf[k++] = p.coerceIn(0f, 1f)
            buf[k++] = ((tNanos - baseNanos) / 1e6).toFloat()
        }
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                val i = e.actionIndex
                add(0, i, e.getX(i), e.getY(i), e.getPressure(i), baseNanos)
            }
            MotionEvent.ACTION_MOVE -> {
                for (h in 0 until e.historySize) {
                    val t = e.getHistoricalEventTime(h) * 1_000_000
                    for (i in 0 until n) add(1, i, e.getHistoricalX(i, h), e.getHistoricalY(i, h), e.getHistoricalPressure(i, h), t)
                }
                for (i in 0 until n) add(1, i, e.getX(i), e.getY(i), e.getPressure(i), baseNanos)
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                val i = e.actionIndex
                add(2, i, e.getX(i), e.getY(i), 0f, baseNanos)
            }
            MotionEvent.ACTION_CANCEL -> for (i in 0 until n) add(3, i, e.getX(i), e.getY(i), 0f, baseNanos)
            else -> return null
        }
        return Batch(buf, k / 6, baseNanos)
    }
}
