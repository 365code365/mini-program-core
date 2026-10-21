package dev.minirender

/**
 * 引擎的 JNI 绑定。**不要直接用它**，用 [MiniProgramView]（原生 View）或 Flutter 插件。
 *
 * 约定：所有方法必须在**同一个线程**调用（逻辑层是 QuickJS，不是线程安全的）。
 * [MiniProgramView] 与 Flutter 插件都已经用一条专用渲染线程包好了。
 *
 * 新增的方法（`nativeCreateEx` 往下）与 `.so` 一起发布：新的 Kotlin 配老的 `.so`
 * 调到它们时会抛 UnsatisfiedLinkError。
 */
object MiniEngine {
    init {
        System.loadLibrary("mini_render")
    }

    /** `pump_ex` 的返回位 */
    const val FRAME_NEW = 1
    const val FRAME_PRESENTED = 2
    const val FRAME_BACKEND_CHANGED = 4
    const val FRAME_PENDING = 8
    /** 引擎还有活要干（动画 / 惯性 / 定时器 / 请求在飞 / 按住的手指）：下一个 vsync 再 pump */
    const val FRAME_WANTS_MORE = 16
    /** 事件队列非空：[nativePollEvent] 取到 null 为止 */
    const val FRAME_EVENTS = 32

    /** 创建实例，返回句柄；0 表示失败 */
    external fun nativeCreate(
        appDir: String,
        dataDir: String?,
        width: Int,
        height: Int,
        dpr: Float,
    ): Long
    external fun nativeDestroy(handle: Long)
    /** route 传 null = app.json 的首页 */
    external fun nativeLaunch(handle: Long, route: String?): Boolean
    external fun nativeNavigate(handle: Long, url: String): Boolean
    /** 跑一帧；返回 true 说明有新画面该上屏 */
    external fun nativePump(handle: Long, nowMs: Long): Boolean
    /** 高 32 位 = 宽，低 32 位 = 高（都是设备像素） */
    external fun nativePixelSize(handle: Long): Long
    /** 把当前帧拷进 out（RGBA8），返回写入字节数 */
    external fun nativePixels(handle: Long, out: ByteArray): Int
    /** phase: 0 按下 / 1 移动 / 2 抬起 / 3 取消 */
    external fun nativePointer(handle: Long, phase: Int, x: Float, y: Float)
    external fun nativeTextInput(handle: Long, text: String)
    /** 按键名表见 MiniProgramView.sendKey（与引擎 KEY_NAMES 一致） */
    external fun nativeKey(handle: Long, name: String)
    /** 返回 false = 页面栈只剩一页，宿主该关掉小程序 */
    external fun nativeBack(handle: Long): Boolean
    /** what: 0 进前台 / 1 进后台 / 2 内存告警 */
    external fun nativeLifecycle(handle: Long, what: Int)

    // ───────────── 渲染后端 ─────────────
    /** 后端偏好 / 实际后端（`MR_BACKEND_*`）。偏好默认 cpu */
    const val BACKEND_AUTO = 0
    const val BACKEND_CPU = 1
    const val BACKEND_GPU = 2

    /** pref: [BACKEND_AUTO] / [BACKEND_CPU] / [BACKEND_GPU] */
    external fun nativeSetBackend(handle: Long, pref: Int): Boolean
    external fun nativeSetDeviceInfo(handle: Long, totalMem: Long, lowRam: Boolean, api: Int)
    external fun nativeBackend(handle: Long): Int
    external fun nativeBackendReason(handle: Long): String?
    /**
     * 把 Surface 交给 GPU 合成器，返回实际后端（2 GPU / 1 CPU / 0 参数错误）。
     * 返回 1 时这块 Surface 可能已被 GPU 连过：要 CPU 直写就换一块新的 Surface。
     * 偏好是 cpu（默认）时不碰 Surface，直接返回 1。
     */
    external fun nativeAttachSurface(handle: Long, surface: android.view.Surface): Int
    /** Surface 销毁前**同步**调：返回之后引擎不再碰它 */
    external fun nativeDetachSurface(handle: Long)
    /** 跑一帧，返回 FRAME_* 的按位或 */
    external fun nativePumpEx(handle: Long, nowMs: Long): Int

    // ───────────── 嵌入宿主（Flutter 插件 / 原生 View） ─────────────
    /**
     * 带选项创建（系统信息 / 安全区 / 宿主 API / 原生组件，JSON），见 `include/mini_render.h`
     * 的 `mr_app_create_ex`。选项格式错返回 0。
     */
    external fun nativeCreateEx(
        appDir: String,
        dataDir: String?,
        width: Int,
        height: Int,
        dpr: Float,
        optionsJson: String?,
    ): Long
    /** 改尺寸（逻辑像素）与像素比；变了返回 true（已出好新尺寸的一帧） */
    external fun nativeResize(handle: Long, width: Int, height: Int, dpr: Float): Boolean
    /** 安全区（逻辑像素） */
    external fun nativeSetSafeArea(handle: Long, top: Float, right: Float, bottom: Float, left: Float): Boolean
    /** 滚轮 / 触控板；precise = 像素级精密滚动 */
    external fun nativeWheel(handle: Long, dy: Float, precise: Boolean)
    /** 页面栈深度（Flutter `PopScope.canPop` 要提前知道） */
    external fun nativePageDepth(handle: Long): Int
    /** 把当前帧写进一张 ARGB_8888 Bitmap（尺寸要等于 [nativePixelSize]）；返回写入字节数，≤0 失败 */
    external fun nativePixelsToBitmap(handle: Long, bitmap: android.graphics.Bitmap): Int
    /** 挂一块给 CPU 直写的 Surface（之前挂的先放掉） */
    external fun nativeAttachCpuWindow(handle: Long, surface: android.view.Surface): Boolean
    /** 放掉 CPU 直写的 Surface（`onSurfaceCleanup` 里同步调） */
    external fun nativeDetachCpuWindow(handle: Long)
    /**
     * 把当前帧直接写进挂上的 Surface 并提交（零中间拷贝）。1 成功；≤0 失败 ——
     * 这块 Surface 不支持 CPU 锁（Flutter 的 ImageReader 后端可能如此），换新 Surface 走 Bitmap 路。
     */
    external fun nativeBlitToWindow(handle: Long): Int
    /** 取一条事件（JSON）；没有了返回 null */
    external fun nativePollEvent(handle: Long): String?
    /** 回 hostCall */
    external fun nativeHostReply(handle: Long, id: Int, ok: Boolean, resultJson: String?): Boolean
    /** 在逻辑层执行 JS（深链参数注入、集成测试）；抛异常返回 null */
    external fun nativeEval(handle: Long, code: String): String?
    /** 宿主承载的原生组件回传事件（`bind<name>`） */
    external fun nativeNativeViewEvent(handle: Long, key: String, name: String, detailJson: String?): Boolean
}
