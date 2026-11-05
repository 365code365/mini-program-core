package dev.minirender

import android.Manifest
import android.graphics.Canvas
import android.graphics.Color
import android.os.Handler
import android.os.Looper
import android.util.Log
import org.json.JSONObject
import java.io.File
import java.nio.ByteBuffer

/**
 * SDK 自带的 `<camera>`（官方 component/camera）与 `<voip-room mode="camera">`（自己那一格的画面），同层渲染进页面：
 * 相机画面每帧在 `mini-camera` 线程上按框裁好交给引擎（[CameraPipe]），滚动时就是页面里的一块像素。
 *
 * - 属性：`device-position`（camera 默认 back、voip-room 默认 front）、`flash`（auto / on / off / torch）、
 *   `resolution`（low / medium / high → 预览长边 640 / 1280 / 1920，只在打开时生效，同官方「不支持动态修改」）、`mode`（扫码由引擎在画面上解）；
 * - 事件：`initdone {maxZoom}`、`error {errMsg}`（没有权限 / 打不开）、`stop`（进后台、被别的应用占了）；
 * - CameraContext：`takePhoto`（预览那一帧存 JPEG，见 [CameraPipe] 为什么）、`startRecord` / `stopRecord`（H.264 + AAC 的 mp4，
 *   [AvEncoder] + [Mp4Sink]；`timeout` 默认 30 秒、最长 300 秒，到点自己停并推 `recordTimeout`，逻辑层再要一次结果交给 `timeoutCallback`）、`setZoom`。
 *   `onCameraFrame` 不经宿主：引擎从交上去的画面里取（host/engine_camera.rs）。
 * - `<voip-room mode="video" openid>`（别人的画面）：向宿主的 [MiniVoip.video] 要这个人的画面（宿主自己的通话后台）；
 *   没接的宿主只画黑底，`binderror`（进不了房）由引擎派。
 */
class MiniCameraView(private val tag: String = "camera") : MiniSameLayerView() {
    private val main = Handler(Looper.getMainLooper())
    private var pipe: CameraPipe? = null
    private var front = tag == "voip-room"
    private var side = 1280
    private var flash = "auto"
    private var paused = false
    private var wantOpen = false

    override val contextClass: String get() = if (tag == "camera") "CameraContext" else ""
    override val contextMethods: List<String> get() = if (tag == "camera") METHODS else emptyList()

    private val selfOnly get() = tag == "voip-room" && attrs.optString("mode").trim() == "video"

    override fun onDraw(canvas: Canvas, width: Int, height: Int, density: Float) {
        // 第一帧画面到之前、voip-room 别人的那一格：黑底（与微信的原生相机一致）
        canvas.drawColor(Color.BLACK)
    }

    override fun onBox(w: Int, h: Int) {
        pipe?.setBox(w, h)
        if (selfOnly) attachRemote()
    }

    /** voip-room 别人那一格：正在接的那一路（openid、宽、高）与它的句柄 */
    private var remote: Triple<String, Int, Int>? = null
    private var remoteHandle: AutoCloseable? = null

    /** 按 openid 与框的大小向宿主要画面（变了才重接） */
    private fun attachRemote() {
        val src = MiniVoip.video ?: return
        val id = attrs.optString("openid").trim()
        val want = Triple(id, boxWidth, boxHeight)
        if (want == remote) return
        detachRemote()
        if (id.isEmpty() || boxWidth <= 0 || boxHeight <= 0) return
        remote = want
        remoteHandle = try {
            src.attach(id, boxWidth, boxHeight) { buf, w, h -> pushFrame(w, h, buf) }
        } catch (e: Exception) {
            Log.w(TAG, "📞 voip-room：宿主接不上 $id 的画面 $e")
            null
        }
    }

    private fun detachRemote() {
        try {
            remoteHandle?.close()
        } catch (_: Exception) {
        }
        remoteHandle = null
        remote = null
    }

    override fun onAttrs(attrs: JSONObject) {
        invalidate()
        if (selfOnly) return attachRemote()
        val pos = attrs.optString("device-position").trim()
        val f = if (pos.isEmpty()) tag == "voip-room" else pos == "front"
        val fl = attrs.optString("flash").trim().ifEmpty { "auto" }
        val first = !wantOpen
        if (first) {
            side = when (attrs.optString("resolution").trim()) {
                "low" -> 640
                "high" -> 1920
                else -> 1280
            }
            front = f
            flash = fl
            wantOpen = true
            ensureAndOpen()
            return
        }
        if (f != front) {
            front = f
            open()
        }
        if (fl != flash) {
            flash = fl
            pipe?.setFlash(fl)
        }
    }

    private fun ensureAndOpen() {
        val ctx = context ?: return
        CameraPermissions.ensure(ctx, Manifest.permission.CAMERA) { ok, why ->
            if (!ok) {
                emit("error", JSONObject().put("errMsg", "$tag:fail auth deny（$why）"))
                return@ensure
            }
            open()
        }
    }

    private fun open() {
        val ctx = context ?: return
        if (paused) return
        val p = pipe ?: CameraPipe(ctx, listener).also { pipe = it }
        p.setBox(boxWidth, boxHeight)
        p.setFlash(flash)
        p.open(front, side, 4.0 / 3.0)
    }

    private val listener = object : CameraPipe.Listener {
        override fun onOpened(maxZoom: Double) {
            if (tag == "camera") emit("initdone", JSONObject().put("maxZoom", maxZoom))
        }

        override fun onError(msg: String, denied: Boolean) {
            Log.w(TAG, "📷 $key：$msg")
            emit("error", JSONObject().put("errMsg", "$tag:fail $msg"))
        }

        override fun onFirstFrame() {}

        override fun onPreview(buf: ByteBuffer, w: Int, h: Int) = pushFrame(w, h, buf)
    }

    override fun onPause() {
        if (paused || CameraPermissions.requesting) return
        paused = true
        val p = pipe ?: return
        if (!p.isOpen) return
        recorder?.let { finishRecord(null) }
        p.close()
        // 官方 bindstop：摄像头在非正常终止时触发，如退出后台
        emit("stop", JSONObject())
    }

    override fun onResume() {
        if (!paused) return
        paused = false
        if (wantOpen && !selfOnly) open()
    }

    override fun onDetach() {
        detachRemote()
        recorder?.let { finishRecord(null) }
        pipe?.release()
        pipe = null
    }

    // ───────── CameraContext ─────────

    private class Recording(val enc: AvEncoder, val sink: Mp4Sink, val video: File, val thumb: File, val timeout: Runnable, val mirror: Boolean)
    private var recorder: Recording? = null
    /** 录到上限自己停下的那一份，等逻辑层来要（stopRecord） */
    private var finished: JSONObject? = null

    override fun onContextCall(method: String, args: JSONObject, reply: MiniHostReply): Boolean {
        val p = pipe
        when (method) {
            "takePhoto" -> {
                if (p == null) return reply.fail("takePhoto:fail 相机还没打开").let { true }
                // 微信：扫码模式下不让拍照（真机回的就是这句）
                if (attrs.optString("mode").trim() == "scanCode") return reply.fail("takePhoto:fail Not allow to invoke takePhoto in 'scanCode' mode.").let { true }
                val (s, q) = CameraPipe.QUALITY_SIDE[args.optString("quality", "normal")] ?: CameraPipe.QUALITY_SIDE["normal"]!!
                val mirror = CameraPipe.extraMirror(p.front, p.front && args.optBoolean("selfieMirror", true))
                p.takePhoto(s, q, mirror, newFile("jpg")) { f, err ->
                    if (f != null) reply.success(JSONObject().put("tempImagePath", "file://${f.absolutePath}"))
                    else reply.fail("takePhoto:fail $err")
                }
            }
            "setZoom" -> {
                if (p == null) return reply.fail("setZoom:fail 相机还没打开").let { true }
                p.setZoom(args.optDouble("zoom", 1.0)) { z -> reply.success(JSONObject().put("zoom", z)) }
            }
            "startRecord" -> startRecord(args, reply)
            "stopRecord" -> stopRecord(reply)
            else -> return false
        }
        return true
    }

    private fun startRecord(args: JSONObject, reply: MiniHostReply) {
        val p = pipe ?: return reply.fail("startRecord:fail 相机还没打开")
        if (recorder != null) return reply.fail("startRecord:fail 已经在录像")
        val ctx = context ?: return reply.fail("startRecord:fail 宿主没有 Context")
        CameraPermissions.ensure(ctx, Manifest.permission.RECORD_AUDIO) { micOk, _ ->
            val fw = p.frameW
            val fh = p.frameH
            if (fw <= 0) return@ensure reply.fail("startRecord:fail 相机还没出画面")
            // 编码尺寸：摆正后的画面，长边不超过 1280，宽高取偶数（编码器的要求）
            val s = minOf(1f, 1280f / maxOf(fw, fh))
            val w = ((fw * s).toInt() and 1.inv()).coerceAtLeast(2)
            val h = ((fh * s).toInt() and 1.inv()).coerceAtLeast(2)
            val video = newFile("mp4")
            val thumb = newFile("jpg")
            val sink = Mp4Sink(video, micOk)
            val enc = try {
                AvEncoder(w, h, 30, (w * h * 30 * 0.12).toInt(), 1, if (micOk) AvEncoder.AudioCfg(44100, 96_000) else null, sink) { isVideo, msg ->
                    Log.w(TAG, "📷 录像：$msg")
                    if (!isVideo) sink.audioFailed()
                }
            } catch (e: Exception) {
                return@ensure reply.fail("startRecord:fail 编码器打不开：$e")
            }
            val mirror = CameraPipe.extraMirror(p.front, p.front && args.optBoolean("selfieMirror", true))
            p.attachEncoder(enc.surface, w, h, mirror) { ok ->
                main.post {
                    if (!ok) {
                        enc.stop()
                        return@post reply.fail("startRecord:fail 编码器的输入画面建不起来")
                    }
                    enc.start()
                    val limit = args.optDouble("timeout", 30.0).coerceIn(1.0, 300.0)
                    val timeout = Runnable {
                        finishRecord { res ->
                            finished = res
                            dispatchContextEvent(JSONObject().put("context", "CameraContext").put("contextId", "").put("event", "recordTimeout").put("data", JSONObject()))
                        }
                    }
                    recorder = Recording(enc, sink, video, thumb, timeout, mirror)
                    main.postDelayed(timeout, (limit * 1000).toLong())
                    reply.success()
                }
            }
        }
    }

    private fun stopRecord(reply: MiniHostReply) {
        finished?.let {
            finished = null
            return reply.success(it)
        }
        if (recorder == null) return reply.fail("stopRecord:fail 没有在录像")
        finishRecord { res -> if (res != null) reply.success(res) else reply.fail("stopRecord:fail 录像没有写出来") }
    }

    /** 停编码器、收尾文件（后台线程上，编码器要等最后几帧），[done] 在主线程上回 */
    private fun finishRecord(done: ((JSONObject?) -> Unit)?) {
        val r = recorder ?: return
        recorder = null
        main.removeCallbacks(r.timeout)
        val p = pipe ?: return main.post { done?.invoke(null) }.let { }
        // 封面：停之前取当前这一帧（与录像同一个方向 / 镜像），拍到了再停编码器
        // 相机这时没在出帧（正关着）就等不到那一帧：最多等 800ms，没封面也照样收尾
        val once = java.util.concurrent.atomic.AtomicBoolean(false)
        val go = Runnable { if (once.compareAndSet(false, true)) stopEncoder(r, done) }
        p.takePhoto(480, 75, r.mirror, r.thumb) { _, _ -> main.post(go) }
        main.postDelayed(go, 800)
    }

    private fun stopEncoder(r: Recording, done: ((JSONObject?) -> Unit)?) {
        pipe?.detachEncoder {
            Thread({
                r.enc.stop()
                val ok = r.sink.finish()
                val res = if (!ok) null else JSONObject()
                    .put("tempVideoPath", "file://${r.video.absolutePath}")
                    .put("tempThumbPath", if (r.thumb.length() > 0) "file://${r.thumb.absolutePath}" else "")
                main.post { done?.invoke(res) }
            }, "mini-rec-stop").start()
        } ?: main.post { done?.invoke(null) }
    }

    private fun newFile(ext: String): File {
        val dir = File(context?.cacheDir, "minirender-camera").apply { mkdirs() }
        return File(dir, "${tag}_${System.currentTimeMillis()}_${(Math.random() * 1e6).toInt()}.$ext")
    }

    companion object {
        private const val TAG = "mini-render"
        /** 交给宿主的 CameraContext 方法（`onCameraFrame` 在引擎里，不在这张表） */
        val METHODS = listOf("takePhoto", "setZoom", "startRecord", "stopRecord")
    }
}
