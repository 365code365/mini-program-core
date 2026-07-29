package dev.minirender

/**
 * 引擎的 JNI 绑定。**不要直接用它**，用 [MiniProgramView]。
 *
 * 约定：所有方法必须在**同一个线程**调用（逻辑层是 QuickJS，不是线程安全的）。
 * [MiniProgramView] 已经用一条专用渲染线程包好了。
 */
internal object MiniEngine {

    init {
        System.loadLibrary("mini_render")
    }

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

    /** enter / backspace / delete / left / right / home / end / selectall / escape */
    external fun nativeKey(handle: Long, name: String)

    /** 返回 false = 页面栈只剩一页，宿主该关掉小程序 */
    external fun nativeBack(handle: Long): Boolean

    /** what: 0 进前台 / 1 进后台 / 2 内存告警 */
    external fun nativeLifecycle(handle: Long, what: Int)
}
