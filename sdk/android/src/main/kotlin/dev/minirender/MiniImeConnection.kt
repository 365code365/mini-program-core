package dev.minirender

import android.content.Context
import android.text.InputType
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
 * 引擎自己管输入框的值、光标、选区（`inputFocus` / `inputBlur` 事件），宿主只当「键盘的搬运工」：
 * - `commitText` → `sendText`；
 * - 拼音这类**组字**（`setComposingText`）：先退格删掉上一次组的字、再送新的，组字过程直接显示在输入框里，
 *   上屏（`commitText` / `finishComposingText`）时换成最终的字 —— 引擎没有 IME 组字区，这是最省事又对得上的做法；
 * - `deleteSurroundingText` / 退格键 → `backspace` / `delete`；回车 / 编辑器动作（搜索、发送…）→ `enter`。
 *
 * 从前原生视图什么都没接，点输入框只有光标、不弹键盘（Flutter 插件有自己的 MiniTextInput）。
 */
internal class MiniIme(private val view: View, private val sendText: (String) -> Unit, private val sendKey: (String) -> Unit) {
    /** 当前聚焦的输入框（`inputFocus` 事件）；null = 没有，键盘收起 */
    private var focus: JSONObject? = null
    /** 输入法正在组的字（已经送进引擎、还没上屏的那一截） */
    private var composing = ""

    private val imm get() = view.context.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager

    val active: Boolean get() = focus != null

    fun onFocus(ev: JSONObject) {
        val changed = focus?.optString("id") != ev.optString("id") || focus?.optString("inputType") != ev.optString("inputType")
        focus = ev
        composing = ""
        view.isFocusableInTouchMode = true
        view.requestFocus()
        // 换了输入框 / 键盘类型：让输入法重新问 onCreateInputConnection（数字键盘、密码、回车键文字）
        if (changed) imm?.restartInput(view)
        imm?.showSoftInput(view, InputMethodManager.SHOW_IMPLICIT)
    }

    fun onBlur() {
        if (focus == null) return
        focus = null
        composing = ""
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
        out.initialSelStart = -1
        out.initialSelEnd = -1
        return Connection()
    }

    /**
     * 不经输入法的按键（硬件键盘、`adb shell input text`、部分输入法的退格 / 回车）：落到视图的 `onKeyDown`。
     * 返回是否吃掉了（没有聚焦的输入框时不吃，返回键照常交给 Activity）
     */
    fun onKey(event: KeyEvent): Boolean {
        if (focus == null || event.keyCode == KeyEvent.KEYCODE_BACK) return false
        when (event.keyCode) {
            KeyEvent.KEYCODE_DEL -> sendKey("backspace")
            KeyEvent.KEYCODE_FORWARD_DEL -> sendKey("delete")
            KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> sendKey("enter")
            KeyEvent.KEYCODE_DPAD_LEFT -> sendKey("left")
            KeyEvent.KEYCODE_DPAD_RIGHT -> sendKey("right")
            else -> {
                val c = event.unicodeChar
                if (c == 0) return false
                sendText(String(Character.toChars(c)))
            }
        }
        return true
    }

    /** 把组字区换成 [text]：退格删掉旧的，再送新的 */
    private fun replaceComposing(text: String) {
        repeat(composing.codePointCount(0, composing.length)) { sendKey("backspace") }
        if (text.isNotEmpty()) sendText(text)
        composing = text
    }

    private inner class Connection : BaseInputConnection(view, false) {
        override fun commitText(text: CharSequence, newCursorPosition: Int): Boolean {
            val s = text.toString()
            // 回车上屏：textarea 换行、input 触发 confirm（引擎认 "\n"）
            replaceComposing(s)
            composing = ""
            return true
        }

        override fun setComposingText(text: CharSequence, newCursorPosition: Int): Boolean {
            replaceComposing(text.toString())
            return true
        }

        override fun finishComposingText(): Boolean {
            composing = ""
            return true
        }

        override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
            if (composing.isNotEmpty()) {
                replaceComposing("")
                return true
            }
            repeat(beforeLength.coerceAtMost(64)) { sendKey("backspace") }
            repeat(afterLength.coerceAtMost(64)) { sendKey("delete") }
            return true
        }

        override fun performEditorAction(actionCode: Int): Boolean {
            sendKey("enter")
            return true
        }

        override fun sendKeyEvent(event: KeyEvent): Boolean {
            if (event.action != KeyEvent.ACTION_DOWN) return true
            when (event.keyCode) {
                KeyEvent.KEYCODE_DEL -> sendKey("backspace")
                KeyEvent.KEYCODE_FORWARD_DEL -> sendKey("delete")
                KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> sendKey("enter")
                KeyEvent.KEYCODE_DPAD_LEFT -> sendKey("left")
                KeyEvent.KEYCODE_DPAD_RIGHT -> sendKey("right")
                else -> {
                    val c = event.unicodeChar
                    if (c != 0) sendText(String(Character.toChars(c)))
                }
            }
            return true
        }
    }
}
