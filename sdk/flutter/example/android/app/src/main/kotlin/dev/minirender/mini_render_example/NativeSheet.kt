package dev.minirender.mini_render_example

import android.app.Activity
import android.app.AlertDialog
import android.app.Dialog
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.Outline
import android.graphics.Typeface
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.ViewOutlineProvider
import android.view.Window
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.HorizontalScrollView
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.net.HttpURLConnection
import java.net.URL

/**
 * 小程序页上的弹层（胶囊「•••」菜单、关于框）用**原生窗口**画（`mini_render_example/sheet` 通道，Dart 侧 lib/native_sheet.dart）。
 *
 * 为什么不用 Flutter 的 showModalBottomSheet：小程序画面是 hybrid composition 的平台视图（引擎自己的 SurfaceView），
 * Flutter 在它上面每画一帧都要经叠加层、和平台线程同步 —— 一加 PLY110 上量到弹层动画每帧 22~42ms（90Hz 下一帧该 11ms），
 * 打开菜单那 250ms 只出七八帧，看着就是「点了半天才慢慢出来」。原生 Dialog 是单独一个窗口，由系统合成，
 * Flutter 一帧都不用画。样子与 lib/store/store_sheets.dart、原生示例的 StoreSheets.kt 同一套。
 */
class NativeSheet(private val a: Activity) : MethodChannel.MethodCallHandler {
    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "capsule" -> capsule(call, result)
            "alert" -> alert(call, result)
            "toast" -> {
                android.widget.Toast.makeText(a, call.argument<String>("text") ?: "", android.widget.Toast.LENGTH_SHORT).show()
                result.success(null)
            }
            else -> result.notImplemented()
        }
    }

    /** 回一次就够（点了某项 / 取消 / 点外面收起都会走到这里） */
    private class Once(val r: MethodChannel.Result) {
        var done = false
        fun reply(v: Any?) {
            if (!done) {
                done = true
                r.success(v)
            }
        }
    }

    private fun alert(call: MethodCall, result: MethodChannel.Result) {
        val once = Once(result)
        val buttons = call.argument<List<String>>("buttons") ?: listOf("确定")
        val b = AlertDialog.Builder(a)
            .setTitle(call.argument<String>("title") ?: "")
            .setMessage(call.argument<String>("message") ?: "")
            .setOnDismissListener { once.reply(null) }
        // 最多两个：最后一个是主按钮（右边），前一个放中间
        buttons.getOrNull(buttons.size - 1)?.let { t -> b.setPositiveButton(t) { _, _ -> once.reply(buttons.size - 1) } }
        if (buttons.size >= 2) b.setNeutralButton(buttons[buttons.size - 2]) { _, _ -> once.reply(buttons.size - 2) }
        b.show()
    }

    @Suppress("UNCHECKED_CAST")
    private fun capsule(call: MethodCall, result: MethodChannel.Result) {
        val once = Once(result)
        val col = column()
        val d = bottomDialog(col)
        d.setOnDismissListener { once.reply(null) }
        fun pick(id: String?) {
            once.reply(id)
            d.dismiss()
        }
        val name = call.argument<String>("name") ?: ""
        col.addView(LinearLayout(a).apply {
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(20f), dp(16f), dp(16f), dp(14f))
            isClickable = true
            contentDescription = "关于$name"
            setBackgroundResource(android.R.drawable.list_selector_background)
            addView(icon(call.argument<String>("id") ?: "", name, call.argument<String>("iconUrl") ?: "", 30f))
            addView(text(name, 15f, Color.BLACK, bold = true).apply { setPadding(dp(10f), 0, dp(2f), 0) })
            addView(text("›", 18f, 0x61000000), LinearLayout.LayoutParams(0, -2, 1f))
            addView(text(call.argument<String>("subtitle") ?: "", 12f, 0x61000000))
            setOnClickListener { pick("header") }
        })
        for (key in listOf("top", "bottom")) {
            col.addView(divider())
            val items = call.argument<List<Map<String, Any?>>>(key) ?: emptyList()
            col.addView(HorizontalScrollView(a).apply {
                isHorizontalScrollBarEnabled = false
                addView(LinearLayout(a).apply {
                    setPadding(dp(12f), dp(16f), dp(12f), dp(14f))
                    items.forEach { addView(cell(it) { id -> pick(id) }) }
                })
            })
        }
        col.addView(View(a), LinearLayout.LayoutParams(-1, dp(8f)))
        col.addView(text("取消", 17f, 0xDE000000.toInt()).apply {
            gravity = Gravity.CENTER
            setBackgroundColor(Color.WHITE)
            foreground = a.getDrawable(android.R.drawable.list_selector_background)
            minHeight = dp(56f)
            isClickable = true
            setOnClickListener { pick(null) }
        })
        d.show()
    }

    private fun cell(it: Map<String, Any?>, onPick: (String) -> Unit) = LinearLayout(a).apply {
        val enabled = it["enabled"] as? Boolean ?: true
        val active = it["active"] as? Boolean ?: false
        val label = it["label"] as? String ?: ""
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_HORIZONTAL
        isEnabled = enabled
        contentDescription = label.replace("\n", "")
        addView(TextView(a).apply {
            text = it["glyph"] as? String ?: "•"
            gravity = Gravity.CENTER
            textSize = 22f
            setTextColor(if (!enabled) 0x42000000 else if (active) 0xFFFFB400.toInt() else 0xDE000000.toInt())
            background = GradientDrawable().apply { cornerRadius = dp(12f).toFloat(); setColor(Color.WHITE) }
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        }, LinearLayout.LayoutParams(dp(52f), dp(52f)))
        addView(text(label, 11f, if (enabled) 0x8A000000.toInt() else 0x42000000).apply {
            gravity = Gravity.CENTER
            maxLines = 2
            setLineSpacing(0f, 1.15f)
            setPadding(0, dp(8f), 0, 0)
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        })
        layoutParams = LinearLayout.LayoutParams(dp(76f), -2)
        val id = it["id"] as? String ?: ""
        if (enabled) setOnClickListener { onPick(id) }
    }

    /** 图标：名字首字的圆（底色按 id 算，与 Dart 的 StoreAppIcon 同一个哈希）；有地址就后台拉下来盖上去 */
    private fun icon(id: String, name: String, url: String, sizeDp: Float): View {
        val px = dp(sizeDp)
        val slot = FrameLayout(a).apply { layoutParams = LinearLayout.LayoutParams(px, px) }
        slot.addView(TextView(a).apply {
            gravity = Gravity.CENTER
            text = name.takeIf { it.isNotEmpty() }?.let { String(Character.toChars(it.codePointAt(0))) } ?: "?"
            setTextColor(Color.WHITE)
            textSize = sizeDp * 0.42f
            typeface = Typeface.DEFAULT_BOLD
            background = GradientDrawable().apply { shape = GradientDrawable.OVAL; setColor(color(id)) }
        }, FrameLayout.LayoutParams(px, px))
        if (url.isNotEmpty()) {
            val img = ImageView(a).apply {
                scaleType = ImageView.ScaleType.CENTER_CROP
                outlineProvider = object : ViewOutlineProvider() {
                    override fun getOutline(view: View, outline: Outline) = outline.setOval(0, 0, view.width, view.height)
                }
                clipToOutline = true
            }
            slot.addView(img, FrameLayout.LayoutParams(px, px))
            Thread {
                val bm = runCatching {
                    (URL(url).openConnection() as HttpURLConnection).run {
                        connectTimeout = 4000
                        readTimeout = 4000
                        inputStream.use { BitmapFactory.decodeStream(it) }
                    }
                }.getOrNull() ?: return@Thread
                a.runOnUiThread { img.setImageBitmap(bm) }
            }.start()
        }
        return slot
    }

    private fun color(id: String): Int {
        val colors = intArrayOf(
            0xFF07C160.toInt(), 0xFF10AEFF.toInt(), 0xFFFA9D3B.toInt(), 0xFFE64340.toInt(),
            0xFF6467F0.toInt(), 0xFF1485EE.toInt(), 0xFF91D300.toInt(), 0xFF576B95.toInt(),
        )
        var h = 0L
        id.codePoints().forEach { h = (h * 31 + it) and 0xFFFFFFFFL }
        return colors[(h % colors.size).toInt()]
    }

    private fun column() = LinearLayout(a).apply {
        orientation = LinearLayout.VERTICAL
        background = GradientDrawable().apply {
            setColor(0xFFF7F7F7.toInt())
            val r = dp(12f).toFloat()
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

    private fun bottomDialog(content: View) = Dialog(a).apply {
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

    private fun divider() = View(a).apply {
        setBackgroundColor(0x1F000000)
        layoutParams = LinearLayout.LayoutParams(-1, maxOf(1, dp(0.5f))).apply {
            marginStart = dp(20f); marginEnd = dp(20f)
        }
    }

    private fun text(s: String, size: Float, color: Int, bold: Boolean = false) = TextView(a).apply {
        text = s
        textSize = size
        setTextColor(color)
        maxLines = 1
        ellipsize = TextUtils.TruncateAt.END
        if (bold) typeface = Typeface.DEFAULT_BOLD
    }

    private fun dp(v: Float) = (v * a.resources.displayMetrics.density).toInt()
}
