package dev.minirender.flutter

import android.app.Activity
import android.app.Application
import android.content.Context
import android.os.Bundle
import android.view.View
import dev.minirender.MiniGameView
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugin.common.StandardMessageCodec
import io.flutter.plugin.platform.PlatformView
import io.flutter.plugin.platform.PlatformViewFactory
import java.io.File

/**
 * 小游戏的平台视图（Dart 侧 `MiniGameView`）：直接嵌原生 SDK 的 [MiniGameView]（同一份源码编进插件，见 build.gradle.kts），
 * 与原生示例的 GameActivity 走同一条路 —— 独立 SurfaceView + 引擎线程上屏、触摸 MotionEvent 原样进 [MiniGameView.onTouchEvent]。
 *
 * 与原生 GameActivity 不同的几处（Flutter 只有一个 Activity，游戏页只是其中一页）：
 * - **方向**：[MiniGameView] 建的时候按 game.json 锁 Activity 的方向；这里记下锁之前的值，视图销毁时还回去，
 *   否则退出横屏游戏后整个 App 还横着；
 * - **前后台**：GameActivity 在 onResume / onPause 里转发；这里听宿主 Activity 的生命周期回调转发（Dart 不用管）；
 * - **宿主事件**：`exit` / `restart` / `hostCall` 等走每个视图自己的通道 `dev.minirender/game_<id>` 交给 Dart。
 *
 * 创建参数（Map）：`dir`（解好的包，含 game.json）、`dataDir`（可空，缺省 filesDir/game-data，与原生示例同一处）、
 * `options`（mr_game_create 的选项 JSON 串）。
 */
internal class GameViewFactory(
    private val messenger: BinaryMessenger,
    private val activityOf: () -> Activity?,
) : PlatformViewFactory(StandardMessageCodec.INSTANCE) {
    override fun create(context: Context, viewId: Int, args: Any?): PlatformView {
        val a = args as? Map<*, *> ?: emptyMap<String, Any>()
        // 锁方向要 Activity：平台视图拿到的 context 不一定是它（可能是包了一层的 ContextWrapper）
        val ctx: Context = activityOf() ?: context
        val dataDir = (a["dataDir"] as? String)?.takeIf { it.isNotEmpty() }
            ?: File(ctx.filesDir, "game-data").apply { mkdirs() }.absolutePath
        return GameView(ctx, viewId, messenger, a["dir"] as? String ?: "", dataDir, a["options"] as? String ?: "{}")
    }
}

internal class GameView(
    context: Context,
    viewId: Int,
    messenger: BinaryMessenger,
    dir: String,
    dataDir: String,
    options: String,
) : PlatformView {
    private val activity = context as? Activity
    /** 游戏锁方向之前 Activity 的方向：退出时还原 */
    private val savedOrientation = activity?.requestedOrientation
    private val channel = MethodChannel(messenger, "dev.minirender/game_$viewId")
    private val view = MiniGameView(context, dir, dataDir, options)
    private var disposed = false

    private val lifecycle = object : Application.ActivityLifecycleCallbacks {
        override fun onActivityResumed(a: Activity) { if (a === activity) view.onResume() }
        override fun onActivityPaused(a: Activity) { if (a === activity) view.onPause() }
        override fun onActivityCreated(a: Activity, b: Bundle?) {}
        override fun onActivityStarted(a: Activity) {}
        override fun onActivityStopped(a: Activity) {}
        override fun onActivitySaveInstanceState(a: Activity, b: Bundle) {}
        override fun onActivityDestroyed(a: Activity) {}
    }

    init {
        activity?.application?.registerActivityLifecycleCallbacks(lifecycle)
        // 主线程回调：type 与原始 JSON 原样交给 Dart（exit / restart / hostCall / report …）
        view.onHostEvent = { type, o -> if (!disposed) channel.invokeMethod("event", mapOf("type" to type, "json" to o.toString())) }
        // 第一帧上屏：Dart 的 loading 层等它再撤（之前 SurfaceView 是空的，露黑底）
        view.onFirstFrame = { if (!disposed) channel.invokeMethod("firstFrame", null) }
        channel.setMethodCallHandler { call, result ->
            when (call.method) {
                "hostReply" -> {
                    view.hostReply(call.argument<Int>("id") ?: 0, call.argument<Boolean>("ok") ?: false, call.argument<String>("json") ?: "{}")
                    result.success(null)
                }
                "dispatchEvent" -> {
                    view.dispatchEvent(call.argument<String>("name") ?: "", call.argument<String>("json") ?: "{}")
                    result.success(null)
                }
                "eval" -> view.eval(call.argument<String>("js") ?: "") { result.success(it) }
                else -> result.notImplemented()
            }
        }
    }

    override fun getView(): View = view

    override fun dispose() {
        if (disposed) return
        disposed = true
        channel.setMethodCallHandler(null)
        activity?.application?.unregisterActivityLifecycleCallbacks(lifecycle)
        view.dispose()
        if (savedOrientation != null) activity?.requestedOrientation = savedOrientation
    }
}
