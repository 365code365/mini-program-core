package dev.minirender
import android.media.MediaPlayer
import android.media.PlaybackParams
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.widget.FrameLayout
import android.widget.MediaController
import android.widget.VideoView
import org.json.JSONObject
import java.io.File

/**
 * 小游戏的 `wx.createVideo`（game-doc/04 G7.7；引擎侧见 `src/js/prelude/game-proxies.js`）。
 *
 * 引擎发宿主事件 `{type:'video', op, id, props?, time?, direction?}`，这里每个 id 一个 [VideoView]，
 * 按 props 的 x / y / width / height（逻辑像素）摆在游戏画面上面。事件经 `contextEvent` 推回去：
 * play / pause / ended / waiting / error，播放中每 250ms 一次 timeUpdate {position, duration}（秒）。
 *
 * 取舍（记在 game-doc/验收/G7.md）：
 * - `underGameView`：引擎上屏是不透明的 Surface，视频透不上来，这里一律盖在游戏上面；
 * - `objectFit`：VideoView 只做 contain，cover / fill 没做；
 * - `requestFullScreen`：铺满宿主视图，不转屏（direction 只记下来）。
 * 所有方法在主线程调；[emit] 由调用方投递到引擎线程。
 */
internal class GameVideo(
    private val host: FrameLayout,
    private val dir: String,
    private val emit: (name: String, json: String) -> Unit,
) {
    private val density = host.resources.displayMetrics.density
    private val main = Handler(Looper.getMainLooper())
    private val videos = HashMap<String, Item>()

    private inner class Item(val id: String) {
        val view = VideoView(host.context)
        var props = JSONObject()
        var player: MediaPlayer? = null
        var src = ""
        var full = false
        /** 要播（play 之后、pause / stop 之前）：play 常在 prepared 之前调，那时 isPlaying 还是 false，tick 不能因此停 */
        var want = false
        val tick = object : Runnable {
            override fun run() {
                if (!want) return
                if (view.isPlaying) send(id, "timeUpdate", JSONObject().put("position", view.currentPosition / 1000.0).put("duration", view.duration / 1000.0))
                main.postDelayed(this, 250)
            }
        }
    }

    fun handle(o: JSONObject) {
        val id = o.optString("id")
        val op = o.optString("op")
        if (op == "create") { create(id, o.optJSONObject("props") ?: JSONObject()); return }
        val v = videos[id] ?: return
        when (op) {
            "update" -> apply(v, o.optJSONObject("props") ?: JSONObject())
            "play" -> play(v)
            "pause" -> { v.want = false; v.view.pause(); send(id, "pause") }
            "stop" -> { v.want = false; v.view.pause(); v.view.seekTo(0); send(id, "pause") }
            "seek" -> v.view.seekTo((o.optDouble("time", 0.0) * 1000).toInt())
            "requestFullScreen" -> { v.full = true; layout(v) }
            "exitFullScreen" -> { v.full = false; layout(v) }
            "destroy" -> destroy(v)
        }
    }

    fun release() = videos.values.toList().forEach { destroy(it) }

    private fun send(id: String, event: String, data: JSONObject = JSONObject()) =
        emit("contextEvent", JSONObject().put("context", "Video").put("contextId", id).put("event", event).put("data", data).toString())

    private fun create(id: String, p: JSONObject) {
        val v = Item(id)
        videos[id] = v
        // 游戏画面也是 SurfaceView：两个 SurfaceView 的前后不定，不设这句视频可能被压在下面
        v.view.setZOrderMediaOverlay(true)
        v.view.setOnPreparedListener { mp ->
            v.player = mp
            applyPlayer(v)
            val t = v.props.optDouble("initialTime", 0.0)
            if (t > 0) v.view.seekTo((t * 1000).toInt())
            mp.setOnInfoListener { _, what, _ ->
                when (what) {
                    MediaPlayer.MEDIA_INFO_BUFFERING_START -> send(id, "waiting")
                    MediaPlayer.MEDIA_INFO_BUFFERING_END -> send(id, "play")
                }
                false
            }
            mp.setOnBufferingUpdateListener { _, pct -> send(id, "progress", JSONObject().put("buffered", pct)) }
            if (v.props.optBoolean("autoplay")) play(v)
        }
        v.view.setOnCompletionListener { if (!v.props.optBoolean("loop")) send(id, "ended") }
        v.view.setOnErrorListener { _, what, extra ->
            send(id, "error", JSONObject().put("errMsg", "video error $what/$extra"))
            true
        }
        host.addView(v.view)
        apply(v, p)
    }

    private fun apply(v: Item, p: JSONObject) {
        v.props = p
        layout(v)
        v.view.setBackgroundColor(0) // backgroundColor 画在 VideoView 后面会盖住画面，留透明
        v.view.setMediaController(if (p.optBoolean("controls", true)) MediaController(host.context).also { it.setAnchorView(v.view) } else null)
        val src = p.optString("src")
        if (src.isNotEmpty() && src != v.src) {
            v.src = src
            v.player = null
            v.view.setVideoURI(uri(src))
        }
        applyPlayer(v)
    }

    /** loop / muted / playbackRate 要等 prepared 之后拿到 MediaPlayer 才能设 */
    private fun applyPlayer(v: Item) {
        val mp = v.player ?: return
        try {
            mp.isLooping = v.props.optBoolean("loop")
            val vol = if (v.props.optBoolean("muted")) 0f else 1f
            mp.setVolume(vol, vol)
            val rate = v.props.optDouble("playbackRate", 1.0).toFloat()
            if (Build.VERSION.SDK_INT >= 23 && rate > 0 && rate != 1f) {
                val playing = mp.isPlaying
                mp.playbackParams = PlaybackParams().setSpeed(rate)
                if (!playing) mp.pause() // 设速度会让暂停中的 MediaPlayer 开播
            }
        } catch (_: IllegalStateException) {}
    }

    private fun layout(v: Item) {
        val lp = if (v.full) FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT)
        else FrameLayout.LayoutParams(px(v.props.optDouble("width", 300.0)), px(v.props.optDouble("height", 150.0))).apply {
            leftMargin = px(v.props.optDouble("x", 0.0))
            topMargin = px(v.props.optDouble("y", 0.0))
        }
        v.view.layoutParams = lp
    }

    private fun play(v: Item) {
        v.want = true
        v.view.start()
        send(v.id, "play")
        main.removeCallbacks(v.tick)
        main.post(v.tick)
    }

    private fun destroy(v: Item) {
        v.want = false
        main.removeCallbacks(v.tick)
        v.view.stopPlayback()
        host.removeView(v.view)
        videos.remove(v.id)
    }

    private fun px(logical: Double) = (logical * density).toInt()

    /** 网络地址原样；其余按游戏包内的相对路径 */
    private fun uri(src: String): Uri =
        if (src.startsWith("http://") || src.startsWith("https://") || src.startsWith("file://")) Uri.parse(src)
        else Uri.fromFile(File(dir, src.removePrefix("/")))
}
