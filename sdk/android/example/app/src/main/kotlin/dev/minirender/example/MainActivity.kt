package dev.minirender.example

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.HapticFeedbackConstants
import android.view.View
import android.view.WindowManager
import dev.minirender.MiniProgramView
import org.json.JSONObject
import java.io.File

/**
 * 原生集成的最小宿主：整个 Activity 只有一个 [MiniProgramView]，照 doc/原生App集成指南.md 「极简集成」那几行写。
 *
 * 验收用的两个 intent 参数（tools/android-native-check.sh 在用）：
 * - `route`：起始页（深链），如 `pages/detail/detail?from=dump`；
 * - `dump`：第一条 `page` 事件之后 2 秒，把视图当前画的内容存成 `files/engine_frame.png`，并在日志打一行
 *   `MINI_DUMP`（视图在屏幕上的位置与尺寸）。与 `adb exec-out screencap` 裁出的同一块逐像素比，
 *   差异为 0 才说明画面没被拉伸、颜色通道没反。
 *
 * 引擎发来的每条事件都打到 logcat（tag `mini-example`），脚本靠 `[事件] page <路由>` 判断进没进对页面。
 */
class MainActivity : Activity() {
    private lateinit var mini: MiniProgramView
    private val main = Handler(Looper.getMainLooper())
    private var dumpPending = false
    private var geomLogged = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        edgeToEdge()
        val dir = DemoAssets.unpack(this)
        mini = MiniProgramView(this)
        // 登录、支付、定位、扫码……要 App 自己接的 wx.*（选图 / 选文件 SDK 自带，不用登记）
        mini.hostApis.putAll(DemoHostApis(this).all)
        // 宿主承载的 <map>：同层渲染（宿主画、引擎画进页面），滑动时跟页面在同一张图里走
        mini.sameLayerViews["map"] = { DemoMapView() }
        mini.onEvent = { ev ->
            Log.i(TAG, "[事件] " + summarize(ev))
            if (ev.optString("type") == "vibrate") vibrate(ev)
        }
        setContentView(mini)
        dumpPending = intent.getBooleanExtra("dump", false)
        mini.open(dir, intent.getStringExtra("route"))
    }

    /**
     * 画面铺到状态栏下面：引擎按 insets 算 `statusBarHeight`、胶囊位置与 `env(safe-area-inset-*)`。
     * targetSdk 35 在 Android 15 上本来就是 edge-to-edge，这里给低版本补上
     */
    @Suppress("DEPRECATION")
    private fun edgeToEdge() {
        window.addFlags(WindowManager.LayoutParams.FLAG_DRAWS_SYSTEM_BAR_BACKGROUNDS)
        window.statusBarColor = Color.TRANSPARENT
        window.navigationBarColor = Color.TRANSPARENT
        window.decorView.systemUiVisibility = View.SYSTEM_UI_FLAG_LAYOUT_STABLE or
            View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN or
            View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION or
            View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR
        if (Build.VERSION.SDK_INT >= 28) {
            window.attributes = window.attributes.apply {
                layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
            }
        }
    }

    private fun summarize(e: JSONObject): String {
        val type = e.optString("type")
        if (type == "page") {
            // 视图几何（设备像素）：验收脚本按它把逻辑坐标换成点击位置、裁掉系统画的状态栏 / 导航条
            if (!geomLogged) {
                geomLogged = true
                Log.i(TAG, "MINI_GEOM " + geometry())
            }
            // 第一条 page 事件之后再等 2 秒存帧（冷启动要加载字体）
            if (dumpPending) {
                dumpPending = false
                main.postDelayed(::dump, 2000)
            }
            val title = e.optJSONObject("window")?.optString("navigationBarTitleText")
            return "page ${e.optString("route")} depth=${e.optInt("depth")} title=$title"
        }
        val copy = JSONObject(e.toString()).apply { remove("type") }
        return "$type $copy"
    }

    /**
     * 引擎当前这一帧 → PNG（引擎像素尺寸，在视图里 1:1 摆在左上角）。画面在 SurfaceView 上，
     * `View.draw(canvas)` 截不到（截出来是背景色），所以走 [MiniProgramView.dumpFrame]
     */
    private fun dump() {
        val out = File(getExternalFilesDir(null), "engine_frame.png")
        // w / h 报画面（SurfaceView）的尺寸 = 引擎像素尺寸：视图宽 1080、引擎 1079 时画面只占左边 1079 列
        mini.dumpFrame(out) { ok ->
            Log.i(TAG, "MINI_DUMP ok=$ok " + geometry(mini.pixelWidth, mini.pixelHeight) + " backend=${mini.activeBackend}")
        }
    }

    @Suppress("DEPRECATION")
    private fun geometry(w: Int = mini.width, h: Int = mini.height): String {
        val loc = IntArray(2).also { mini.getLocationOnScreen(it) }
        val i = mini.rootWindowInsets
        return "x=${loc[0]} y=${loc[1]} w=$w h=$h " +
            "statusBar=${i?.systemWindowInsetTop ?: 0} bottom=${i?.systemWindowInsetBottom ?: 0} " +
            "density=${resources.displayMetrics.density}"
    }

    /**
     * `wx.vibrateShort` / `vibrateLong`：触感反馈不用 VIBRATE 权限（与 Flutter 示例的 HapticFeedback 同一档）。
     * 长振动用 LONG_PRESS 那一档代替（没权限时 Vibrator 振不了）
     */
    private fun vibrate(ev: JSONObject) {
        val kind = when {
            ev.optString("duration") == "long" -> HapticFeedbackConstants.LONG_PRESS
            ev.optString("style") == "light" -> HapticFeedbackConstants.CLOCK_TICK
            else -> HapticFeedbackConstants.VIRTUAL_KEY
        }
        mini.performHapticFeedback(kind)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        intent.getStringExtra("route")?.let { mini.navigate("/$it") }
    }

    @Deprecated("Activity.onBackPressed：minSdk 24 上没有 OnBackInvokedCallback，两条都要能走")
    override fun onBackPressed() {
        if (!mini.goBack()) super.onBackPressed()   // 页面栈空了才退出
    }

    override fun onResume() { super.onResume(); mini.onHostResume() }
    override fun onPause() { super.onPause(); mini.onHostPause() }
    override fun onTrimMemory(level: Int) { super.onTrimMemory(level); mini.onHostTrimMemory() }
    override fun onDestroy() { super.onDestroy(); mini.close() }

    companion object { const val TAG = "mini-example" }
}
