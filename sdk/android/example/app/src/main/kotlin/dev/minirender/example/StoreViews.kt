package dev.minirender.example

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.Button
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView

/** 商店界面的小零件：圆形图标、微信式 loading 层。与 Flutter 示例 lib/store/store_pages.dart 同一套样子 */
object StoreViews {
    private val COLORS = intArrayOf(
        0xFF07C160.toInt(), 0xFF10AEFF.toInt(), 0xFFFA9D3B.toInt(), 0xFFE64340.toInt(),
        0xFF6467F0.toInt(), 0xFF1485EE.toInt(), 0xFF91D300.toInt(), 0xFF576B95.toInt(),
    )
    const val GREEN = 0xFF07C160.toInt()

    fun dp(ctx: Context, v: Float) = (v * ctx.resources.displayMetrics.density).toInt()

    /** 底色按 id 算（与管理页 admin.js、Flutter 示例同一个哈希，三处颜色一样） */
    private fun color(id: String): Int {
        var h = 0L
        id.codePoints().forEach { h = (h * 31 + it) and 0xFFFFFFFFL }
        return COLORS[(h % COLORS.size).toInt()]
    }

    /** 圆形图标：名字的第一个字 */
    fun icon(ctx: Context, id: String, name: String, sizeDp: Float): TextView = TextView(ctx).apply {
        val px = dp(ctx, sizeDp)
        layoutParams = LinearLayout.LayoutParams(px, px)
        gravity = Gravity.CENTER
        text = name.takeIf { it.isNotEmpty() }?.let { String(Character.toChars(it.codePointAt(0))) } ?: "?"
        setTextColor(Color.WHITE)
        textSize = sizeDp * 0.42f
        typeface = Typeface.DEFAULT_BOLD
        background = GradientDrawable().apply { shape = GradientDrawable.OVAL; setColor(color(id)) }
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }

    /**
     * 点开小程序时盖在上面的一层：图标 + 名字 + 三个依次变深的点 + 下载进度。
     * [hide] 淡出后把自己从父视图拿掉、停掉动画（留着的话每帧都在 UI 线程上重绘）
     */
    class LoadingCover(ctx: Context, id: String, name: String, onClose: () -> Unit) : FrameLayout(ctx) {
        // 包一层 WRAP_CONTENT：竖排 LinearLayout 默认给子视图满宽，图标就贴到左边去了
        private val iconSlot = FrameLayout(ctx).apply {
            layoutParams = LinearLayout.LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.WRAP_CONTENT)
            addView(icon(ctx, id, name, 64f))
        }
        private val title = TextView(ctx).apply {
            text = name
            // 竖排 LinearLayout 给子视图满宽：不居中的话名字贴左
            gravity = Gravity.CENTER_HORIZONTAL
            textSize = 17f
            setTextColor(Color.BLACK)
            setPadding(0, dp(ctx, 14f), 0, dp(ctx, 28f))
        }
        private val dots = List(3) {
            View(ctx).apply {
                layoutParams = LinearLayout.LayoutParams(dp(ctx, 8f), dp(ctx, 8f)).apply {
                    marginStart = dp(ctx, 4f); marginEnd = dp(ctx, 4f)
                }
                background = GradientDrawable().apply { shape = GradientDrawable.OVAL; setColor(0xFFD8D8D8.toInt()) }
            }
        }
        private val dotRow = LinearLayout(ctx).apply {
            gravity = Gravity.CENTER
            contentDescription = "加载中"
            dots.forEach { addView(it) }
        }
        private val status = TextView(ctx).apply {
            gravity = Gravity.CENTER
            textSize = 12f
            setTextColor(0x61000000)
            setPadding(dp(ctx, 32f), dp(ctx, 12f), dp(ctx, 32f), 0)
        }
        private val retry = Button(ctx).apply { text = "重试"; visibility = GONE }
        private val anim = ValueAnimator.ofInt(0, 3).apply {
            duration = 900
            repeatCount = ValueAnimator.INFINITE
            addUpdateListener { a ->
                val on = (a.animatedValue as Int).coerceAtMost(2)
                dots.forEachIndexed { i, v ->
                    (v.background as GradientDrawable).setColor(if (i == on) GREEN else 0xFFD8D8D8.toInt())
                }
            }
        }

        init {
            setBackgroundColor(Color.WHITE)
            isClickable = true   // 盖着的时候吃掉触摸，别漏到底下还没起来的小程序
            val col = LinearLayout(ctx).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_HORIZONTAL
                addView(iconSlot)
                addView(title)
                addView(dotRow)
                addView(status)
                addView(retry)
            }
            addView(col, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.WRAP_CONTENT, Gravity.CENTER).apply {
                bottomMargin = dp(ctx, 120f)
            })
            addView(TextView(ctx).apply {
                text = "✕"
                textSize = 20f
                setTextColor(0x8A000000.toInt())
                contentDescription = "关闭"
                setPadding(dp(ctx, 16f), dp(ctx, 12f), dp(ctx, 16f), dp(ctx, 12f))
                setOnClickListener { onClose() }
            }, LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.WRAP_CONTENT, Gravity.TOP or Gravity.END).apply {
                topMargin = dp(ctx, 36f)
            })
            anim.start()
        }

        fun progress(text: String) {
            status.text = text
        }

        /** 深链进来时先只知道 AppID：服务器回了最新版信息后换上真名字与图标 */
        fun identity(id: String, name: String) {
            title.text = name
            iconSlot.removeAllViews()
            iconSlot.addView(icon(context, id, name, 64f))
        }

        fun error(text: String, onRetry: () -> Unit) {
            anim.pause()
            dotRow.visibility = GONE
            status.setTextColor(0xFFE64340.toInt())
            status.text = text
            retry.visibility = VISIBLE
            retry.setOnClickListener {
                retry.visibility = GONE
                dotRow.visibility = VISIBLE
                status.setTextColor(0x61000000)
                status.text = ""
                anim.resume()
                onRetry()
            }
        }

        fun hide() {
            isClickable = false
            animate().alpha(0f).setDuration(200).withEndAction {
                anim.cancel()
                (parent as? android.view.ViewGroup)?.removeView(this)
            }.start()
        }

        override fun onDetachedFromWindow() {
            anim.cancel()
            super.onDetachedFromWindow()
        }
    }
}
