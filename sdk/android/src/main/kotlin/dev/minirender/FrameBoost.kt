package dev.minirender

import android.annotation.TargetApi
import android.content.Context
import android.hardware.display.DisplayManager
import android.os.Build
import android.os.PerformanceHintManager
import android.os.Process
import android.util.Log
import android.view.Display

/**
 * 引擎线程的 CPU 性能提示（ADPF：`PerformanceHintManager`，Android 12+）。只在引擎线程上调用。
 *
 * 为什么要：引擎线程每个 vsync 醒一次、干 1~3ms 活就睡，调度器按「利用率低」把它放在小核上。
 * 一加 PLY110（SM7635：4×A520 1.8GHz + 3×A720 2.4GHz + 1×A720 2.7GHz）实测滑 WXSS 页时
 * 92% 的时间在 0~3 号小核上 —— 桌面 0.3ms 的补条带，手机上 5~11ms，复杂页面一滑就掉帧，
 * 这就是「越复杂的页面越顿」。nice 值只改 CFS 权重、不管大小核；ADPF 才是系统给的那条路：
 * 报告「每帧实际干了多久」与「目标时长」，超了系统就提频 / 迁大核。
 *
 * 目标时长取刷新周期的一半：取满一个周期的话，10ms 的补条带在 11ms 的目标下算「达标」，
 * 系统不会提频，而合成、上屏还要再占几毫秒。
 *
 * 没用 `sendHint(CPU_LOAD_UP)`：它不在公开 SDK 里（android-36 的 android.jar 里 Session 只有
 * report / updateTarget / setThreads / setPreferPowerEfficiency）。
 *
 * 低于 Android 12、或系统不给会话（`createHintSession` 返回 null）时什么都不做，只留 nice 值。
 * 诊断：logcat `mini-render` 里有一行「⚡ ADPF …」。
 */
class FrameBoost private constructor(private val impl: Impl?) {
    internal interface Impl {
        fun report(workNanos: Long)
        fun close()
    }

    /** 这一帧引擎实际干活的时长（pump + 上屏），没干活的空转帧别报 */
    fun report(workNanos: Long) {
        if (workNanos > 0) impl?.report(workNanos)
    }

    fun close() = impl?.close()

    companion object {
        private const val TAG = "mini-render"

        /** 在引擎线程上调：给当前线程开一个性能提示会话 */
        fun forCurrentThread(context: Context): FrameBoost {
            // 与系统 RenderThread 同一档的 nice（DISPLAY = -4）：抢不过 UI / 输入，但不被后台任务挤
            runCatching { Process.setThreadPriority(Process.THREAD_PRIORITY_DISPLAY) }
            if (Build.VERSION.SDK_INT < 31) {
                Log.i(TAG, "⚡ ADPF 不可用（API ${Build.VERSION.SDK_INT} < 31），只调了线程优先级")
                return FrameBoost(null)
            }
            val period = periodNanos(context)
            val impl = runCatching { Adpf.create(context, Process.myTid(), period / 2) }.getOrNull()
            Log.i(TAG, if (impl != null) "⚡ ADPF 会话已建：目标 ${period / 2 / 1000}µs（刷新周期 ${period / 1000}µs）"
                else "⚡ ADPF 系统没给会话，只调了线程优先级")
            return FrameBoost(impl)
        }

        private fun periodNanos(context: Context): Long {
            val dm = context.getSystemService(Context.DISPLAY_SERVICE) as? DisplayManager
            val hz = dm?.getDisplay(Display.DEFAULT_DISPLAY)?.refreshRate ?: 60f
            return (1e9 / hz.coerceIn(30f, 240f)).toLong()
        }
    }
}

@TargetApi(31)
private class Adpf(private val s: PerformanceHintManager.Session) : FrameBoost.Impl {
    override fun report(workNanos: Long) = s.reportActualWorkDuration(workNanos)
    override fun close() = s.close()

    companion object {
        fun create(context: Context, tid: Int, targetNanos: Long): Adpf? {
            val m = context.getSystemService(PerformanceHintManager::class.java) ?: return null
            val s = m.createHintSession(intArrayOf(tid), targetNanos) ?: return null
            return Adpf(s)
        }
    }
}
