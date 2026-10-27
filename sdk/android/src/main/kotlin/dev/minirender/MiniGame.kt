package dev.minirender

/**
 * 小游戏的 JNI 绑定（Rust 侧 `src/ffi_jni_game.rs`）。所有调用必须在同一条引擎线程上（[MiniGameView] 的 HandlerThread）。
 * 每个入口都有 panic 防护：引擎内部出错时实例失效，之后的调用返回失败值，事件队列里有一条 `{"type":"error","kind":"panic"}`。
 */
object MiniGame {
    init {
        System.loadLibrary("mini_render")
    }

    /** 主画布这一帧画过：要 [nativeBlit] 上屏 */
    const val FRAME_NEW = 1
    /** 还有活（挂着的 rAF、1 秒内的定时器、在飞的请求）：下一个 vsync 再来 */
    const val FRAME_WANTS_MORE = 16
    /** 事件队列里有东西 */
    const val FRAME_EVENTS = 32
    /** 只剩远期定时器：按 [nativeNextWakeUs] 定时唤醒，不要每个 vsync 都来 */
    const val FRAME_WAKE_AT = 128

    /** options：`{"appId","systemInfo","launchOptions","watchdogMs","jsMemoryLimitMB"}`。失败返回 0（原因在 logcat） */
    external fun nativeCreate(dir: String, dataDir: String, width: Int, height: Int, dpr: Float, options: String): Long
    external fun nativeDestroy(handle: Long)
    /** nowUs：Choreographer 的 frameTimeNanos / 1000（**微秒**） */
    external fun nativePump(handle: Long, nowUs: Long): Int
    external fun nativePollEvent(handle: Long): String?
    external fun nativeEval(handle: Long, code: String): String?
    external fun nativeSetBackground(handle: Long, background: Boolean)
    /**
     * 一批指针事件：每个 6 个 float `[phase(0 下/1 移/2 抬/3 取消), pointerId, x, y, force, 相对 baseUs 的毫秒]`，
     * 坐标是视图内逻辑像素；baseUs 与 [nativePump] 同一时钟（MotionEvent 的 eventTime 与 Choreographer 都是 CLOCK_MONOTONIC）
     */
    external fun nativeTouchBatch(handle: Long, data: FloatArray, count: Int, baseUs: Long)
    external fun nativePreferredFps(handle: Long): Int
    /** 音频输出统计 JSON（frames / latencyMs / block / callbacks / xruns / rate / device / voices） */
    external fun nativeAudioStats(handle: Long): String?
    external fun nativeNextWakeUs(handle: Long): Long
    external fun nativeAttachWindow(handle: Long, surface: android.view.Surface): Boolean
    external fun nativeDetachWindow(handle: Long)
    /** 1 = 上屏了；0 = 没挂 surface；负数 = 锁不上 */
    external fun nativeBlit(handle: Long): Int
    /** 最近一次上屏：高 32 位 = 等缓冲 µs（vsync 背压，不算引擎工作量），低 32 位 = 拷贝 + 提交 µs */
    external fun nativeBlitStats(handle: Long): Long
    /** 其中提交（unlockAndPost）的微秒数 */
    external fun nativeBlitPostUs(handle: Long): Long
}
