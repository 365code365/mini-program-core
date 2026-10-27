package dev.minirender

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Build
import android.os.Handler
import android.util.Log

/**
 * 小游戏的音频会话（game-doc/04 G5.5，引擎侧见 `src/game/audio/session.rs`）。
 *
 * - `wx.setInnerAudioOption({mixWithOther})`（引擎发 `audioSession` 事件）：
 *   `false` = 申请独占焦点 `AUDIOFOCUS_GAIN`（别的音乐 App 停）；`true`（默认）= 不抢焦点，跟别的 App 混着放；
 * - 打断开始 / 结束 → [onInterruption]：
 *   ① 持有焦点时 `AUDIOFOCUS_LOSS` / `LOSS_TRANSIENT` = 开始，重新拿到 `GAIN` = 结束（`LOSS_TRANSIENT_CAN_DUCK` 交给系统压低音量，不算打断）；
 *   ② 不持有焦点时靠音频模式认来电：`MODE_RINGTONE` / `IN_CALL` / `IN_COMMUNICATION` = 开始，回 `MODE_NORMAL` = 结束。
 *      API 31+ 用 `addOnModeChangedListener`，更早的每秒查一次 `audioManager.mode`（不需要电话权限）；
 *   两路任一在打断就算打断；
 * - 切后台放掉焦点（否则用户的音乐 App 回不来，06 §23），回前台按设置重新申请。被别的 App **永久**抢走焦点
 *   （`AUDIOFOCUS_LOSS`，例如用户在别的 App 里点了播放）系统不会再还回来：打断一直持续到游戏回前台、重新申请拿到为止
 *   （一加 PLY110 实测：酷狗播放 → LOSS → 暂停 → 打断开始；酷狗停了也没有 GAIN）。在后台期间打断状态不变，
 *   不会在后台派发 End（否则游戏在 End 里 play，回前台就自己响了）；
 * - `obeyMuteSwitch` / `speakerOn` 在 Android 上没有对应（没有静音键；外放是通话路由的概念），只记下来。
 *
 * 所有方法在调用方线程（MiniGameView 的引擎线程）上调；焦点回调与模式回调也投递到同一个 [handler]。
 */
internal class GameAudioSession(
    context: Context,
    private val handler: Handler,
    private val onInterruption: (begin: Boolean) -> Unit,
) {
    private val am = context.getSystemService(Context.AUDIO_SERVICE) as AudioManager
    private var mixWithOther = true
    private var foreground = true
    private var hasFocus = false
    private var focusLost = false
    private var inCall = false
    private var interrupted = false
    private var focusRequest: AudioFocusRequest? = null
    private var released = false

    private val focusListener = AudioManager.OnAudioFocusChangeListener { change ->
        handler.post {
            when (change) {
                AudioManager.AUDIOFOCUS_LOSS, AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> focusLost = true
                AudioManager.AUDIOFOCUS_GAIN -> focusLost = false
                else -> {}
            }
            if (change == AudioManager.AUDIOFOCUS_LOSS) hasFocus = false
            Log.i(TAG, "🔈 音频焦点变化 $change")
            update()
        }
    }

    private val modeListener: Any? = if (Build.VERSION.SDK_INT >= 31) {
        AudioManager.OnModeChangedListener { mode -> handler.post { onMode(mode) } }.also {
            am.addOnModeChangedListener({ r -> handler.post(r) }, it)
        }
    } else null

    private val poll = object : Runnable {
        override fun run() {
            if (released) return
            onMode(am.mode)
            handler.postDelayed(this, 1000)
        }
    }

    init {
        onMode(am.mode)
        if (modeListener == null) handler.postDelayed(poll, 1000)
    }

    private fun onMode(mode: Int) {
        val call = mode == AudioManager.MODE_RINGTONE || mode == AudioManager.MODE_IN_CALL || mode == AudioManager.MODE_IN_COMMUNICATION
        if (call != inCall) {
            inCall = call
            Log.i(TAG, "🔈 音频模式 $mode（${if (call) "通话 / 响铃" else "正常"}）")
            update()
        }
    }

    private fun update() {
        val now = focusLost || inCall
        if (now != interrupted && !released) {
            interrupted = now
            onInterruption(now)
        }
    }

    /** 引擎的 `audioSession` 事件 */
    fun apply(o: org.json.JSONObject) {
        val mix = o.optBoolean("mixWithOther", true)
        if (mix == mixWithOther) return
        mixWithOther = mix
        syncFocus(clearLost = true)
    }

    fun setForeground(fg: Boolean) {
        foreground = fg
        syncFocus(clearLost = false)
    }

    /** 该持有焦点 = 前台且 mixWithOther 为 false。`clearLost`：放掉焦点时顺带结束「焦点被抢」的打断（改成可混音时） */
    private fun syncFocus(clearLost: Boolean = true) {
        val want = foreground && !mixWithOther && !released
        if (want && !hasFocus) request() else if (!want && (hasFocus || clearLost && focusLost)) abandon(clearLost)
    }

    private fun request() {
        val r = if (Build.VERSION.SDK_INT >= 26) {
            val req = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
                .setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_GAME).setContentType(AudioAttributes.CONTENT_TYPE_MUSIC).build())
                .setOnAudioFocusChangeListener(focusListener, handler)
                .setWillPauseWhenDucked(false)
                .build()
            focusRequest = req
            am.requestAudioFocus(req)
        } else {
            @Suppress("DEPRECATION")
            am.requestAudioFocus(focusListener, AudioManager.STREAM_MUSIC, AudioManager.AUDIOFOCUS_GAIN)
        }
        hasFocus = r == AudioManager.AUDIOFOCUS_REQUEST_GRANTED
        // 申请被拒（通话中）：与打断同一个状态，等系统还焦点时 GAIN 回调里结束
        focusLost = !hasFocus
        Log.i(TAG, "🔈 申请独占音频焦点：${if (hasFocus) "拿到" else "被拒（$r）"}")
        update()
    }

    private fun abandon(clearLost: Boolean) {
        if (hasFocus) {
            if (Build.VERSION.SDK_INT >= 26) focusRequest?.let { am.abandonAudioFocusRequest(it) }
            else @Suppress("DEPRECATION") am.abandonAudioFocus(focusListener)
        }
        hasFocus = false
        // 改成 mixWithOther: true：不再需要焦点，「被抢」的打断随之结束；切后台：状态留着，回前台重新申请时再定
        if (clearLost) focusLost = false
        update()
    }

    fun release() {
        if (hasFocus) abandon(clearLost = false)
        released = true
        handler.removeCallbacks(poll)
        if (Build.VERSION.SDK_INT >= 31) (modeListener as? AudioManager.OnModeChangedListener)?.let { am.removeOnModeChangedListener(it) }
    }

    private companion object {
        const val TAG = "mini-game"
    }
}
