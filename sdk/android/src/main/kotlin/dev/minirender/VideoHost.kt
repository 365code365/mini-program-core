package dev.minirender

import android.media.AudioAttributes
import android.media.MediaPlayer
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.util.Log
import org.json.JSONArray
import org.json.JSONObject

/**
 * 同层渲染的 `<video>`（创建选项 `sameLayer: ["video"]`）：系统的 [MediaPlayer] 解码（硬解、带声音），
 * 每一帧经 `SurfaceTexture` + GLES 读回 RGBA（[GlFrameReader]）交给引擎（[MiniEngine.nativeVideoFrameRgba]），
 * **引擎把它画进页面**，滚动时视频就是页面画布里的一块像素。
 * （不用 `ImageReader` 直接拿 YUV：硬解码器的缓冲常常 CPU 锁不上，取平面会让进程闪退，见 [GlFrameReader]）
 *
 * ## 为什么不再用 Flutter 控件盖在上面
 * 盖上去的控件与引擎画面走两条上屏路（引擎画到自己的 SurfaceView，系统合成器直接取；控件要经事件、主线程、
 * Flutter 的下一帧、混合合成），每帧差几个 vsync 不固定：一拖视频框就跟页面错开、一顿一顿。
 *
 * ## 线程
 * 播放器、`SurfaceTexture` 的回调、GL 读回都在自己的 `mini-video` 线程上（引擎线程只管画）。
 * 组件事件（`play` / `pause` / `timeupdate` / `ended` / `error` …）经 [emit] 送回引擎线程派发给 `bind*`；
 * 引擎发来的 `videos`（页面上有哪些视频）与 `videoCommand`（点了控制条 / VideoContext）从 [onEvent] 进来。
 *
 * 控制条、全屏、静音按钮都在引擎里：全屏时 `videos` 里那一个的 `box` 换成全屏画面的尺寸、`object-fit`
 * 换成 contain（照着读回就行），静音看 `attrs.muted`（[Player.update]）。
 *
 * 原生 SDK（[EngineSession]）与 Flutter 插件（`EngineHost`）共用这一份。
 */
internal class VideoHost(
    /** 回传组件事件（调用方投递到引擎线程上执行 `nativeNativeViewEvent`） */
    private val emit: (key: String, name: String, detail: JSONObject) -> Unit,
    /** 交了一帧：让引擎线程出一帧（引擎只重画这个视频框） */
    private val onFrame: () -> Unit,
) {
    private val thread = HandlerThread("mini-video").apply { start() }
    private val handler = Handler(thread.looper)

    /** 只在 [handler] 线程上读写 */
    private val players = HashMap<String, Player>()

    /** 这条线程上的 GL 上下文（读回画面用） */
    private val gl = GlFrameReader(handler)
    private var glOk = false

    init {
        handler.post {
            glOk = gl.init()
            if (!glOk) Log.w(TAG, "🎬 建不了 GLES 上下文：同层渲染的视频出不了画面（声音与事件照常）")
        }
    }

    /** 引擎线程调（drainEvents）：认得的事件返回 true（照样也交给宿主的界面线程） */
    fun onEvent(raw: String): Boolean {
        val type = when {
            raw.contains("\"type\":\"videos\"") -> "videos"
            raw.contains("\"type\":\"videoCommand\"") -> "videoCommand"
            else -> return false
        }
        val ev = try {
            JSONObject(raw)
        } catch (e: Exception) {
            return false
        }
        handler.post {
            if (type == "videos") sync(ev.optJSONArray("views") ?: JSONArray()) else players[ev.optString("key")]?.command(ev)
        }
        return true
    }

    /** 进后台：正在播的都停下（与微信一致，回前台不自动接着播） */
    fun pauseAll() {
        handler.post { players.values.forEach { it.pause(emitEvent = true) } }
    }

    fun release() {
        handler.post {
            players.values.forEach { it.release() }
            players.clear()
            gl.release()
            thread.quitSafely()
        }
    }

    /** 页面上的视频变了：新来的建播放器、地址变了的换片源、没了的释放 */
    private fun sync(views: JSONArray) {
        val seen = HashSet<String>()
        for (i in 0 until views.length()) {
            val v = views.optJSONObject(i) ?: continue
            val key = v.optString("key")
            if (key.isEmpty()) continue
            seen.add(key)
            val url = v.optString("url")
            val attrs = v.optJSONObject("attrs") ?: JSONObject()
            val box = v.optJSONArray("box")
            val bw = box?.optInt(0) ?: 0
            val bh = box?.optInt(1) ?: 0
            val p = players[key]
            if (p == null || p.url != url) {
                p?.release()
                players[key] = Player(key, url, attrs).also { it.box(bw, bh) }
            } else {
                p.update(attrs)
                p.box(bw, bh)
            }
        }
        players.keys.filter { it !in seen }.forEach { players.remove(it)?.release() }
    }

    private inner class Player(val key: String, val url: String, private var attrs: JSONObject) {
        private val mp = MediaPlayer()
        /** 解码器往这里交帧（GL 上下文没建起来时为 null：只有声音与事件） */
        private val target = if (glOk) gl.Target { deliver() } else null
        private var videoW = 0
        private var videoH = 0
        /** 引擎里视频框的设备像素尺寸（`videos` 事件的 `box`）：读回时就按它与 object-fit 缩好 */
        private var boxW = 0
        private var boxH = 0

        fun box(w: Int, h: Int) {
            boxW = w
            boxH = h
        }
        private var prepared = false
        private var released = false
        /** 还没准备好就被要求播（autoplay / 点了播放）：准备好了再播 */
        private var wantPlay = false
        private var completed = false
        /**
         * 播放状态以这个为准（`start()` 之后到 pause / 播完 / 出错之前），不看 `mp.isPlaying`：
         * 网络视频 seek 或从头重播后要缓冲，这期间 `isPlaying` 是 false（PLY110 上实测一缓冲十几秒）——
         * 从前 ticker 见它 false 就不再续，进度从此停住直到播完；这时点暂停也被当成「没在播」吞掉，缓冲完接着放
         */
        private var playing = false
        /** `stop` 的那次 seek 不发 seekcomplete（与引擎自己的播放器一致） */
        private var quietSeek = false
        private var rate = 1f

        private val ticker = object : Runnable {
            override fun run() {
                if (released || !playing) return
                emit(key, "timeupdate", time())
                handler.postDelayed(this, TIMEUPDATE_MS)
            }
        }

        init {
            try {
                mp.setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_MEDIA)
                        .setContentType(AudioAttributes.CONTENT_TYPE_MOVIE)
                        .build(),
                )
                mp.setOnPreparedListener { onPrepared() }
                mp.setOnVideoSizeChangedListener { _, w, h -> setVideoSize(w, h) }
                target?.let { mp.setSurface(it.surface) }
                mp.setOnCompletionListener { onCompleted() }
                mp.setOnSeekCompleteListener {
                    if (!quietSeek) emit(key, "seekcomplete", JSONObject().put("position", mp.currentPosition / 1000.0))
                    quietSeek = false
                }
                mp.setOnInfoListener { _, what, _ ->
                    if (what == MediaPlayer.MEDIA_INFO_BUFFERING_START) emit(key, "waiting", JSONObject())
                    false
                }
                mp.setOnErrorListener { _, what, extra ->
                    fail("MediaPlayer error $what/$extra")
                    true
                }
                mp.setDataSource(url)
                mp.isLooping = flag("loop", false)
                mp.prepareAsync()
                wantPlay = flag("autoplay", false)
            } catch (e: Exception) {
                fail("$e")
            }
        }

        private fun flag(name: String, default: Boolean): Boolean {
            if (!attrs.has(name)) return default
            return attrs.optString(name).trim() !in listOf("false", "{{false}}", "0")
        }

        private fun time() = JSONObject()
            .put("currentTime", mp.currentPosition / 1000.0)
            .put("duration", mp.duration.coerceAtLeast(0) / 1000.0)

        private fun fail(msg: String) {
            Log.w(TAG, "🎬 $key：$msg（$url）")
            playing = false
            handler.removeCallbacks(ticker)
            emit(key, "error", JSONObject().put("errMsg", msg))
        }

        private fun onPrepared() {
            if (released) return
            prepared = true
            setVideoSize(mp.videoWidth, mp.videoHeight)
            if (flag("muted", false)) mp.setVolume(0f, 0f)
            emit(
                key,
                "loadedmetadata",
                JSONObject().put("width", mp.videoWidth).put("height", mp.videoHeight).put("duration", mp.duration / 1000.0),
            )
            val initial = attrs.optString("initial-time").toDoubleOrNull() ?: 0.0
            if (initial > 0) {
                quietSeek = true
                mp.seekTo((initial * 1000).toInt())
            }
            if (wantPlay) play()
        }

        private fun setVideoSize(w: Int, h: Int) {
            if (released || w <= 0 || h <= 0) return
            videoW = w
            videoH = h
            target?.setSize(w, h)
        }

        /** 读回多大：按引擎的视频框与 `object-fit` 缩好（引擎那边就只剩整行拷贝）；还不知道框就按视频本身、长边封顶 */
        private fun readSize(vw: Int, vh: Int): Pair<Int, Int> {
            if (boxW <= 0 || boxH <= 0) {
                val s = minOf(1f, MAX_SIDE.toFloat() / maxOf(vw, vh))
                return maxOf(1, (vw * s).toInt()) to maxOf(1, (vh * s).toInt())
            }
            val (sx, sy) = boxW.toFloat() / vw to boxH.toFloat() / vh
            val s = when (attrs.optString("object-fit").trim()) {
                "fill" -> return boxW to boxH
                "cover" -> maxOf(sx, sy)
                else -> minOf(sx, sy)
            }
            return maxOf(1, Math.round(vw * s)) to maxOf(1, Math.round(vh * s))
        }

        /** 解码器交了一帧（SurfaceTexture 的回调，这条线程上）：读回 RGBA 交给引擎 */
        private fun deliver() {
            val t = target ?: return
            if (released) return
            val (vw, vh) = if (videoW > 0 && videoH > 0) videoW to videoH else 16 to 16
            val (w, h) = readSize(vw, vh)
            try {
                // 尺寸还不知道时也要 updateTexImage 取走这一帧，否则解码器不再交下一帧
                t.read(w, h) { buf -> if (videoW > 0) MiniEngine.nativeVideoFrameRgba(key, w, h, w * 4, buf) }
            } catch (e: Exception) {
                Log.w(TAG, "🎬 $key：读帧失败 $e")
                return
            }
            if (videoW > 0) onFrame()
        }

        private fun onCompleted() {
            if (mp.isLooping) return
            completed = true
            playing = false
            handler.removeCallbacks(ticker)
            emit(key, "timeupdate", JSONObject().put("currentTime", mp.duration / 1000.0).put("duration", mp.duration / 1000.0))
            emit(key, "ended", JSONObject())
        }

        fun play() {
            if (!prepared) {
                wantPlay = true
                emit(key, "waiting", JSONObject())
                return
            }
            if (playing) return
            if (completed) {
                completed = false
                quietSeek = true
                mp.seekTo(0)
            }
            if (rate != 1f && Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                // 先设倍速再开播：给暂停中的播放器设非 0 倍速，有的机型会直接开播
                mp.playbackParams = mp.playbackParams.setSpeed(rate)
            }
            mp.start()
            playing = true
            emit(key, "play", JSONObject())
            handler.removeCallbacks(ticker)
            handler.post(ticker)
        }

        fun pause(emitEvent: Boolean = true) {
            wantPlay = false
            if (!prepared || !playing) return
            playing = false
            mp.pause()
            handler.removeCallbacks(ticker)
            if (emitEvent) emit(key, "pause", JSONObject())
        }

        fun command(c: JSONObject) {
            if (released) return
            when (c.optString("command")) {
                "play" -> play()
                "pause" -> pause()
                "stop" -> {
                    pause()
                    if (prepared) {
                        quietSeek = true
                        mp.seekTo(0)
                    }
                }
                "seek" -> if (prepared) {
                    completed = false
                    mp.seekTo((c.optDouble("position", 0.0) * 1000).toInt())
                }
                "playbackRate" -> {
                    rate = c.optDouble("rate", 1.0).toFloat().coerceIn(0.5f, 2f)
                    if (prepared && playing && Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                        mp.playbackParams = mp.playbackParams.setSpeed(rate)
                    }
                }
            }
        }

        /** 属性变了（地址没变）：循环 / 静音即时生效 */
        fun update(next: JSONObject) {
            attrs = next
            mp.isLooping = flag("loop", false)
            if (prepared) {
                val vol = if (flag("muted", false)) 0f else 1f
                mp.setVolume(vol, vol)
            }
        }

        fun release() {
            if (released) return
            released = true
            handler.removeCallbacks(ticker)
            try {
                mp.reset()
                mp.release()
            } catch (_: Exception) {
            }
            target?.release()
        }
    }

    private companion object {
        /** timeupdate 的间隔（微信：250ms 一次） */
        const val TIMEUPDATE_MS = 250L
        /** 读回画面的长边上限（物理像素）：手机上的视频框宽也就 1000 出头 */
        const val MAX_SIDE = 1280
        const val TAG = "mini-render"
    }
}
