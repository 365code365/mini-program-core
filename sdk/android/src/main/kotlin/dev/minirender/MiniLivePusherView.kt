package dev.minirender

import android.Manifest
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Color
import android.os.Handler
import android.os.Looper
import android.util.Log
import org.json.JSONObject
import java.io.File
import java.nio.ByteBuffer

/**
 * SDK 自带的 `<live-pusher>`（官方 component/live-pusher）：相机预览同层渲染进页面（[CameraPipe]），推流是
 * 硬编 H.264 + AAC（[AvEncoder]）经 RTMP 发出去（[RtmpPublisher]）。
 *
 * - 属性：`url`（rtmp）、`mode`（QVGA / HVGA / SD / HD / FHD / RTC → 推流分辨率）、`video-width` / `video-height`（写了以它为准）、
 *   `orientation`（horizontal 宽高对调）、`aspect`（3:4 时按宽算高）、`fps`（1~30，默认 15）、`max-bitrate`（kbps，默认 1000，按它编）、
 *   `audio-quality`（high 48kHz / low 16kHz）、`enable-mic` / `muted`、`enable-camera`、`autopush`、`device-position`（默认 front）、
 *   `local-mirror`（auto / enable / disable，只管预览）、`remote-mirror` / `mirror`（推出去的画面）；
 *   `beauty` / `whiteness` / `beauty-style` / `filter`：GL 着色器（[FrameEffects]），预览与推出去的流一样；
 *   `picture-in-picture-mode`：引擎那边做（离开页面时缩成右下角的小窗，host/engine_pip.rs），这里照常交帧、推流不断；
 *   变声 / 混响 / 自定义渲染 / 自定义特效**没做**（属性收下不生效）。
 * - 事件：`statechange {code, message}`（1003 打开摄像头、1007 首帧、1008 编码器启动、2027 麦克风、1001 / 1002 / 1009 / 1102 / 3001~3005 / -1307 见
 *   [RtmpPublisher]、-1301 / -1302、5000 进后台）、`netstatus {info}` 每秒一次、`error {errCode, errMsg}`（10001 / 10002 没授权）。
 * - LivePusherContext：start / stop / pause / resume / startPreview / stopPreview / switchCamera / toggleTorch / snapshot / setZoom / getMaxZoom / setMICVolume，
 *   背景音 playBGM({url}) / pauseBGM / resumeBGM / stopBGM / setBGMVolume({volume})（[BgmPlayer]：本地放、混进推流；
 *   `bgmstart`、`bgmprogress {progress, duration}`（毫秒，每 500ms）、`bgmcomplete`，打不开 `error {errCode: 10003}`）。
 */
class MiniLivePusherView : MiniSameLayerView() {
    private val main = Handler(Looper.getMainLooper())
    private var pipe: CameraPipe? = null
    private var front = true
    private var preview = false
    private var attached = false
    private var paused = false
    private var torch = false

    private var enc: AvEncoder? = null
    private var pub: RtmpPublisher? = null
    private var pushing = false
    private var pushPaused = false
    private var bgm: BgmPlayer? = null
    private var bgmWasPlaying = false

    override val contextClass: String get() = "LivePusherContext"
    override val contextMethods: List<String> get() = METHODS

    override fun onDraw(canvas: Canvas, width: Int, height: Int, density: Float) {
        canvas.drawColor(Color.BLACK)
    }

    override fun onBox(w: Int, h: Int) {
        pipe?.setBox(w, h)
    }

    private fun flag(name: String, dflt: Boolean): Boolean {
        if (!attrs.has(name)) return dflt
        return attrs.optString(name).trim() !in listOf("false", "0", "")
    }

    private fun state(code: Int, message: String) {
        emit("statechange", JSONObject().put("code", code).put("message", message))
    }

    override fun onAttrs(attrs: JSONObject) {
        invalidate()
        val f = attrs.optString("device-position").trim().let { it.isEmpty() || it == "front" }
        if (!attached) {
            attached = true
            front = f
            if (flag("enable-camera", true)) startPreview(null)
            if (flag("autopush", false) && attrs.optString("url").isNotBlank()) start(null)
            return
        }
        if (f != front) switchCamera(null)
        pipe?.setEffects(FrameEffects.parse(attrs))
        pipe?.previewMirror = previewMirror()
        enc?.micMuted = !flag("enable-mic", true) || flag("muted", false)
    }

    private fun previewMirror(): Boolean {
        val want = when (attrs.optString("local-mirror").trim()) {
            "enable" -> true
            "disable" -> false
            else -> front
        }
        return CameraPipe.extraMirror(pipe?.front ?: front, want)
    }

    private fun remoteMirror(p: CameraPipe) = CameraPipe.extraMirror(p.front, flag("remote-mirror", false) || flag("mirror", false))

    private val listener = object : CameraPipe.Listener {
        override fun onOpened(maxZoom: Double) = state(1003, "打开摄像头成功")
        override fun onError(msg: String, denied: Boolean) {
            Log.w(TAG, "📡 live-pusher：$msg")
            state(-1301, "打开摄像头失败")
            if (denied) emit("error", JSONObject().put("errCode", 10001).put("errMsg", "用户禁止使用摄像头"))
        }
        override fun onFirstFrame() = state(1007, "首帧画面采集完成")
        override fun onPreview(buf: ByteBuffer, w: Int, h: Int) = pushFrame(w, h, buf)
    }

    private fun startPreview(reply: MiniHostReply?) {
        val ctx = context ?: return reply?.fail("startPreview:fail 宿主没有 Context") ?: Unit
        CameraPermissions.ensure(ctx, Manifest.permission.CAMERA) { ok, why ->
            if (!ok) {
                state(-1301, "打开摄像头失败")
                emit("error", JSONObject().put("errCode", 10001).put("errMsg", "用户禁止使用摄像头（$why）"))
                reply?.fail("startPreview:fail $why")
                return@ensure
            }
            preview = true
            if (!paused) openCamera()
            reply?.success()
        }
    }

    private fun openCamera() {
        val ctx = context ?: return
        val p = pipe ?: CameraPipe(ctx, listener).also { pipe = it }
        p.setBox(boxWidth, boxHeight)
        p.setEffects(FrameEffects.parse(attrs))
        p.previewEnabled = true
        p.open(front, 1280, 16.0 / 9.0)
        p.previewMirror = CameraPipe.extraMirror(front, when (attrs.optString("local-mirror").trim()) {
            "enable" -> true
            "disable" -> false
            else -> front
        })
    }

    /** 推流分辨率：写了 video-width / video-height 以它为准，否则按 mode；横屏宽高对调；都取偶数 */
    private fun pushSize(): Pair<Int, Int> {
        var (w, h) = when (attrs.optString("mode").trim()) {
            "QVGA" -> 240 to 320
            "HVGA" -> 320 to 480
            "HD" -> 540 to 960
            "FHD" -> 720 to 1280
            else -> 360 to 640
        }
        if (attrs.optString("aspect").trim() == "3:4") h = w * 4 / 3
        attrs.optString("video-width").toIntOrNull()?.takeIf { it in 16..1920 }?.let { w = it }
        attrs.optString("video-height").toIntOrNull()?.takeIf { it in 16..1920 }?.let { h = it }
        if (attrs.optString("orientation").trim() == "horizontal") {
            val t = w
            w = h
            h = t
        }
        return (w and 1.inv()) to (h and 1.inv())
    }

    private fun start(reply: MiniHostReply?) {
        if (pushing) return reply?.success() ?: Unit
        val url = attrs.optString("url").trim()
        if (url.isEmpty()) return reply?.fail("start:fail 没有推流地址（url）") ?: Unit
        val ctx = context ?: return reply?.fail("start:fail 宿主没有 Context") ?: Unit
        val wantMic = flag("enable-mic", true)
        val go = { micOk: Boolean ->
            if (pipe == null || !preview) {
                preview = true
                openCamera()
            }
            val p = pipe!!
            val (w, h) = pushSize()
            val fps = (attrs.optString("fps").toIntOrNull() ?: 15).coerceIn(1, 30)
            val kbps = (attrs.optString("max-bitrate").toIntOrNull() ?: 1000).coerceIn(100, 8000)
            val rate = if (attrs.optString("audio-quality").trim() == "low") 16000 else 48000
            val publisher = RtmpPublisher(url, object : RtmpPublisher.Listener {
                override fun onState(code: Int, message: String) = state(code, message)
                override fun onNetStatus(info: JSONObject) = emit("netstatus", JSONObject().put("info", info))
                override fun onCongested() { enc?.requestKeyFrame() }
            })
            publisher.videoWidth = w
            publisher.videoHeight = h
            publisher.fps = fps
            publisher.gopSeconds = 2
            val e = try {
                AvEncoder(w, h, fps, kbps * 1000, 2, if (micOk) AvEncoder.AudioCfg(rate, if (rate > 16000) 96_000 else 32_000) else null, publisher) { video, msg ->
                    Log.w(TAG, "📡 live-pusher：$msg")
                    if (video) state(-1303, "视频编码失败") else state(-1302, "打开麦克风失败")
                }
            } catch (ex: Exception) {
                state(-1303, "视频编码失败")
                reply?.fail("start:fail 编码器打不开：$ex")
                null
            }
            if (e != null) {
                e.micMuted = flag("muted", false)
                p.attachEncoder(e.surface, w, h, remoteMirror(p), fps) { ok ->
                    main.post {
                        if (!ok) {
                            e.stop()
                            state(-1303, "视频编码失败")
                            return@post reply?.fail("start:fail 编码器的输入画面建不起来") ?: Unit
                        }
                        enc = e
                        pub = publisher
                        pushing = true
                        pushPaused = false
                        e.start()
                        // 放着背景音：从现在起混进推流
                        bgm?.let { b -> e.bgm = b; b.mixRate = e.audioRate }
                        state(1008, "编码器启动")
                        if (micOk) state(2027, "麦克风启动成功")
                        publisher.start()
                        reply?.success()
                    }
                }
            }
        }
        if (!wantMic) return go(false)
        CameraPermissions.ensure(ctx, Manifest.permission.RECORD_AUDIO) { ok, why ->
            if (!ok) emit("error", JSONObject().put("errCode", 10002).put("errMsg", "用户禁止使用录音（$why）"))
            go(ok)
        }
    }

    private fun stop(reply: MiniHostReply?) {
        val e = enc
        val p = pub
        enc = null
        pub = null
        pushing = false
        bgm?.mixRate = 0
        e?.bgm = null
        p?.stop()
        if (e == null) return reply?.success() ?: Unit
        val done = { Thread({ e.stop(); main.post { reply?.success() } }, "mini-push-stop").start() }
        pipe?.detachEncoder { done() } ?: done()
    }

    private fun switchCamera(reply: MiniHostReply?) {
        front = !front
        val p = pipe ?: return reply?.success() ?: Unit
        val e = enc
        p.open(front, 1280, 16.0 / 9.0)
        p.previewMirror = previewMirror()
        // 重开相机会清掉编码器的输入（closeNow）：推着流时接回去
        if (e != null) p.attachEncoder(e.surface, e.width, e.height, remoteMirror(p), e.fps) { e.requestKeyFrame() }
        reply?.success()
    }

    override fun onContextCall(method: String, args: JSONObject, reply: MiniHostReply): Boolean {
        when (method) {
            "start" -> start(reply)
            "stop" -> stop(reply)
            "pause" -> {
                pushPaused = true
                enc?.micMuted = true
                pipe?.detachEncoder()
                reply.success()
            }
            "resume" -> {
                val p = pipe
                val e = enc
                pushPaused = false
                e?.micMuted = !flag("enable-mic", true) || flag("muted", false)
                if (p != null && e != null) p.attachEncoder(e.surface, e.width, e.height, remoteMirror(p), e.fps) { e.requestKeyFrame() }
                reply.success()
            }
            "startPreview" -> startPreview(reply)
            "stopPreview" -> {
                preview = false
                pipe?.close()
                reply.success()
            }
            "switchCamera" -> switchCamera(reply)
            "toggleTorch" -> {
                torch = !torch
                pipe?.torch(torch)
                reply.success()
            }
            "setZoom" -> pipe?.setZoom(args.optDouble("zoom", 1.0)) { z -> reply.success(JSONObject().put("zoom", z)) }
                ?: reply.fail("setZoom:fail 相机还没打开")
            "getMaxZoom" -> reply.success(JSONObject().put("maxZoom", pipe?.maxZoom() ?: 1.0))
            "playBGM" -> playBgm(args.optString("url"), reply)
            "pauseBGM" -> {
                bgm?.pause()
                reply.success()
            }
            "resumeBGM" -> {
                bgm?.resume()
                reply.success()
            }
            "stopBGM" -> {
                bgm?.stop()
                reply.success()
            }
            "setBGMVolume" -> {
                val v = args.optDouble("volume", 1.0).toFloat()
                (bgm ?: BgmPlayer(bgmListener).also { bgm = it }).volume = v
                reply.success()
            }
            "setMICVolume" -> {
                enc?.micVolume = args.optDouble("volume", 1.0).toFloat().coerceIn(0f, 1f)
                reply.success()
            }
            "snapshot" -> {
                val p = pipe ?: return reply.fail("snapshot:fail 相机还没打开").let { true }
                val dir = File(context?.cacheDir, "minirender-camera").apply { mkdirs() }
                val f = File(dir, "pusher_${System.currentTimeMillis()}.jpg")
                val side = if (args.optString("quality") == "compressed") 640 else 1280
                p.takePhoto(side, 85, p.previewMirror, f) { file, err ->
                    if (file == null) return@takePhoto reply.fail("snapshot:fail $err")
                    val o = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                    BitmapFactory.decodeFile(file.absolutePath, o)
                    reply.success(JSONObject().put("tempImagePath", "file://${file.absolutePath}").put("width", o.outWidth).put("height", o.outHeight))
                }
            }
            else -> return false
        }
        return true
    }

    override fun onPause() {
        if (paused || CameraPermissions.requesting) return
        paused = true
        bgmWasPlaying = bgm?.playing == true
        if (bgmWasPlaying) bgm?.pause()
        if (pushing) {
            state(5000, "推流：被挂起，小程序或微信被退后台时挂起推流")
            enc?.micMuted = true
            pipe?.detachEncoder()
        }
        pipe?.close()
    }

    override fun onResume() {
        if (!paused) return
        paused = false
        if (bgmWasPlaying) bgm?.resume()
        if (preview) openCamera()
        val p = pipe
        val e = enc
        if (pushing && !pushPaused && p != null && e != null) {
            e.micMuted = !flag("enable-mic", true) || flag("muted", false)
            p.attachEncoder(e.surface, e.width, e.height, remoteMirror(p), e.fps) { e.requestKeyFrame() }
        }
    }

    override fun onDetach() {
        bgm?.stop()
        bgm = null
        stop(null)
        pipe?.release()
        pipe = null
    }

    private val bgmListener = object : BgmPlayer.Listener {
        override fun onStart() = main.post { emit("bgmstart", JSONObject()) }.let { }
        override fun onProgress(progressMs: Long, durationMs: Long) =
            main.post { emit("bgmprogress", JSONObject().put("progress", progressMs).put("duration", durationMs)) }.let { }
        override fun onComplete() = main.post { emit("bgmcomplete", JSONObject()) }.let { }
        override fun onError(msg: String) = main.post {
            emit("error", JSONObject().put("errCode", 10003).put("errMsg", "背景音资源（BGM）加载失败：$msg"))
        }.let { }
    }

    /** playBGM：网络地址直接给解码器，小程序里的路径（包内 / wxfile://）换成本地文件 */
    private fun playBgm(url: String, reply: MiniHostReply) {
        val u = url.trim()
        val src = when {
            u.startsWith("http://") || u.startsWith("https://") -> u
            u.startsWith("file://") -> u.removePrefix("file://")
            else -> resolvePath(u)?.absolutePath
        } ?: return reply.fail("playBGM:fail 找不到背景音 $u").also {
            emit("error", JSONObject().put("errCode", 10003).put("errMsg", "背景音资源（BGM）加载失败"))
        }
        val b = bgm ?: BgmPlayer(bgmListener).also { bgm = it }
        enc?.let { e -> e.bgm = b; b.mixRate = e.audioRate }
        b.play(src)
        reply.success()
    }

    companion object {
        private const val TAG = "mini-render"
        val METHODS = listOf("start", "stop", "pause", "resume", "startPreview", "stopPreview", "switchCamera", "toggleTorch",
            "snapshot", "setZoom", "getMaxZoom", "setMICVolume", "playBGM", "pauseBGM", "resumeBGM", "stopBGM", "setBGMVolume")
    }
}
