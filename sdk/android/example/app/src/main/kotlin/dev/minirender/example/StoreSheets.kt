package dev.minirender.example

import android.app.Activity
import android.app.Dialog
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.net.Uri
import android.text.TextUtils
import android.view.Gravity
import android.view.HapticFeedbackConstants
import android.view.View
import android.view.ViewGroup
import android.view.Window
import android.view.WindowManager
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast

/**
 * 两个底部弹层，照微信的样子（与 Flutter 示例 lib/store/store_sheets.dart 同一套）：
 * - [appActions]：长按一个小程序 —— 添加到 / 移出「我的小程序」、从「最近使用」删除、打开；
 * - [capsuleMenu]：小程序里胶囊的「•••」—— 顶上小程序名（点了看关于），两排圆角方块，最下面隔开一条「取消」。
 * 不依赖 AndroidX（示例只用框架自带的类）：底部弹层就是一个贴底、从下滑入的 [Dialog]。
 */
object StoreSheets {
    private const val SHEET_BG = 0xFFF7F7F7.toInt()
    private const val DANGER = 0xFFFA5151.toInt()
    private const val GOLD = 0xFFFFB400.toInt()

    /** 胶囊菜单的一格。[glyph] 是一个字符图标（框架里没有现成的矢量图标，不为示例拉 material 依赖） */
    class Item(val glyph: String, val label: String, val enabled: Boolean = true, val active: Boolean = false, val onClick: () -> Unit)

    fun appActions(a: Activity, store: MiniStore, app: MiniStore.App, scene: Int = 1001) {
        val local = StoreLocal(a)
        val fav = local.isFavorite(app.id)
        val inRecent = local.recent().any { it.id == app.id }
        val col = column(a)
        val d = bottomDialog(a, col)
        col.addView(LinearLayout(a).apply {
            gravity = Gravity.CENTER_VERTICAL
            setBackgroundColor(Color.WHITE)
            setPadding(dp(a, 20f), dp(a, 18f), dp(a, 20f), dp(a, 16f))
            addView(StoreIcons.view(store, app, 44f))
            addView(LinearLayout(a).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(dp(a, 12f), 0, 0, 0)
                addView(text(a, app.name, 16f, Color.BLACK, bold = true))
                val sub = app.description.ifEmpty { app.developer }
                if (sub.isNotEmpty()) addView(text(a, sub, 12f, 0x73000000).apply { setPadding(0, dp(a, 3f), 0, 0) })
            }, LinearLayout.LayoutParams(0, -2, 1f))
        })
        col.addView(divider(a))
        col.addView(option(a, if (fav) "从「我的小程序」移除" else "添加到「我的小程序」") {
            d.dismiss()
            val now = local.toggleFavorite(app)
            toast(a, if (now) "已添加到「我的小程序」，下拉入口页可见" else "已从「我的小程序」移除")
        })
        if (inRecent) {
            col.addView(divider(a))
            col.addView(option(a, "从「最近使用」删除", DANGER) {
                d.dismiss()
                local.removeRecent(app.id)
                toast(a, "已从「最近使用」删除")
            })
        }
        col.addView(divider(a))
        col.addView(option(a, "打开") { d.dismiss(); StoreWidgets.open(a, store, app, scene) })
        col.addView(View(a), LinearLayout.LayoutParams(-1, dp(a, 8f)))
        col.addView(option(a, "取消") { d.dismiss() })
        a.window.decorView.performHapticFeedback(HapticFeedbackConstants.LONG_PRESS)
        d.show()
    }

    fun capsuleMenu(a: Activity, icon: View, name: String, subtitle: String, onHeader: () -> Unit, top: List<Item>, bottom: List<Item>) {
        val col = column(a)
        val d = bottomDialog(a, col)
        col.addView(LinearLayout(a).apply {
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(a, 20f), dp(a, 16f), dp(a, 16f), dp(a, 14f))
            isClickable = true
            contentDescription = "关于$name"
            setBackgroundResource(android.R.drawable.list_selector_background)
            addView(icon)
            addView(text(a, name, 15f, Color.BLACK, bold = true).apply { setPadding(dp(a, 10f), 0, dp(a, 2f), 0) })
            addView(text(a, "›", 18f, 0x61000000), LinearLayout.LayoutParams(0, -2, 1f))
            addView(text(a, subtitle, 12f, 0x61000000))
            setOnClickListener { d.dismiss(); onHeader() }
        })
        col.addView(divider(a, inset = 20f))
        col.addView(row(a, d, top))
        col.addView(divider(a, inset = 20f))
        col.addView(row(a, d, bottom))
        col.addView(View(a), LinearLayout.LayoutParams(-1, dp(a, 8f)))
        col.addView(option(a, "取消") { d.dismiss() })
        d.show()
    }

    /** 转发：微信是发给好友；示例复制一条能打开它的链接（外部 App / 浏览器里点开即进小程序） */
    fun copyLink(a: Activity, app: MiniStore.App, route: String?) {
        val b = Uri.Builder().appendQueryParameter("appid", app.key)
        if (!route.isNullOrEmpty()) b.appendQueryParameter("path", route)
        val link = "minirender://open?" + b.build().encodedQuery
        (a.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager).setPrimaryClip(ClipData.newPlainText(app.name, link))
        toast(a, "链接已复制：在别的 App 或浏览器里打开即可进入「${app.name}」")
    }

    fun toast(a: Activity, s: String) = Toast.makeText(a, s, Toast.LENGTH_SHORT).show()

    private fun row(a: Activity, d: Dialog, items: List<Item>) = HorizontalScrollView(a).apply {
        isHorizontalScrollBarEnabled = false
        addView(LinearLayout(a).apply {
            setPadding(dp(a, 12f), dp(a, 16f), dp(a, 12f), dp(a, 14f))
            items.forEach { addView(cell(a, d, it)) }
        })
    }

    private fun cell(a: Activity, d: Dialog, it: Item) = LinearLayout(a).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_HORIZONTAL
        isEnabled = it.enabled
        contentDescription = it.label.replace("\n", "")
        addView(TextView(a).apply {
            text = it.glyph
            gravity = Gravity.CENTER
            textSize = 22f
            setTextColor(if (!it.enabled) 0x42000000 else if (it.active) GOLD else 0xDE000000.toInt())
            background = GradientDrawable().apply { cornerRadius = dp(a, 12f).toFloat(); setColor(Color.WHITE) }
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        }, LinearLayout.LayoutParams(dp(a, 52f), dp(a, 52f)))
        addView(text(a, it.label, 11f, if (it.enabled) 0x8A000000.toInt() else 0x42000000).apply {
            gravity = Gravity.CENTER
            maxLines = 2
            setLineSpacing(0f, 1.15f)
            setPadding(0, dp(a, 8f), 0, 0)
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        })
        layoutParams = LinearLayout.LayoutParams(dp(a, 76f), -2)
        if (it.enabled) setOnClickListener { _ -> d.dismiss(); it.onClick() }
    }

    private fun column(a: Activity) = LinearLayout(a).apply {
        orientation = LinearLayout.VERTICAL
        background = GradientDrawable().apply {
            setColor(SHEET_BG)
            val r = dp(a, 12f).toFloat()
            cornerRadii = floatArrayOf(r, r, r, r, 0f, 0f, 0f, 0f)
        }
        clipToOutline = true
        // 强制 edge-to-edge 的系统上弹层也铺到导航条底下：让出它
        setOnApplyWindowInsetsListener { v, insets ->
            @Suppress("DEPRECATION")
            v.setPadding(0, 0, 0, insets.systemWindowInsetBottom)
            insets
        }
    }

    private fun bottomDialog(a: Activity, content: View) = Dialog(a).apply {
        requestWindowFeature(Window.FEATURE_NO_TITLE)
        setContentView(content)
        window?.apply {
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            setGravity(Gravity.BOTTOM)
            setLayout(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT)
            // 输入法的进出动画就是「从底下滑上来 / 滑下去」
            setWindowAnimations(android.R.style.Animation_InputMethod)
            addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND)
            setDimAmount(0.45f)
            decorView.setPadding(0, 0, 0, 0)
        }
    }

    private fun option(a: Activity, label: String, color: Int = 0xDE000000.toInt(), onClick: () -> Unit) =
        text(a, label, 17f, color).apply {
            gravity = Gravity.CENTER
            setBackgroundColor(Color.WHITE)
            foreground = a.getDrawable(android.R.drawable.list_selector_background)
            minHeight = dp(a, 56f)
            isClickable = true
            setOnClickListener { onClick() }
        }

    private fun divider(a: Activity, inset: Float = 0f) = View(a).apply {
        setBackgroundColor(0x1F000000)
        layoutParams = LinearLayout.LayoutParams(-1, maxOf(1, dp(a, 0.5f))).apply {
            marginStart = dp(a, inset); marginEnd = dp(a, inset)
        }
    }

    private fun text(ctx: Context, s: String, size: Float, color: Int, bold: Boolean = false) = TextView(ctx).apply {
        text = s
        textSize = size
        setTextColor(color)
        maxLines = 1
        ellipsize = TextUtils.TruncateAt.END
        if (bold) typeface = Typeface.DEFAULT_BOLD
    }

    private fun dp(ctx: Context, v: Float) = StoreViews.dp(ctx, v)
}
