package dev.minirender.example

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.util.Log
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.GridLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import kotlin.concurrent.thread

/**
 * 入口页（桌面图标进来的就是它）。灰底上一张张白卡片：
 * - 顶上一条搜索框 → [SearchActivity]（历史、热搜、联想、按分类筛）；
 * - 「最近使用」：本机打开过的，最新在前（[StoreLocal]，离线也在）；
 * - 分类一排，点了进搜索页看这个分类；
 * - 「全部小程序」：服务器上的（tools/mini-store），四列圆图标，收藏过的右上角带星；
 * - 最底下是开发用的：内置示例、上传诊断日志、诊断开关。
 * 滚到顶再往下拉（或点标题栏右边的 ⊞）：微信式的下拉面板（[StorePanel]，「最近使用」「我的小程序」）。
 * 任何一个图标长按：操作单（收藏 / 移出、从最近使用删除，[StoreSheets.appActions]）。
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
    private lateinit var drawer: PullDrawerLayout
    private lateinit var panel: StorePanel
    private var apps: List<MiniStore.App> = emptyList()
    private val onLocal: () -> Unit = { showRecent(); showGrid(); panel.refresh() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        store = MiniStore(this, intent.getStringExtra("store") ?: MiniStore.DEFAULT_URL)
        val col = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(0, 0, 0, dp(24f)) }
        col.addView(searchBar())
        recentRow = LinearLayout(this).apply { setPadding(dp(8f), 0, dp(8f), 0) }
        recentBox = card("最近使用", hint = "长按可删除").apply {
            visibility = View.GONE
            addView(android.widget.HorizontalScrollView(context).apply { isHorizontalScrollBarEnabled = false; addView(recentRow) })
        }
        col.addView(recentBox)
        catBox = FlowLayout(this).apply { setPadding(dp(16f), dp(4f), dp(16f), 0) }
        col.addView(card("分类").apply { addView(catBox) })
        status = TextView(this).apply {
            textSize = 12f
            setTextColor(0x73000000)
            setPadding(dp(16f), 0, dp(16f), 0)
        }
        grid = GridLayout(this).apply {
            columnCount = 4
            setPadding(dp(8f), dp(8f), dp(8f), 0)
        }
        col.addView(card("全部小程序", action = "刷新") { refresh() }.apply { addView(status); addView(grid) })
        col.addView(card("开发调试").apply {
            addView(row("内置示例", "APK 里的演示小程序（assets/demo）") {
                startActivity(Intent(this@StoreActivity, MainActivity::class.java))
            })
            // 真机出问题、手边没 USB：把本进程日志传到 mini-store，开发机上 tools/mini-store/diag.sh 拉回来
            addView(row("上传诊断日志", "把运行日志、机型与版本传到服务器，交给开发分析") {
                DiagUpload.ask(this@StoreActivity, store.baseUrl)
            })
            addView(diagRow())
        })
        val scroll = ScrollView(this).apply { setBackgroundColor(PAGE_BG); addView(col) }
        val page = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            // 白底：状态栏那一截跟标题栏连成一片；内容区的灰底在 scroll 上
            setBackgroundColor(Color.WHITE)
            addView(titleBar())
            addView(scroll, LinearLayout.LayoutParams(-1, 0, 1f))
            // 画到状态栏 / 导航条底下（下拉面板要铺满整屏），自己让出来
            setOnApplyWindowInsetsListener { v, insets ->
                @Suppress("DEPRECATION")
                v.setPadding(0, insets.systemWindowInsetTop, 0, insets.systemWindowInsetBottom)
                insets
            }
        }
        panel = StorePanel(this, store) { SearchActivity.start(this, store.baseUrl) }
        drawer = PullDrawerLayout(
            this, panel, page,
            contentAtTop = { !scroll.canScrollVertically(-1) },
            panelAtBottom = { !panel.scroll.canScrollVertically(1) },
        )
        drawer.onStateChanged = ::lightStatusBar
        drawEdgeToEdge()
        setContentView(drawer)
        StoreLocal.listeners += onLocal
        refresh()
    }

    override fun onDestroy() {
        StoreLocal.listeners -= onLocal
        super.onDestroy()
    }

    private fun dp(v: Float) = StoreViews.dp(this, v)

    override fun onResume() {
        super.onResume()
        showRecent()
        panel.refresh()
    }

    @Deprecated("Activity.onBackPressed：minSdk 24 上没有 OnBackInvokedCallback")
    override fun onBackPressed() {
        if (drawer.isOpen) drawer.close() else super.onBackPressed()
    }

    /** 状态栏透明、内容画到它底下（targetSdk 35 在 Android 15 上本来就强制；老系统上自己设） */
    @Suppress("DEPRECATION")
    private fun drawEdgeToEdge() {
        window.statusBarColor = Color.TRANSPARENT
        window.navigationBarColor = PAGE_BG
        window.decorView.systemUiVisibility = View.SYSTEM_UI_FLAG_LAYOUT_STABLE or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
        lightStatusBar(false)
    }

    /** 面板是深色的：展开时状态栏图标换白 */
    @Suppress("DEPRECATION")
    private fun lightStatusBar(open: Boolean) {
        val v = window.decorView
        v.systemUiVisibility = if (open) v.systemUiVisibility and View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR.inv()
        else v.systemUiVisibility or View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR
    }

    private fun titleBar() = LinearLayout(this).apply {
        gravity = Gravity.CENTER_VERTICAL
        setBackgroundColor(Color.WHITE)
        setPadding(dp(16f), 0, dp(4f), 0)
        addView(LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            addView(TextView(context).apply { text = "灵犀原生"; textSize = 20f; typeface = Typeface.DEFAULT_BOLD; setTextColor(Color.BLACK) })
            addView(TextView(context).apply { text = "小程序引擎演示 · 下拉打开小程序面板"; textSize = 11f; setTextColor(0x61000000) })
        }, LinearLayout.LayoutParams(0, -2, 1f))
        addView(TextView(context).apply {
            text = "⊞"
            textSize = 22f
            gravity = Gravity.CENTER
            setTextColor(0xDE000000.toInt())
            contentDescription = "小程序面板"
            setBackgroundResource(android.R.drawable.list_selector_background)
            setOnClickListener { drawer.open() }
        }, LinearLayout.LayoutParams(dp(48f), dp(48f)))
        layoutParams = LinearLayout.LayoutParams(-1, dp(56f))
    }

    /** 白底圆角卡片：标题行（可带右边的提示 / 按钮）+ 往里加的内容 */
    private fun card(title: String, hint: String = "", action: String? = null, onAction: (() -> Unit)? = null) =
        LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, 0, 0, dp(12f))
            background = GradientDrawable().apply { cornerRadius = dp(14f).toFloat(); setColor(Color.WHITE) }
            clipToOutline = true
            addView(StoreWidgets.header(context, title, action ?: hint.ifEmpty { null }, onAction).apply {
                setPadding(dp(16f), dp(14f), dp(8f), dp(6f))
            })
            layoutParams = LinearLayout.LayoutParams(-1, -2).apply { setMargins(dp(12f), 0, dp(12f), dp(12f)) }
        }

    /** 白底圆角的假搜索框：点了进搜索页（微信也是这样，入口页上不直接输入） */
    private fun searchBar() = LinearLayout(this).apply {
        gravity = Gravity.CENTER
        isClickable = true
        contentDescription = "搜索小程序"
        background = GradientDrawable().apply { cornerRadius = dp(10f).toFloat(); setColor(Color.WHITE) }
        addView(TextView(context).apply { text = "🔍  搜索小程序"; textSize = 15f; setTextColor(0x61000000) })
        setOnClickListener { SearchActivity.start(this@StoreActivity, store.baseUrl) }
        layoutParams = LinearLayout.LayoutParams(-1, dp(40f)).apply { setMargins(dp(12f), dp(12f), dp(12f), dp(12f)) }
    }

    private fun showRecent() {
        if (!::recentRow.isInitialized) return
        val recent = StoreLocal(this).recent()
        recentBox.visibility = if (recent.isEmpty()) View.GONE else View.VISIBLE
        recentRow.removeAllViews()
        recent.forEach { recentRow.addView(cell(it), LinearLayout.LayoutParams(cellWidth(), -2)) }
    }

    private fun cellWidth() = (resources.displayMetrics.widthPixels - dp(24f) - dp(16f)) / 4

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
                    l.offline -> "⚠ 服务器连不上，显示上次的列表（${l.error}）"
                    l.apps.isEmpty() -> "服务器上还没有小程序"
                    else -> "来自 ${store.baseUrl} · ${l.apps.size} 个"
                }
                Log.i(MainActivity.TAG, "[商店] 列表 ${l.apps.size} 个 offline=${l.offline}")
                apps = l.apps
                showCategories(l.apps)
                showGrid()
            }
        }
    }

    private fun showGrid() {
        if (!::grid.isInitialized) return
        grid.removeAllViews()
        val fav = StoreLocal(this).favorites().map { it.id }.toSet()
        apps.forEach { grid.addView(cell(it, it.id in fav), GridLayout.LayoutParams().apply { width = cellWidth() }) }
    }

    private fun cell(app: MiniStore.App, fav: Boolean = false) = LinearLayout(this).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_HORIZONTAL
        setPadding(0, dp(10f), 0, dp(10f))
        isClickable = true
        isFocusable = true
        contentDescription = "打开小程序 ${app.name}${if (fav) "（已收藏）" else ""}，长按更多操作"
        setBackgroundResource(android.R.drawable.list_selector_background)
        val icon = StoreIcons.view(store, app, 50f)
        // 收藏过的：图标右上角一颗星（白底圆，压在图标边上）
        if (fav) icon.addView(TextView(context).apply {
            text = "★"
            textSize = 11f
            gravity = Gravity.CENTER
            setTextColor(0xFFFFB400.toInt())
            background = GradientDrawable().apply { shape = GradientDrawable.OVAL; setColor(Color.WHITE) }
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        }, FrameLayout.LayoutParams(dp(17f), dp(17f), Gravity.TOP or Gravity.END))
        addView(icon)
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
        setOnLongClickListener { StoreSheets.appActions(this@StoreActivity, store, app); true }
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

    companion object {
        private const val PAGE_BG = 0xFFF2F3F5.toInt()
    }
}
