package dev.minirender

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Rect
import android.graphics.SurfaceTexture
import android.hardware.camera2.CameraAccessException
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CameraMetadata
import android.hardware.camera2.CaptureRequest
import android.hardware.camera2.CaptureResult
import android.hardware.camera2.TotalCaptureResult
import android.opengl.EGL14
import android.opengl.EGLSurface
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.util.Log
import android.util.Size
import android.view.Surface
import java.io.File
import java.io.FileOutputStream
import java.nio.ByteBuffer
import kotlin.math.abs
import kotlin.math.ln
import kotlin.math.max
import kotlin.math.roundToInt

/**
 * 相机的一路画面（`<camera>` / `<live-pusher>` / `<voip-room mode="camera">` 共用）：Camera2 → `SurfaceTexture`
 * （[GlFrameReader]）→ 每一帧在自己的 `mini-camera` 线程上：
 * 1. 按引擎里那个框的设备像素尺寸 cover 裁好读回（[Listener.onPreview]，调用方交给引擎 `nativeViewFrameRgba`）；
 * 2. 录像 / 推流时再画一遍进编码器的输入 Surface（[attachEncoder]，按编码尺寸 cover 裁）；
 * 3. 拍照时按画面原尺寸读回一张存 JPEG（[takePhoto]）。
 *
 * ## 方向与镜像
 * Camera2 往 `SurfaceTexture` 交帧时，相机服务按传感器方向（以竖屏为自然方向）与前后置设了缓冲变换：
 * `getTransformMatrix` 已经把画面摆正，前置还左右镜像（像照镜子）。所以「摆正后的画面」宽高 = 传感器方向 90 / 270 时缓冲的高 × 宽，
 * 要不要再镜像一次按 `front xor 想要镜像`（[extraMirror]）。设备横过来时不另转（小程序页面是竖屏）。
 *
 * ## 为什么拍照不走 `ImageReader(JPEG)`
 * 静态拍照要另开一路输出、重配会话（切换时预览会停一下），拍出来还要按 EXIF 转向、前置再镜像。这里直接取预览那一帧
 * （最大 1920×1080，[QUALITY_SIDE]），方向、镜像与屏上看到的一致；`flash="on"`（或 auto 且测光说要闪）时先开手电 [FLASH_MS] 再取。
 *
 * **所有 Camera2 回调、GL 调用都在 [handler] 线程上**；对外的方法可以在任何线程调（投递过来）。
 */
internal class CameraPipe(private val ctx: Context, private val listener: Listener) {
    interface Listener {
        /** 相机开好了（[maxZoom]：可用的最大变焦倍数） */
        fun onOpened(maxZoom: Double)
        /** 打不开 / 中途断了（[denied]：没有权限） */
        fun onError(msg: String, denied: Boolean)
        /** 第一帧画面到了 */
        fun onFirstFrame()
        /** 一帧预览（`w×h` RGBA，从上往下；这条线程上，用完就还） */
        fun onPreview(buf: ByteBuffer, w: Int, h: Int)
    }

    private val thread = HandlerThread("mini-camera").apply { start() }
    val handler = Handler(thread.looper)
    private val gl = GlFrameReader(handler)
    private var target: GlFrameReader.Target? = null

    private var device: CameraDevice? = null
    private var session: CameraCaptureSession? = null
    private var request: CaptureRequest.Builder? = null
    private var chars: CameraCharacteristics? = null
    private var released = false
    private var opening = false

    var front = false
        private set
    /** 摆正之后的画面尺寸 */
    var frameW = 0
        private set
    var frameH = 0
        private set
    private var firstFrame = true

    /** 引擎里框的设备像素尺寸、预览要不要再镜像 */
    @Volatile private var boxW = 0
    @Volatile private var boxH = 0
    @Volatile var previewMirror = false
    @Volatile var previewEnabled = true

    private var flash = "auto"
    private var zoom = 1.0
    private var maxZoom = 1.0
    private var aeFlashRequired = false

    private class Enc(val surface: Surface, val egl: EGLSurface, val w: Int, val h: Int, val mirror: Boolean, val fps: Int) {
        var lastNs = 0L
    }
    private var enc: Enc? = null

    private class Photo(val side: Int, val quality: Int, val mirror: Boolean, val file: File, val done: (File?, String?) -> Unit)
    private val photos = ArrayList<Photo>()

    init {
        handler.post {
            if (gl.init()) {
                target = gl.Target { onFrame() }
            } else {
                Log.w(TAG, "📷 建不了 GLES 上下文：相机出不了画面")
            }
        }
    }

    fun setBox(w: Int, h: Int) {
        boxW = w
        boxH = h
    }

    /** 美颜 / 美白 / 滤镜（live-pusher，[FrameEffects]）：预览、推流、snapshot 都带上；null 关掉 */
    fun setEffects(p: FrameEffects.Params?) {
        gl.effects = p
    }

    /**
     * 开相机：[facingFront] 前后置；[side] 预览的长边（resolution：low 640 / medium 1280 / high 1920）；
     * [aspect]：要的传感器画面长短边比（相机 4:3、推流 16:9）。不按框的比例挑：那样会挑到 1:1 这类少见的输出，
     * 拍出来的照片也成了方的；框与画面比例不同时 cover 裁
     */
    @SuppressLint("MissingPermission")
    fun open(facingFront: Boolean, side: Int, aspect: Double = 4.0 / 3.0) = handler.post {
        if (released) return@post
        // 重开（换前后置、回前台）：编码器的输入留着，录像 / 推流不断
        closeNow(keepEncoder = true)
        front = facingFront
        val t = target ?: return@post listener.onError("相机画面通道没建起来（GLES）", false)
        val mgr = ctx.getSystemService(Context.CAMERA_SERVICE) as CameraManager
        val want = if (facingFront) CameraCharacteristics.LENS_FACING_FRONT else CameraCharacteristics.LENS_FACING_BACK
        val id = try {
            mgr.cameraIdList.firstOrNull { mgr.getCameraCharacteristics(it).get(CameraCharacteristics.LENS_FACING) == want }
                ?: mgr.cameraIdList.firstOrNull()
        } catch (e: Exception) {
            null
        } ?: return@post listener.onError("没有可用的摄像头", false)
        val c = mgr.getCameraCharacteristics(id)
        chars = c
        front = c.get(CameraCharacteristics.LENS_FACING) == CameraCharacteristics.LENS_FACING_FRONT
        val sensor = c.get(CameraCharacteristics.SENSOR_ORIENTATION) ?: 90
        val sizes = c.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP)?.getOutputSizes(SurfaceTexture::class.java)
            ?: return@post listener.onError("摄像头没有可用的预览尺寸", false)
        val rotated = sensor % 180 == 90
        val size = pickSize(sizes, side, aspect, rotated)
        t.setSize(size.width, size.height)
        frameW = if (rotated) size.height else size.width
        frameH = if (rotated) size.width else size.height
        maxZoom = maxZoomOf(c)
        firstFrame = true
        opening = true
        try {
            mgr.openCamera(id, object : CameraDevice.StateCallback() {
                override fun onOpened(d: CameraDevice) {
                    opening = false
                    if (released) return d.close()
                    device = d
                    startSession(d, t)
                }

                override fun onDisconnected(d: CameraDevice) {
                    opening = false
                    d.close()
                    if (device === d) device = null
                    listener.onError("摄像头被别的应用占用或断开了", false)
                }

                override fun onError(d: CameraDevice, error: Int) {
                    opening = false
                    d.close()
                    if (device === d) device = null
                    listener.onError("打开摄像头失败（$error）", false)
                }
            }, handler)
        } catch (e: SecurityException) {
            opening = false
            listener.onError("没有相机权限", true)
        } catch (e: Exception) {
            opening = false
            listener.onError("打开摄像头失败：$e", false)
        }
    }

    private fun startSession(d: CameraDevice, t: GlFrameReader.Target) {
        try {
            val b = d.createCaptureRequest(CameraDevice.TEMPLATE_RECORD)
            b.addTarget(t.surface)
            b.set(CaptureRequest.CONTROL_AF_MODE, CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_VIDEO)
            request = b
            @Suppress("DEPRECATION")
            d.createCaptureSession(listOf(t.surface), object : CameraCaptureSession.StateCallback() {
                override fun onConfigured(s: CameraCaptureSession) {
                    if (released || device !== d) return s.close()
                    session = s
                    applyRequest()
                    listener.onOpened(maxZoom)
                }

                override fun onConfigureFailed(s: CameraCaptureSession) {
                    listener.onError("相机会话配置失败", false)
                }
            }, handler)
        } catch (e: CameraAccessException) {
            listener.onError("打开摄像头失败：$e", false)
        }
    }

    private val results = object : CameraCaptureSession.CaptureCallback() {
        override fun onCaptureCompleted(s: CameraCaptureSession, r: CaptureRequest, result: TotalCaptureResult) {
            aeFlashRequired = result.get(CaptureResult.CONTROL_AE_STATE) == CaptureResult.CONTROL_AE_STATE_FLASH_REQUIRED
        }
    }

    /** 闪光 / 手电 / 变焦落到重复请求上 */
    private fun applyRequest(torch: Boolean = flash == "torch") {
        val b = request ?: return
        val s = session ?: return
        val hasFlash = chars?.get(CameraCharacteristics.FLASH_INFO_AVAILABLE) == true
        b.set(CaptureRequest.CONTROL_AE_MODE, CaptureRequest.CONTROL_AE_MODE_ON)
        b.set(CaptureRequest.FLASH_MODE, if (torch && hasFlash) CameraMetadata.FLASH_MODE_TORCH else CameraMetadata.FLASH_MODE_OFF)
        applyZoom(b)
        try {
            s.setRepeatingRequest(b.build(), results, handler)
        } catch (e: Exception) {
            Log.w(TAG, "📷 更新相机参数失败 $e")
        }
    }

    private fun applyZoom(b: CaptureRequest.Builder) {
        val z = zoom.coerceIn(1.0, maxZoom)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R && chars?.get(CameraCharacteristics.CONTROL_ZOOM_RATIO_RANGE) != null) {
            b.set(CaptureRequest.CONTROL_ZOOM_RATIO, z.toFloat())
            return
        }
        val a = chars?.get(CameraCharacteristics.SENSOR_INFO_ACTIVE_ARRAY_SIZE) ?: return
        val w = (a.width() / z).roundToInt()
        val h = (a.height() / z).roundToInt()
        val l = a.left + (a.width() - w) / 2
        val tp = a.top + (a.height() - h) / 2
        b.set(CaptureRequest.SCALER_CROP_REGION, Rect(l, tp, l + w, tp + h))
    }

    fun setFlash(mode: String) = handler.post {
        flash = mode
        applyRequest()
    }

    /** 返回实际用上的倍数（夹在 1 ~ maxZoom） */
    fun setZoom(z: Double, done: (Double) -> Unit) = handler.post {
        zoom = z.coerceIn(1.0, maxZoom)
        applyRequest()
        done(zoom)
    }

    fun maxZoom(): Double = maxZoom

    fun torch(on: Boolean) = handler.post {
        flash = if (on) "torch" else "off"
        applyRequest()
    }

    /** 新的一帧（这条线程上） */
    private fun onFrame() {
        val t = target ?: return
        if (released) return
        try {
            t.update()
        } catch (e: Exception) {
            return
        }
        if (frameW <= 0) return
        if (firstFrame) {
            firstFrame = false
            listener.onFirstFrame()
        }
        val bw = boxW
        val bh = boxH
        if (previewEnabled && bw > 0 && bh > 0) {
            val crop = GlFrameReader.coverCrop(frameW, frameH, bw, bh, previewMirror)
            try {
                t.readLatest(bw, bh, crop) { buf -> listener.onPreview(buf, bw, bh) }
            } catch (e: Exception) {
                Log.w(TAG, "📷 读帧失败 $e")
            }
        }
        enc?.let { e ->
            // 相机按自己的帧率（多半 30）交帧：编码按要的帧率取（live-pusher 的 fps 默认 15），多出来的不画进编码器。
            // 不取的话推出去的流实际 30 帧、元数据写 15，收端按 15 算时间戳会重
            val now = System.nanoTime()
            // 时间戳至少隔一个帧间隔（相机交帧有抖动，30 帧里隔一帧取时偶尔两帧只差 50ms）：收端按恒定帧率算的话不重
            val interval = 1_000_000_000L / e.fps
            if (now - e.lastNs >= interval * 9 / 10) {
                val pts = if (e.lastNs == 0L) now else maxOf(now, e.lastNs + interval)
                e.lastNs = pts
                gl.drawTo(t, e.egl, e.w, e.h, GlFrameReader.coverCrop(frameW, frameH, e.w, e.h, e.mirror), pts)
            }
        }
        if (photos.isNotEmpty()) {
            val list = ArrayList(photos)
            photos.clear()
            for (p in list) shoot(t, p)
        }
    }

    private fun shoot(t: GlFrameReader.Target, p: Photo) {
        val s = minOf(1f, p.side.toFloat() / max(frameW, frameH))
        val w = max(2, (frameW * s).roundToInt())
        val h = max(2, (frameH * s).roundToInt())
        val bmp = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
        try {
            t.readLatest(w, h, GlFrameReader.cropMatrix(1f, 1f, 0f, 0f, p.mirror)) { buf ->
                buf.rewind()
                bmp.copyPixelsFromBuffer(buf)
            }
        } catch (e: Exception) {
            bmp.recycle()
            return p.done(null, "读帧失败：$e")
        }
        // JPEG 压缩不占相机线程
        Thread({
            try {
                FileOutputStream(p.file).use { bmp.compress(Bitmap.CompressFormat.JPEG, p.quality, it) }
                p.done(p.file, null)
            } catch (e: Exception) {
                p.done(null, "$e")
            } finally {
                bmp.recycle()
            }
        }, "mini-photo").start()
    }

    /**
     * 拍一张：[side] 长边上限、[quality] JPEG 质量、[mirror] 在摆正的画面上再左右翻（[extraMirror] 算好的）。
     * 要闪光（flash=on，或 auto 且测光说要闪）时先开手电 [FLASH_MS] 再取那一帧
     */
    fun takePhoto(side: Int, quality: Int, mirror: Boolean, file: File, done: (File?, String?) -> Unit) = handler.post {
        if (session == null) return@post done(null, "相机还没开好")
        val p = Photo(side, quality, mirror, file, done)
        val hasFlash = chars?.get(CameraCharacteristics.FLASH_INFO_AVAILABLE) == true
        val needFlash = hasFlash && !front && (flash == "on" || (flash == "auto" && aeFlashRequired))
        if (!needFlash) {
            photos += p
            return@post
        }
        applyRequest(torch = true)
        handler.postDelayed({
            photos += p
            handler.postDelayed({ applyRequest() }, 120)
        }, FLASH_MS)
    }

    /** 录像 / 推流：编码器的输入 Surface，每帧按 `w×h` cover 裁好画进去 */
    fun attachEncoder(s: Surface, w: Int, h: Int, mirror: Boolean, fps: Int = 30, done: (Boolean) -> Unit) = handler.post {
        detachNow()
        val egl = gl.windowSurface(s)
        if (egl == EGL14.EGL_NO_SURFACE) return@post done(false)
        enc = Enc(s, egl, w, h, mirror, fps.coerceIn(1, 60))
        done(true)
    }

    fun detachEncoder(done: () -> Unit = {}) = handler.post {
        detachNow()
        done()
    }

    private fun detachNow() {
        enc?.let { gl.destroySurface(it.egl) }
        enc = null
    }

    fun close() = handler.post { closeNow() }

    private fun closeNow(keepEncoder: Boolean = false) {
        if (!keepEncoder) detachNow()
        try {
            session?.close()
        } catch (_: Exception) {
        }
        session = null
        request = null
        device?.close()
        device = null
    }

    val isOpen: Boolean get() = device != null || opening

    fun release() {
        released = true
        handler.post {
            closeNow()
            photos.forEach { it.done(null, "相机已关闭") }
            photos.clear()
            target?.release()
            target = null
            gl.release()
            thread.quitSafely()
        }
    }

    internal companion object {
        const val TAG = "mini-render"
        const val FLASH_MS = 450L
        /** takePhoto 的 quality → (长边上限, JPEG 质量) */
        val QUALITY_SIDE = mapOf("original" to (4096 to 95), "high" to (1920 to 90), "normal" to (1280 to 85), "low" to (640 to 70))

        /** 前置的画面本来就是镜像的：想要的镜像与之不同时再翻一次 */
        fun extraMirror(front: Boolean, wantMirrored: Boolean) = front != wantMirrored

        /**
         * 选预览尺寸：长短边比最贴近 [want]（`log` 比值差最小，按 0.1 分档），同一档里长边最接近 [side]；长边不超过 [side] 的优先
         */
        @Suppress("UNUSED_PARAMETER")
        fun pickSize(sizes: Array<Size>, side: Int, want: Double, rotated: Boolean): Size {
            fun ratio(s: Size) = max(s.width, s.height).toDouble() / minOf(s.width, s.height)
            val fits = sizes.filter { max(it.width, it.height) <= side }.ifEmpty { sizes.toList() }
            return fits.minWithOrNull(compareBy<Size>({ (abs(ln(ratio(it) / want)) * 10).roundToInt() }, { abs(max(it.width, it.height) - side) }))
                ?: sizes.first()
        }

        fun maxZoomOf(c: CameraCharacteristics): Double {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                c.get(CameraCharacteristics.CONTROL_ZOOM_RATIO_RANGE)?.let { return it.upper.toDouble() }
            }
            return (c.get(CameraCharacteristics.SCALER_AVAILABLE_MAX_DIGITAL_ZOOM) ?: 1f).toDouble()
        }
    }
}
