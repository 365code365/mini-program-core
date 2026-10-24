package dev.minirender.example

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.os.Bundle
import android.util.Log
import android.view.Gravity
import android.view.View
import android.widget.GridLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import kotlin.concurrent.thread

/**
 * 入口页（桌面图标进来的就是它），照微信下拉出来的小程序面板：
 * - 顶上一条搜索框 → [SearchActivity]（历史、热搜、联想、按分类筛）；
 * - 「最近使用」：本机打开过的，最新在前（[StoreLocal]，离线也在）；
 * - 分类一排，点了进搜索页看这个分类；
 * - 「全部小程序」：服务器上的（tools/mini-store），四列圆图标（服务器有 icon.png 就用它）；
 * - 最底下是开发用的：内置示例、上传诊断日志、诊断开关。
 * 点一个小程序 → [MainActivity] 带着它的信息起来，由它盖 loading 层、下载解包、打开。
 *
 * 验收脚本（android-native-check / android-jank / android-native-align）直接 `am start -n …/.MainActivity`
 * 打开内置示例，不经过这一页。商店地址可以用 intent 参数 `store` 换（`am start -n …/.StoreActivity --es store http://…`）
 */
class StoreActivity : Activity() {
    private lateinit var grid: GridLayout
    private lateinit var status: TextView
    private lateinit var store: MiniStore
    private lateinit var recentRow: LinearLayout
    private lateinit var recentBox: LinearLayout
    private lateinit var catBox: FlowLayout

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        store = MiniStore(this, intent.getStringExtra("store") ?: MiniStore.DEFAULT_URL)
        val col = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        col.addView(TextView(this).apply {
            text = "小程序"
            textSize = 22f
            typeface = Typeface.DEFAULT_BOLD
            setTextColor(Color.BLACK)
            setPadding(dp(16f), dp(14f), dp(16f), dp(10f))
        })
        col.addView(searchBar())
        recentBox = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; visibility = View.GONE }
        recentBox.addView(StoreWidgets.header(this, "最近使用"))
        recentRow = LinearLayout(this).apply { setPadding(dp(8f), dp(4f), dp(8f), 0) }
        recentBox.addView(android.widget.HorizontalScrollView(this).apply { isHorizontalScrollBarEnabled = false; addView(recentRow) })
        col.addView(recentBox)
        col.addView(StoreWidgets.header(this, "分类"))
        catBox = FlowLayout(this).apply { setPadding(dp(16f), dp(4f), dp(16f), dp(4f)) }
        col.addView(catBox)
        col.addView(LinearLayout(this).apply {
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(16f), dp(20f), dp(8f), 0)
            addView(TextView(context).apply {
                text = "全部小程序"
                textSize = 14f
                typeface = Typeface.DEFAULT_BOLD
                setTextColor(Color.BLACK)
            }, LinearLayout.LayoutParams(0, -2, 1f))
            addView(TextView(context).apply {
                text = "刷新"
                textSize = 14f
                setTextColor(StoreViews.GREEN)
                setPadding(dp(12f), dp(8f), dp(12f), dp(8f))
                setOnClickListener { refresh() }
            })
        })
        status = TextView(this).apply {
            textSize = 12f
            setTextColor(0x73000000)
            setPadding(dp(16f), 0, dp(16f), 0)
        }
        col.addView(status)
        grid = GridLayout(this).apply {
            columnCount = 4
            setPadding(dp(8f), dp(12f), dp(8f), dp(8f))
        }
        col.addView(grid)
        col.addView(StoreWidgets.header(this, "开发调试"))
        col.addView(row("内置示例", "APK 里的演示小程序（assets/demo）") {
            startActivity(Intent(this, MainActivity::class.java))
        })
        // 真机出问题、手边没 USB：把本进程日志传到 mini-store，开发机上 tools/mini-store/diag.sh 拉回来
        col.addView(row("上传诊断日志", "把运行日志、机型与版本传到服务器，交给开发分析") {
            DiagUpload.ask(this, store.baseUrl)
        })
        col.addView(diagRow())
        val scroll = ScrollView(this).apply {
            setBackgroundColor(Color.WHITE)
            addView(col)
            // targetSdk 35 在 Android 15 上强制 edge-to-edge：自己让出状态栏 / 导航条
            setOnApplyWindowInsetsListener { v, insets ->
                @Suppress("DEPRECATION")
                v.setPadding(0, insets.systemWindowInsetTop, 0, insets.systemWindowInsetBottom)
                insets
            }
        }
        setContentView(scroll)
        refresh()
    }

    private fun dp(v: Float) = StoreViews.dp(this, v)

    override fun onResume() {
        super.onResume()
        showRecent()
    }

    /** 灰底圆角的假搜索框：点了进搜索页（微信也是这样，入口页上不直接输入） */
    private fun searchBar() = LinearLayout(this).apply {
        gravity = Gravity.CENTER
        isClickable = true
        contentDescription = "搜索小程序"
        background = android.graphics.drawable.GradientDrawable().apply {
            cornerRadius = dp(8f).toFloat(); setColor(0xFFEDEDED.toInt())
        }
        addView(TextView(context).apply { text = "🔍  搜索小程序"; textSize = 15f; setTextColor(0x61000000) })
        setOnClickListener { SearchActivity.start(this@StoreActivity, store.baseUrl) }
        layoutParams = LinearLayout.LayoutParams(-1, dp(38f)).apply { setMargins(dp(16f), 0, dp(16f), dp(4f)) }
    }

    private fun showRecent() {
        if (!::recentRow.isInitialized) return
        val recent = StoreLocal(this).recent()
        recentBox.visibility = if (recent.isEmpty()) View.GONE else View.VISIBLE
        recentRow.removeAllViews()
        val w = (resources.displayMetrics.widthPixels - dp(16f)) / 4
        recent.forEach { recentRow.addView(cell(it), LinearLayout.LayoutParams(w, -2)) }
    }

    private fun row(title: String, sub: String, onClick: () -> Unit) = LinearLayout(this).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(16f), dp(12f), dp(16f), dp(12f))
        isClickable = true
        isFocusable = true
        setBackgroundResource(android.R.drawable.list_selector_background)
        addView(TextView(context).apply { text = title; textSize = 16f; setTextColor(Color.BLACK) })
        addView(TextView(context).apply { text = sub; textSize = 12f; setTextColor(0x73000000) })
        setOnClickListener { onClick() }
    }

    /**
     * 测试用的诊断开关：打开后小程序里每一帧慢在哪、为什么整帧、关闭耗时都打到 logcat
     * （`adb logcat -s mini-render`），不用 `adb shell setprop`。下一次打开小程序生效
     */
    private fun diagRow(): View {
        val prefs = getSharedPreferences(MainActivity.PREFS, MODE_PRIVATE)
        val sub = TextView(this)
        fun label() {
            val on = prefs.getBoolean(MainActivity.PREF_DIAG, false)
            sub.text = if (on) "已打开：adb logcat -s mini-render 看〔慢：…〕、整帧原因、[生命周期]" else "关着（点一下打开；下一次打开小程序生效）"
        }
        return row("诊断日志", "") {
            prefs.edit().putBoolean(MainActivity.PREF_DIAG, !prefs.getBoolean(MainActivity.PREF_DIAG, false)).apply()
            label()
        }.also { r ->
            // row() 的第二行是空的副标题：换成会变的那个
            (r as LinearLayout).removeViewAt(1)
            sub.textSize = 12f
            sub.setTextColor(0x73000000)
            r.addView(sub)
            label()
        }
    }

    private fun refresh() {
        status.text = "正在连接 ${store.baseUrl} …"
        thread(name = "store-list") {
            val l = store.list()
            runOnUiThread {
                if (isDestroyed) return@runOnUiThread
                status.text = when {
                    l.offline -> "服务器连不上，显示上次的列表（${l.error}）"
                    l.apps.isEmpty() -> "服务器上还没有小程序"
                    else -> "来自 ${store.baseUrl}"
                }
                Log.i(MainActivity.TAG, "[商店] 列表 ${l.apps.size} 个 offline=${l.offline}")
                grid.removeAllViews()
                showCategories(l.apps)
                val cell = (resources.displayMetrics.widthPixels - dp(16f)) / 4
                l.apps.forEach { grid.addView(cell(it), GridLayout.LayoutParams().apply { width = cell }) }
            }
        }
    }

    private fun cell(app: MiniStore.App) = LinearLayout(this).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_HORIZONTAL
        setPadding(0, dp(10f), 0, dp(10f))
        isClickable = true
        isFocusable = true
        contentDescription = "打开小程序 ${app.name}"
        setBackgroundResource(android.R.drawable.list_selector_background)
        addView(StoreIcons.view(store, app, 52f))
        addView(TextView(context).apply {
            text = app.name
            textSize = 12f
            maxLines = 1
            gravity = Gravity.CENTER_HORIZONTAL   // 竖排 LinearLayout 默认给子视图满宽，不居中的话字贴左
            ellipsize = android.text.TextUtils.TruncateAt.END
            setTextColor(Color.BLACK)
            setPadding(dp(4f), dp(8f), dp(4f), 0)
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        })
        setOnClickListener { StoreWidgets.open(this@StoreActivity, store, app) }
    }

    /** 分类按列表现算（离线时用缓存的列表也能出来），顺序跟服务器的固定分类表走 */
    private fun showCategories(apps: List<MiniStore.App>) {
        catBox.removeAllViews()
        val order = listOf("美食", "购物", "出行", "生活服务", "医疗健康", "金融", "工具", "教育", "娱乐", "资讯", "政务", "其他")
        val counts = apps.groupingBy { it.category.ifEmpty { "其他" } }.eachCount()
        order.filter { (counts[it] ?: 0) > 0 }.forEach { c ->
            catBox.addView(StoreWidgets.chip(this, "$c ${counts[c]}") { SearchActivity.start(this, store.baseUrl, c) })
        }
    }
}
