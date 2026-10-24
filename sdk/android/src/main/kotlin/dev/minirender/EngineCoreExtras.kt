package dev.minirender

import android.graphics.Bitmap
import android.util.Log
import java.io.File
import java.io.FileOutputStream
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

// [EngineCore] 里不碰出帧状态的那几样：同步命令、存帧、上屏计时。拆出来让 EngineCore.kt 只剩出帧与上屏本身

/**
 * 同步等引擎线程的结果（返回键要立刻决定 finish 还是不动）。引擎没建好返回 [fallback]。
 * 等到做完为止：超时就当「没消费」的话，JS 卡一下用户按返回就整个退出了
 */
internal fun <T> EngineCore.call(fallback: T, block: (Long) -> T): T {
    if (disposed || !ready) return fallback
    val latch = CountDownLatch(1)
    val box = AtomicReference(fallback)
    engine.post {
        try {
            if (handle != 0L) {
                box.set(block(handle))
                requestFrame()
            }
        } finally {
            latch.countDown()
        }
    }
    while (!latch.await(500, TimeUnit.MILLISECONDS)) if (disposed) break
    return box.get()
}

/** 把引擎当前这一帧存成 PNG（SurfaceView 的内容 `View.draw` 取不到）。[done] 在主线程回调 */
internal fun EngineCore.dumpFrame(file: File, done: (Boolean) -> Unit) = engine.post {
    var ok = false
    if (handle != 0L && pixelW > 0 && pixelH > 0) {
        val bmp = Bitmap.createBitmap(pixelW, pixelH, Bitmap.Config.ARGB_8888)
        if (MiniEngine.nativePixelsToBitmap(handle, bmp) > 0) {
            ok = runCatching { FileOutputStream(file).use { bmp.compress(Bitmap.CompressFormat.PNG, 100, it) } }
                .getOrDefault(false)
        }
        bmp.recycle()
    }
    main.post { done(ok) }
}

/** `debug.mini.fps`：每秒一行「上屏」耗时（`ANativeWindow_lock` 等空闲缓冲 + 写像素），引擎的 📊 行不含它 */
internal class BlitStats {
    private var start = 0L
    private var n = 0
    private var max = 0f
    private var sum = 0f

    fun record(ms: Float) {
        val now = System.nanoTime()
        if (start == 0L) start = now
        n++; sum += ms; if (ms > max) max = ms
        if (now - start >= 1_000_000_000L) {
            Log.i("mini-render", "📤 上屏 $n 次，平均 ${"%.1f".format(sum / n)}ms 最慢 ${"%.1f".format(max)}ms")
            start = now; n = 0; sum = 0f; max = 0f
        }
    }
}

/**
 * 引擎的销毁在引擎线程上异步做（[EngineCore.dispose]），这里记着「还有一个没销毁完」：
 * 下一个引擎建之前在它自己的引擎线程上等（一个进程同一时刻只能有一个引擎实例：小程序根目录、数据目录是进程级的）。
 * 主线程只等「放手 Surface」那一小段，耗时打 `[生命周期]` 一行（`adb logcat -s mini-render`）
 */
internal object EngineTeardown {
    @Volatile private var pending: CountDownLatch? = null

    /** 主线程：放手 Surface 之后、开始异步销毁之前 */
    fun begin(t0: Long): CountDownLatch {
        val l = CountDownLatch(1)
        pending = l
        Log.i("mini-render", "[生命周期] 关闭：主线程等放手 Surface ${"%.1f".format((System.nanoTime() - t0) / 1e6)}ms，销毁转到引擎线程")
        return l
    }

    /** 引擎线程：销毁做完 */
    fun done(l: CountDownLatch, t: Long) {
        Log.i("mini-render", "[生命周期] 引擎线程上销毁引擎 ${"%.1f".format((System.nanoTime() - t) / 1e6)}ms")
        l.countDown()
    }

    /** 新引擎的线程上：等上一个销毁完（最多 3 秒，超时照建，打一行警告） */
    fun awaitPrevious() {
        val l = pending ?: return
        val t = System.nanoTime()
        if (!l.await(3, TimeUnit.SECONDS)) Log.w("mini-render", "[生命周期] 上一个引擎 3 秒还没销毁完，照常建新的")
        val ms = (System.nanoTime() - t) / 1e6
        if (ms >= 1) Log.i("mini-render", "[生命周期] 新引擎等上一个销毁完 ${"%.1f".format(ms)}ms")
    }
}
