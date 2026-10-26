package dev.minirender.example

import android.app.Activity
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.GridLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView

/**
 * 下拉出来的深色小程序面板（微信聊天页往下拉的那一屏）：搜索框、「最近使用」、「我的小程序」。
 * 点图标打开，长按弹操作单（[StoreSheets.appActions]）。数据都是本机记的，[refresh] 重画。
 * 与 Flutter 示例 lib/store/store_panel.dart 同一套布局与场景值。
 */
class StorePanel(private val a: Activity, private val store: MiniStore, onSearch: () -> Unit) : FrameLayout(a) {
    /** 面板里的滚动区（[PullDrawerLayout] 要知道它是不是滚到底了） */
    val scroll = ScrollView(a).apply { isVerticalScrollBarEnabled = false; isFillViewport = true }
    private val body = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }

    init {
        background = GradientDrawable(
            GradientDrawable.Orientation.TOP_BOTTOM,
            intArrayOf(0xFF3B3B4F.toInt(), 0xFF262633.toInt(), 0xFF1C1C25.toInt()),
        )
        val col = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }
        col.addView(TextView(a).apply {
            text = "小程序"
            textSize = 17f
            typeface = Typeface.DEFAULT_BOLD
            setTextColor(Color.WHITE)
            gravity = Gravity.CENTER
        }, LinearLayout.LayoutParams(-1, dp(48f)))
        col.addView(TextView(a).apply {
            text = "🔍  搜索小程序"
            textSize = 15f
            gravity = Gravity.CENTER
            setTextColor(0x8AFFFFFF.toInt())
            isClickable = true
            contentDescription = "搜索小程序"
            background = GradientDrawable().apply { cornerRadius = dp(10f).toFloat(); setColor(0x1AFFFFFF) }
            setOnClickListener { onSearch() }
        }, LinearLayout.LayoutParams(-1, dp(38f)).apply { setMargins(dp(16f), 0, dp(16f), 0) })
        scroll.addView(body)
        col.addView(scroll, LinearLayout.LayoutParams(-1, 0, 1f))
        addView(col)
        // 顶上让出状态栏；底下让出首页露出来的那一截（PullDrawerLayout.PEEK_DP）+ 导航条
        setOnApplyWindowInsetsListener { _, insets ->
            @Suppress("DEPRECATION")
            col.setPadding(0, insets.systemWindowInsetTop, 0, 0)
            @Suppress("DEPRECATION")
            body.setPadding(0, 0, 0, dp(PullDrawerLayout.PEEK_DP + 24f) + insets.systemWindowInsetBottom)
            insets
        }
        refresh()
    }

    fun refresh() {
        val local = StoreLocal(a)
        val recent = local.recent()
        val fav = local.favorites()
        body.removeAllViews()
        body.addView(header("最近使用", if (recent.isEmpty()) "" else "长按可删除"))
        body.addView(if (recent.isEmpty()) empty("打开过的小程序会出现在这里") else grid(recent, SCENE_RECENT))
        body.addView(header("我的小程序", if (fav.isEmpty()) "" else "${fav.size} 个"))
        body.addView(
            if (fav.isEmpty()) empty("长按任意小程序，选「添加到我的小程序」\n或在小程序里点右上角 ••• 收藏")
            else grid(fav, SCENE_FAVORITE),
        )
        body.addView(TextView(a).apply {
            text = "上推或点下方返回"
            textSize = 12f
            gravity = Gravity.CENTER
            setTextColor(0x3DFFFFFF)
            setPadding(0, dp(28f), 0, 0)
        })
    }

    private fun header(title: String, hint: String) = LinearLayout(a).apply {
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(20f), dp(22f), dp(20f), dp(10f))
        addView(TextView(a).apply {
            text = title
            textSize = 14f
            typeface = Typeface.DEFAULT_BOLD
            setTextColor(0xB3FFFFFF.toInt())
        }, LinearLayout.LayoutParams(0, -2, 1f))
        addView(TextView(a).apply { text = hint; textSize = 12f; setTextColor(0x4DFFFFFF) })
    }

    private fun empty(s: String) = TextView(a).apply {
        text = s
        textSize = 13f
        gravity = Gravity.CENTER
        setLineSpacing(0f, 1.3f)
        setTextColor(0x61FFFFFF)
        setPadding(dp(16f), dp(22f), dp(16f), dp(22f))
        background = GradientDrawable().apply {
            cornerRadius = dp(12f).toFloat()
            setColor(0x0DFFFFFF)
            setStroke(maxOf(1, dp(1f)), 0x1FFFFFFF)
        }
        layoutParams = LinearLayout.LayoutParams(-1, -2).apply { setMargins(dp(16f), 0, dp(16f), 0) }
    }

    private fun grid(apps: List<MiniStore.App>, scene: Int) = GridLayout(a).apply {
        columnCount = 4
        setPadding(dp(8f), 0, dp(8f), 0)
        val w = (a.resources.displayMetrics.widthPixels - dp(16f)) / 4
        apps.forEach { addView(cell(it, scene), GridLayout.LayoutParams().apply { width = w }) }
    }

    private fun cell(app: MiniStore.App, scene: Int) = LinearLayout(a).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_HORIZONTAL
        setPadding(0, dp(10f), 0, dp(10f))
        isClickable = true
        isFocusable = true
        contentDescription = "打开小程序 ${app.name}，长按更多操作"
        setBackgroundResource(android.R.drawable.list_selector_background)
        addView(StoreIcons.view(store, app, 50f))
        addView(TextView(a).apply {
            text = app.name
            textSize = 12f
            maxLines = 1
            gravity = Gravity.CENTER_HORIZONTAL
            ellipsize = TextUtils.TruncateAt.END
            setTextColor(0xB3FFFFFF.toInt())
            setPadding(dp(4f), dp(8f), dp(4f), 0)
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        })
        setOnClickListener { StoreWidgets.open(a, store, app, scene) }
        setOnLongClickListener { StoreSheets.appActions(a, store, app, scene); true }
    }

    private fun dp(v: Float) = StoreViews.dp(a, v)

    companion object {
        /** 微信的场景值：1089 下拉「最近使用」、1104 下拉「我的小程序」 */
        const val SCENE_RECENT = 1089
        const val SCENE_FAVORITE = 1104
    }
}
