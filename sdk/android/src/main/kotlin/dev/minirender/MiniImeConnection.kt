package dev.minirender

import android.content.Context
import android.text.Editable
import android.text.InputType
import android.text.Selection
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager
import org.json.JSONObject

/**
 * 输入框聚焦时弹系统软键盘，把输入法交来的字交给引擎（[MiniProgramView] 内部用）。
 *
 * 引擎自己管输入框的值、光标、选区（`inputFocus` / `inputBlur` 事件），宿主只当「键盘的搬运工」。
 *
 * **对输入法要像一个真的编辑框**（`BaseInputConnection` 的 fullEditor + 一份镜像文本 [mirror]，聚焦时报出光标位置）：
 * 从前是「哑」连接（fullEditor = false、`initialSelStart = -1`、读不到光标前的字）—— 搜狗 / 百度 / 讯飞 / ColorOS
 * 这类中文输入法据此判定「这里不能编辑」，退成逐键发英文字母的模式：键盘弹出来是英文、切不了拼音。
 *
 * 输入法的每一下（上屏、组字、删字）都先落到镜像上（父类按标准语义改 [mirror]，含组字区），再把镜像前后的差
 * 换算成引擎的按键：删掉的字 → 退格，新出现的字 → `sendText`。拼音组字时组字区直接显示在输入框里（引擎没有组字区），
 * 上屏时换成最终的字。回车 / 编辑器动作（搜索、发送…）→ `enter`。
 *
 * 从前原生视图什么都没接，点输入框只有光标、不弹键盘（Flutter 插件有自己的 MiniTextInput）。
 */
internal class MiniIme(private val view: View, private val sendText: (String) -> Unit, private val sendKey: (String) -> Unit) {
    /** 当前聚焦的输入框（`inputFocus` 事件）；null = 没有，键盘收起 */
    private var focus: JSONObject? = null
    /** 输入框内容的镜像（输入法读写它，见 [Connection]）：与引擎里的值保持一致 */
    private val mirror: Editable = Editable.Factory.getInstance().newEditable("")

    private val imm get() = view.context.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager

    val active: Boolean get() = focus != null

    fun onFocus(ev: JSONObject) {
        val changed = focus?.optString("id") != ev.optString("id") || focus?.optString("inputType") != ev.optString("inputType")
        focus = ev
        val value = ev.optString("value")
        mirror.replace(0, mirror.length, value)
        BaseInputConnection.removeComposingSpans(mirror)
        val cursor = ev.optInt("cursor", value.length).coerceIn(0, value.length)
        Selection.setSelection(mirror, cursor)
        view.isFocusableInTouchMode = true
        view.requestFocus()
        // 换了输入框 / 键盘类型：让输入法重新问 onCreateInputConnection（数字键盘、密码、回车键文字）
        if (changed) imm?.restartInput(view)
        imm?.showSoftInput(view, InputMethodManager.SHOW_IMPLICIT)
    }

    fun onBlur() {
        if (focus == null) return
        focus = null
        mirror.clear()
        imm?.hideSoftInputFromWindow(view.windowToken, 0)
        imm?.restartInput(view)
    }

    fun create(out: EditorInfo): InputConnection? {
        val f = focus ?: return null
        val password = f.optBoolean("password")
        val textarea = f.optString("tag") == "textarea"
        out.inputType = when (f.optString("inputType")) {
            "number" -> InputType.TYPE_CLASS_NUMBER
            "digit" -> InputType.TYPE_CLASS_NUMBER or InputType.TYPE_NUMBER_FLAG_DECIMAL
            "idcard" -> InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
            else -> InputType.TYPE_CLASS_TEXT or (if (textarea) InputType.TYPE_TEXT_FLAG_MULTI_LINE else 0)
        } or (if (password) InputType.TYPE_TEXT_VARIATION_PASSWORD else 0)
        out.imeOptions = when (f.optString("confirmType")) {
            "search" -> EditorInfo.IME_ACTION_SEARCH
            "send" -> EditorInfo.IME_ACTION_SEND
            "next" -> EditorInfo.IME_ACTION_NEXT
            "go" -> EditorInfo.IME_ACTION_GO
            else -> if (textarea) EditorInfo.IME_ACTION_NONE else EditorInfo.IME_ACTION_DONE
        } or EditorInfo.IME_FLAG_NO_FULLSCREEN
        // 报出真实的光标位置：-1（「不支持」）会让中文输入法退成逐键发字母
        out.initialSelStart = Selection.getSelectionStart(mirror).coerceAtLeast(0)
        out.initialSelEnd = Selection.getSelectionEnd(mirror).coerceAtLeast(0)
        out.initialCapsMode = 0
        return Connection()
    }

    /**
     * 不经输入法的按键（硬件键盘、`adb shell input text`、部分输入法的退格 / 回车）：落到视图的 `onKeyDown`。
     * 返回是否吃掉了（没有聚焦的输入框时不吃，返回键照常交给 Activity）
     */
    fun onKey(event: KeyEvent): Boolean {
        if (focus == null || event.keyCode == KeyEvent.KEYCODE_BACK) return false
        val at = Selection.getSelectionStart(mirror).coerceIn(0, mirror.length)
        when (event.keyCode) {
            KeyEvent.KEYCODE_DEL -> {
                if (at > 0) mirror.delete(at - 1, at)
                sendKey("backspace")
            }
            KeyEvent.KEYCODE_FORWARD_DEL -> {
                if (at < mirror.length) mirror.delete(at, at + 1)
                sendKey("delete")
            }
            KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> sendKey("enter")
            KeyEvent.KEYCODE_DPAD_LEFT -> {
                Selection.setSelection(mirror, (at - 1).coerceAtLeast(0))
                sendKey("left")
            }
            KeyEvent.KEYCODE_DPAD_RIGHT -> {
                Selection.setSelection(mirror, (at + 1).coerceAtMost(mirror.length))
                sendKey("right")
            }
            else -> {
                val c = event.unicodeChar
                if (c == 0) return false
                val t = String(Character.toChars(c))
                mirror.insert(at, t)
                sendText(t)
            }
        }
        return true
    }

    /**
     * 镜像从 [before] 变成了现在的样子：把差换算成引擎的按键。取公共前后缀，中间删掉的按退格送、
     * 新出现的按文字送 —— 输入法的编辑都在光标处，引擎光标就在那段的末尾
     */
    private fun syncFrom(before: String) {
        val after = mirror.toString()
        if (before == after) return
        var p = 0
        val max = minOf(before.length, after.length)
        while (p < max && before[p] == after[p]) p++
        var q = 0
        while (q < max - p && before[before.length - 1 - q] == after[after.length - 1 - q]) q++
        val removed = before.substring(p, before.length - q)
        val added = after.substring(p, after.length - q)
        repeat(removed.codePointCount(0, removed.length)) { sendKey("backspace") }
        if (added.isNotEmpty()) sendText(added)
    }

    private inner class Connection : BaseInputConnection(view, true) {
        override fun getEditable(): Editable = mirror

        private inline fun edit(block: () -> Boolean): Boolean {
            val before = mirror.toString()
            val r = block()
            syncFrom(before)
            return r
        }

        override fun commitText(text: CharSequence, newCursorPosition: Int): Boolean {
            // 回车上屏（"\n"）：textarea 换行、input 触发 confirm（引擎认 "\n"），不进镜像以外的分支
            if (text.toString() == "\n" && focus?.optString("tag") != "textarea") {
                // 单行输入框的回车 = 确认：不进镜像（镜像里多一个换行，之后的差就对不上了）
                sendKey("enter")
                return true
            }
            return edit { super.commitText(text, newCursorPosition) }
        }

        override fun setComposingText(text: CharSequence, newCursorPosition: Int): Boolean =
            edit { super.setComposingText(text, newCursorPosition) }

        override fun finishComposingText(): Boolean = edit { super.finishComposingText() }

        override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean =
            edit { super.deleteSurroundingText(beforeLength, afterLength) }

        override fun deleteSurroundingTextInCodePoints(beforeLength: Int, afterLength: Int): Boolean =
            edit { super.deleteSurroundingTextInCodePoints(beforeLength, afterLength) }

        override fun setComposingRegion(start: Int, end: Int): Boolean = edit { super.setComposingRegion(start, end) }

        override fun performEditorAction(actionCode: Int): Boolean {
            sendKey("enter")
            return true
        }

        override fun sendKeyEvent(event: KeyEvent): Boolean {
            if (event.action != KeyEvent.ACTION_DOWN) return true
            when (event.keyCode) {
                // 退格 / 删除键：先改镜像（输入法下一次读光标前的字要对得上），再送引擎
                KeyEvent.KEYCODE_DEL -> edit { super.deleteSurroundingText(if (Selection.getSelectionStart(mirror) > 0) 1 else 0, 0) }
                KeyEvent.KEYCODE_FORWARD_DEL -> edit { super.deleteSurroundingText(0, 1) }
                KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> sendKey("enter")
                KeyEvent.KEYCODE_DPAD_LEFT -> sendKey("left")
                KeyEvent.KEYCODE_DPAD_RIGHT -> sendKey("right")
                else -> {
                    val c = event.unicodeChar
                    if (c != 0) edit { super.commitText(String(Character.toChars(c)), 1) }
                }
            }
            return true
        }
    }
}
