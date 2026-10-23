package dev.minirender

import android.graphics.SurfaceTexture
import android.opengl.EGL14
import android.opengl.EGLConfig
import android.opengl.EGLContext
import android.opengl.EGLDisplay
import android.opengl.EGLSurface
import android.opengl.GLES11Ext
import android.opengl.GLES20
import android.os.Handler
import android.view.Surface
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.FloatBuffer

/**
 * 把系统播放器的画面读成 RGBA：`MediaPlayer` → [Surface]（`SurfaceTexture`）→ GLES 把外部纹理画进离屏 FBO
 * → `glReadPixels`。同层渲染的视频（[VideoHost]）用它把帧交给引擎。
 *
 * ## 为什么不用 ImageReader 直接拿 YUV
 * 硬解码器交给 `ImageReader(YUV_420_888)` 的缓冲在很多机型上是厂商私有格式（压缩过的），CPU 锁不上，
 * `Image.getPlanes()` 会在系统的 JNI 里直接 abort、整个进程闪退（一加 PLY110 上点了播放就退）。
 * `SurfaceTexture` 是解码器输出的标准去处，颜色转换（各种 YUV 矩阵、裁剪框）由 GPU 按解码器给的变换做好，
 * 读回来就是对的 RGBA。读回走一次 GPU→CPU（960×400 约 1~3ms），在 [VideoHost] 自己的线程上。
 *
 * 一个 EGL 上下文（这条线程上）可以带多个 [Target]（每个视频一个外部纹理 + FBO）。
 * **所有方法都在 [handler] 的线程上调**（GL 上下文只在这条线程上 current）。
 */
internal class GlFrameReader(private val handler: Handler) {
    private var display: EGLDisplay = EGL14.EGL_NO_DISPLAY
    private var context: EGLContext = EGL14.EGL_NO_CONTEXT
    private var pbuffer: EGLSurface = EGL14.EGL_NO_SURFACE
    private var program = 0
    private var aPos = 0
    private var aTex = 0
    private var uMatrix = 0

    /** 建上下文；失败（极少：没有 GLES 2）返回 false，同层渲染就出不了画面 */
    fun init(): Boolean {
        display = EGL14.eglGetDisplay(EGL14.EGL_DEFAULT_DISPLAY)
        val ver = IntArray(2)
        if (display == EGL14.EGL_NO_DISPLAY || !EGL14.eglInitialize(display, ver, 0, ver, 1)) return false
        val attrs = intArrayOf(
            EGL14.EGL_RED_SIZE, 8, EGL14.EGL_GREEN_SIZE, 8, EGL14.EGL_BLUE_SIZE, 8, EGL14.EGL_ALPHA_SIZE, 8,
            EGL14.EGL_RENDERABLE_TYPE, EGL14.EGL_OPENGL_ES2_BIT,
            EGL14.EGL_SURFACE_TYPE, EGL14.EGL_PBUFFER_BIT,
            EGL14.EGL_NONE,
        )
        val configs = arrayOfNulls<EGLConfig>(1)
        val n = IntArray(1)
        if (!EGL14.eglChooseConfig(display, attrs, 0, configs, 0, 1, n, 0) || n[0] == 0) return false
        val config = configs[0] ?: return false
        context = EGL14.eglCreateContext(
            display, config, EGL14.EGL_NO_CONTEXT, intArrayOf(EGL14.EGL_CONTEXT_CLIENT_VERSION, 2, EGL14.EGL_NONE), 0,
        )
        if (context == EGL14.EGL_NO_CONTEXT) return false
        pbuffer = EGL14.eglCreatePbufferSurface(display, config, intArrayOf(EGL14.EGL_WIDTH, 1, EGL14.EGL_HEIGHT, 1, EGL14.EGL_NONE), 0)
        if (pbuffer == EGL14.EGL_NO_SURFACE || !EGL14.eglMakeCurrent(display, pbuffer, pbuffer, context)) return false
        program = link(VERTEX, FRAGMENT)
        if (program == 0) return false
        aPos = GLES20.glGetAttribLocation(program, "aPos")
        aTex = GLES20.glGetAttribLocation(program, "aTex")
        uMatrix = GLES20.glGetUniformLocation(program, "uMatrix")
        return true
    }

    fun release() {
        if (display != EGL14.EGL_NO_DISPLAY) {
            if (program != 0) GLES20.glDeleteProgram(program)
            EGL14.eglMakeCurrent(display, EGL14.EGL_NO_SURFACE, EGL14.EGL_NO_SURFACE, EGL14.EGL_NO_CONTEXT)
            if (pbuffer != EGL14.EGL_NO_SURFACE) EGL14.eglDestroySurface(display, pbuffer)
            if (context != EGL14.EGL_NO_CONTEXT) EGL14.eglDestroyContext(display, context)
            EGL14.eglTerminate(display)
        }
        display = EGL14.EGL_NO_DISPLAY
        context = EGL14.EGL_NO_CONTEXT
        pbuffer = EGL14.EGL_NO_SURFACE
    }

    /** 一个视频：解码器往 [surface] 里交帧，[onFrame] 在有新帧时被调（[handler] 线程上） */
    inner class Target(onFrame: () -> Unit) {
        private val oes: Int
        private val texture: SurfaceTexture
        val surface: Surface
        private var fbo = 0
        private var fboTex = 0
        private var fw = 0
        private var fh = 0
        private var out: ByteBuffer? = null
        private val matrix = FloatArray(16)

        init {
            val t = IntArray(1)
            GLES20.glGenTextures(1, t, 0)
            oes = t[0]
            GLES20.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, oes)
            GLES20.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_MIN_FILTER, GLES20.GL_LINEAR)
            GLES20.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_MAG_FILTER, GLES20.GL_LINEAR)
            GLES20.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_WRAP_S, GLES20.GL_CLAMP_TO_EDGE)
            GLES20.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_WRAP_T, GLES20.GL_CLAMP_TO_EDGE)
            texture = SurfaceTexture(oes)
            texture.setOnFrameAvailableListener({ onFrame() }, handler)
            surface = Surface(texture)
        }

        /** 视频尺寸（解码器那边改尺寸时 SurfaceTexture 的缓冲跟着变，这里只决定读回多大） */
        fun setSize(w: Int, h: Int) {
            texture.setDefaultBufferSize(w, h)
        }

        /**
         * 把最新一帧画进 `w×h` 的 FBO 再读回，交给 [sink]（RGBA，紧密排列，**从上往下**）。
         * 画的时候整张上下翻过来：`glReadPixels` 的第 0 行是帧缓冲最底下那行，翻过来读出来就是正的
         */
        fun read(w: Int, h: Int, sink: (ByteBuffer) -> Unit) {
            texture.updateTexImage()
            texture.getTransformMatrix(matrix)
            ensureFbo(w, h)
            GLES20.glBindFramebuffer(GLES20.GL_FRAMEBUFFER, fbo)
            GLES20.glViewport(0, 0, w, h)
            GLES20.glUseProgram(program)
            GLES20.glActiveTexture(GLES20.GL_TEXTURE0)
            GLES20.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, oes)
            GLES20.glUniformMatrix4fv(uMatrix, 1, false, matrix, 0)
            GLES20.glEnableVertexAttribArray(aPos)
            GLES20.glVertexAttribPointer(aPos, 2, GLES20.GL_FLOAT, false, 0, QUAD_POS)
            GLES20.glEnableVertexAttribArray(aTex)
            GLES20.glVertexAttribPointer(aTex, 2, GLES20.GL_FLOAT, false, 0, QUAD_TEX)
            GLES20.glDrawArrays(GLES20.GL_TRIANGLE_STRIP, 0, 4)
            val buf = out!!.apply { clear() }
            GLES20.glReadPixels(0, 0, w, h, GLES20.GL_RGBA, GLES20.GL_UNSIGNED_BYTE, buf)
            GLES20.glBindFramebuffer(GLES20.GL_FRAMEBUFFER, 0)
            sink(buf)
        }

        private fun ensureFbo(w: Int, h: Int) {
            if (fbo != 0 && fw == w && fh == h) return
            freeFbo()
            val ids = IntArray(1)
            GLES20.glGenTextures(1, ids, 0)
            fboTex = ids[0]
            GLES20.glBindTexture(GLES20.GL_TEXTURE_2D, fboTex)
            GLES20.glTexImage2D(GLES20.GL_TEXTURE_2D, 0, GLES20.GL_RGBA, w, h, 0, GLES20.GL_RGBA, GLES20.GL_UNSIGNED_BYTE, null)
            GLES20.glTexParameteri(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_MIN_FILTER, GLES20.GL_LINEAR)
            GLES20.glGenFramebuffers(1, ids, 0)
            fbo = ids[0]
            GLES20.glBindFramebuffer(GLES20.GL_FRAMEBUFFER, fbo)
            GLES20.glFramebufferTexture2D(GLES20.GL_FRAMEBUFFER, GLES20.GL_COLOR_ATTACHMENT0, GLES20.GL_TEXTURE_2D, fboTex, 0)
            GLES20.glBindFramebuffer(GLES20.GL_FRAMEBUFFER, 0)
            fw = w
            fh = h
            out = ByteBuffer.allocateDirect(w * h * 4).order(ByteOrder.nativeOrder())
        }

        private fun freeFbo() {
            if (fbo != 0) GLES20.glDeleteFramebuffers(1, intArrayOf(fbo), 0)
            if (fboTex != 0) GLES20.glDeleteTextures(1, intArrayOf(fboTex), 0)
            fbo = 0
            fboTex = 0
        }

        fun release() {
            freeFbo()
            texture.setOnFrameAvailableListener(null)
            surface.release()
            texture.release()
            GLES20.glDeleteTextures(1, intArrayOf(oes), 0)
        }
    }

    private fun link(vs: String, fs: String): Int {
        fun compile(type: Int, src: String): Int {
            val s = GLES20.glCreateShader(type)
            GLES20.glShaderSource(s, src)
            GLES20.glCompileShader(s)
            val ok = IntArray(1)
            GLES20.glGetShaderiv(s, GLES20.GL_COMPILE_STATUS, ok, 0)
            if (ok[0] == 0) {
                GLES20.glDeleteShader(s)
                return 0
            }
            return s
        }
        val v = compile(GLES20.GL_VERTEX_SHADER, vs)
        val f = compile(GLES20.GL_FRAGMENT_SHADER, fs)
        if (v == 0 || f == 0) return 0
        val p = GLES20.glCreateProgram()
        GLES20.glAttachShader(p, v)
        GLES20.glAttachShader(p, f)
        GLES20.glLinkProgram(p)
        val ok = IntArray(1)
        GLES20.glGetProgramiv(p, GLES20.GL_LINK_STATUS, ok, 0)
        return if (ok[0] == 0) 0 else p
    }

    private companion object {
        const val VERTEX = """
            attribute vec4 aPos;
            attribute vec4 aTex;
            uniform mat4 uMatrix;
            varying vec2 vTex;
            void main() {
                gl_Position = aPos;
                vTex = (uMatrix * aTex).xy;
            }
        """
        const val FRAGMENT = """
            #extension GL_OES_EGL_image_external : require
            precision mediump float;
            uniform samplerExternalOES uTex;
            varying vec2 vTex;
            void main() { gl_FragColor = texture2D(uTex, vTex); }
        """

        fun floats(vararg v: Float): FloatBuffer =
            ByteBuffer.allocateDirect(v.size * 4).order(ByteOrder.nativeOrder()).asFloatBuffer().apply { put(v).position(0) }

        /** 上下翻过来的整屏四边形（见 [Target.read]） */
        val QUAD_POS = floats(-1f, 1f, 1f, 1f, -1f, -1f, 1f, -1f)
        val QUAD_TEX = floats(0f, 0f, 1f, 0f, 0f, 1f, 1f, 1f)
    }
}
