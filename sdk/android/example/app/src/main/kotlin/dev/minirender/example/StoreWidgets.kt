package dev.minirender.example

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.TextView

/** 商店入口页与搜索页共用的零件：自动换行的标签行、标签、小程序结果行、打开小程序 */
object StoreWidgets {
    const val GRAY_BG = 0xFFF7F7F7.toInt()
    const val SUB = 0x8A000000.toInt()

    /** 打开一个商店小程序：记进「最近使用」，起 [MainActivity]（微信是从下往上推出来的） */
    fun open(a: Activity, store: MiniStore, app: MiniStore.App) {
        StoreLocal(a).touch(app)
        a.startActivity(Intent(a, MainActivity::class.java)
            .putExtra(MainActivity.EXTRA_STORE_APP, app.toJson())
            .putExtra(MainActivity.EXTRA_STORE_URL, store.baseUrl))
        @Suppress("DEPRECATION")
        a.overridePendingTransition(R.anim.store_slide_up, 0)
    }

    fun chip(ctx: Context, text: String, selected: Boolean = false, onClick: () -> Unit) = TextView(ctx).apply {
        this.text = text
        textSize = 13f
        maxLines = 1
        ellipsize = TextUtils.TruncateAt.END
        setTextColor(if (selected) StoreViews.GREEN else 0xDE000000.toInt())
        setPadding(dp(ctx, 12f), dp(ctx, 6f), dp(ctx, 12f), dp(ctx, 6f))
        background = GradientDrawable().apply {
            cornerRadius = dp(ctx, 16f).toFloat()
            setColor(if (selected) 0x1A07C160 else GRAY_BG)
        }
        setOnClickListener { onClick() }
    }

    fun header(ctx: Context, title: String, action: String? = null, onAction: (() -> Unit)? = null) = LinearLayout(ctx).apply {
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(ctx, 16f), dp(ctx, 18f), dp(ctx, 8f), dp(ctx, 6f))
        addView(TextView(ctx).apply {
            text = title
            textSize = 14f
            typeface = Typeface.DEFAULT_BOLD
            setTextColor(Color.BLACK)
        }, LinearLayout.LayoutParams(0, -2, 1f))
        if (action != null) addView(TextView(ctx).apply {
            text = action
            textSize = 13f
            setTextColor(SUB)
            setPadding(dp(ctx, 8f), dp(ctx, 4f), dp(ctx, 8f), dp(ctx, 4f))
            setOnClickListener { onAction?.invoke() }
        })
    }

    /** 搜索结果的一行：图标、名称（命中的字标绿）、分类 · 开发者、简介 */
    fun resultRow(a: Activity, store: MiniStore, app: MiniStore.App, q: String) = LinearLayout(a).apply {
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(a, 16f), dp(a, 12f), dp(a, 16f), dp(a, 12f))
        isClickable = true
        isFocusable = true
        contentDescription = "打开小程序 ${app.name}"
        setBackgroundResource(android.R.drawable.list_selector_background)
        addView(StoreIcons.view(store, app, 48f))
        val col = LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(a, 12f), 0, 0, 0)
        }
        col.addView(TextView(a).apply {
            text = highlight(app.name, q)
            textSize = 16f
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setTextColor(Color.BLACK)
        })
        col.addView(TextView(a).apply {
            text = listOf(app.category, app.developer).filter { it.isNotEmpty() }.joinToString(" · ")
            textSize = 12f
            setTextColor(SUB)
            visibility = if (text.isEmpty()) View.GONE else View.VISIBLE
        })
        col.addView(TextView(a).apply {
            text = app.description
            textSize = 13f
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setTextColor(0x99000000.toInt())
            visibility = if (app.description.isEmpty()) View.GONE else View.VISIBLE
        })
        addView(col, LinearLayout.LayoutParams(0, -2, 1f))
        setOnClickListener { open(a, store, app) }
    }

    /** 名称里出现的关键词标绿（微信搜索结果就是这样）；每段关键词各标一次 */
    fun highlight(text: String, q: String): CharSequence {
        val sp = android.text.SpannableString(text)
        q.split(' ').filter { it.isNotBlank() }.forEach { t ->
            val i = text.indexOf(t, ignoreCase = true)
            if (i >= 0) sp.setSpan(android.text.style.ForegroundColorSpan(StoreViews.GREEN), i, i + t.length, 0)
        }
        return sp
    }

    private fun dp(ctx: Context, v: Float) = StoreViews.dp(ctx, v)
}

/** 自动换行的一行行标签（搜索历史、热搜、分类）。子视图按自己的宽度排，放不下就折到下一行 */
class FlowLayout(ctx: Context, private val gap: Int = StoreViews.dp(ctx, 8f)) : ViewGroup(ctx) {
    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val maxW = MeasureSpec.getSize(widthMeasureSpec) - paddingLeft - paddingRight
        var x = 0
        var y = 0
        var lineH = 0
        for (i in 0 until childCount) {
            val c = getChildAt(i)
            c.measure(MeasureSpec.makeMeasureSpec(maxW, MeasureSpec.AT_MOST), MeasureSpec.UNSPECIFIED)
            if (x > 0 && x + c.measuredWidth > maxW) {
                x = 0
                y += lineH + gap
                lineH = 0
            }
            x += c.measuredWidth + gap
            lineH = maxOf(lineH, c.measuredHeight)
        }
        setMeasuredDimension(MeasureSpec.getSize(widthMeasureSpec), y + lineH + paddingTop + paddingBottom)
    }

    override fun onLayout(changed: Boolean, l: Int, t: Int, r: Int, b: Int) {
        val maxW = r - l - paddingLeft - paddingRight
        var x = 0
        var y = 0
        var lineH = 0
        for (i in 0 until childCount) {
            val c = getChildAt(i)
            if (x > 0 && x + c.measuredWidth > maxW) {
                x = 0
                y += lineH + gap
                lineH = 0
            }
            c.layout(paddingLeft + x, paddingTop + y, paddingLeft + x + c.measuredWidth, paddingTop + y + c.measuredHeight)
            x += c.measuredWidth + gap
            lineH = maxOf(lineH, c.measuredHeight)
        }
    }
}
