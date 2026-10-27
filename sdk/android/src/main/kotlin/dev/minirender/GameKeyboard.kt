package dev.minirender

import android.content.Context
import android.graphics.Color
import android.os.Build
import android.text.Editable
import android.text.InputFilter
import android.text.InputType
import android.text.TextWatcher
import android.view.Gravity
import android.view.KeyEvent
import android.view.View
import android.view.WindowInsets
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.Button
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.LinearLayout

/**
 * 小游戏的键盘（game-doc/04 G7.2；引擎侧见 `src/js/prelude/game-keyboard.js`）。
 *
 * 与微信一致：游戏画面不动（窗口 adjustNothing），输入法上方浮一条原生输入框 + 确认键，游戏只收事件。
 * - `showKeyboard`：defaultValue / maxLength / multiple（多行）/ confirmType（done / next / search / go / send 对应输入法的动作键）；
 * - 输入每变一次 → `keyboardInput {value}`；输入法动作键或确认键 → `keyboardConfirm {value}`；
 * - 收起（hideKeyboard、用户按返回键收掉输入法）→ `keyboardComplete {value}`；
 * - 输入法高度变化 → `keyboardHeightChange {height}`（逻辑像素），输入条贴着输入法顶上。
 * 输入条的样式没有取证（微信是白底 + 绿色确认键），这里按那个样子画个大概。
 * 所有方法在主线程调；[emit] 由调用方投递到引擎线程。
 */
internal class GameKeyboard(
    private val host: FrameLayout,
    private val emit: (name: String, json: String) -> Unit,
) {
    private val ctx: Context = host.context
    private val density = ctx.resources.displayMetrics.density
    private var bar: LinearLayout? = null
    private var edit: EditText? = null
    private var shown = false
    private var imeHeight = 0
    private var suppressInput = false

    init {
        (ctx as? android.app.Activity)?.window?.setSoftInputMode(android.view.WindowManager.LayoutParams.SOFT_INPUT_ADJUST_NOTHING)
        host.setOnApplyWindowInsetsListener { v, insets ->
            if (Build.VERSION.SDK_INT >= 30) onIme(insets.getInsets(WindowInsets.Type.ime()).bottom)
            v.onApplyWindowInsets(insets)
        }
    }

    private fun value(): String = edit?.text?.toString() ?: ""

    private fun json(k: String, v: Any) = org.json.JSONObject().put(k, v).toString()

    private fun onIme(px: Int) {
        if (px == imeHeight) return
        val was = imeHeight
        imeHeight = px
        emit("keyboardHeightChange", json("height", (px / density).toInt()))
        bar?.translationY = -px.toFloat()
        // 用户自己把输入法收掉（返回键）：当作收起
        if (shown && was > 0 && px == 0) hide()
    }

    private fun ensureBar(): Pair<LinearLayout, EditText> {
        bar?.let { return it to edit!! }
        val e = EditText(ctx).apply {
            setBackgroundColor(Color.WHITE)
            setTextColor(Color.BLACK)
            setPadding((12 * density).toInt(), (8 * density).toInt(), (12 * density).toInt(), (8 * density).toInt())
        }
        val b = Button(ctx).apply {
            setBackgroundColor(Color.rgb(0x07, 0xC1, 0x60))
            setTextColor(Color.WHITE)
            setOnClickListener { confirm() }
        }
        val row = LinearLayout(ctx).apply {
            orientation = LinearLayout.HORIZONTAL
            setBackgroundColor(Color.rgb(0xF7, 0xF7, 0xF7))
            setPadding((8 * density).toInt(), (6 * density).toInt(), (8 * density).toInt(), (6 * density).toInt())
            addView(e, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(b, LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply { marginStart = (8 * density).toInt() })
            visibility = View.GONE
        }
        host.addView(row, FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.WRAP_CONTENT, Gravity.BOTTOM))
        e.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}
            override fun onTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}
            override fun afterTextChanged(s: Editable?) { if (!suppressInput && shown) emit("keyboardInput", json("value", value())) }
        })
        e.setOnEditorActionListener { _, action, ev ->
            val enter = ev != null && ev.keyCode == KeyEvent.KEYCODE_ENTER && ev.action == KeyEvent.ACTION_DOWN
            if (action != EditorInfo.IME_NULL || enter) { confirm(); true } else false
        }
        bar = row
        edit = e
        return row to e
    }

    private fun confirm() {
        if (shown) emit("keyboardConfirm", json("value", value()))
    }

    fun show(o: org.json.JSONObject) {
        val (row, e) = ensureBar()
        val multiple = o.optBoolean("multiple")
        val type = o.optString("confirmType", "done")
        e.filters = arrayOf(InputFilter.LengthFilter(o.optInt("maxLength", 100).coerceAtLeast(1)))
        e.inputType = InputType.TYPE_CLASS_TEXT or if (multiple) InputType.TYPE_TEXT_FLAG_MULTI_LINE else 0
        e.isSingleLine = !multiple
        e.imeOptions = when (type) {
            "next" -> EditorInfo.IME_ACTION_NEXT
            "search" -> EditorInfo.IME_ACTION_SEARCH
            "go" -> EditorInfo.IME_ACTION_GO
            "send" -> EditorInfo.IME_ACTION_SEND
            else -> EditorInfo.IME_ACTION_DONE
        }
        (row.getChildAt(1) as Button).text = when (type) { "next" -> "下一步"; "search" -> "搜索"; "go" -> "前往"; "send" -> "发送"; else -> "完成" }
        suppressInput = true
        e.setText(o.optString("defaultValue"))
        e.setSelection(e.text.length)
        suppressInput = false
        row.visibility = View.VISIBLE
        shown = true
        e.requestFocus()
        e.post { (ctx.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager).showSoftInput(e, InputMethodManager.SHOW_IMPLICIT) }
    }

    fun update(o: org.json.JSONObject) {
        val e = edit ?: return
        suppressInput = true
        e.setText(o.optString("value"))
        e.setSelection(e.text.length)
        suppressInput = false
    }

    fun hide() {
        if (!shown) return
        shown = false
        val v = value()
        edit?.let { (ctx.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager).hideSoftInputFromWindow(it.windowToken, 0) }
        bar?.visibility = View.GONE
        emit("keyboardComplete", json("value", v))
    }
}
