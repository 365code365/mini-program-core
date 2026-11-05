package dev.minirender

import android.opengl.GLES20
import org.json.JSONObject
import kotlin.math.min

/**
 * live-pusher 的美颜（`beauty` 0~9、`beauty-style` smooth / nature）、美白（`whiteness` 0~9）、色彩滤镜（`filter`）：
 * 一个片段着色器，直接从相机的外部纹理采样，预览读回与画进编码器（[GlFrameReader] 的两条路）都走它，推出去的流与预览一样。
 *
 * - 磨皮：16 个邻点按颜色相近程度加权（简化的双边滤波：边缘两侧颜色差得远，权重小，轮廓不糊），按 YCbCr 的肤色范围
 *   做一个软遮罩（smooth 遮罩外也磨一半，nature 只磨皮肤、强度七成）。半径随档位 1~4 像素（按 720p 折算）；
 * - 美白：对数曲线提亮（暗部提得多、高光不过曝），档位越高曲线越弯；
 * - 滤镜：每个名字一个 3×3 颜色矩阵 + 偏移。**颜色是按名字调的近似**：微信用的是腾讯云 SDK 的查找表，查不到，
 *   只保证「怀旧偏棕、蓝调偏蓝、樱红偏红」这类方向一致，不逐像素对齐。
 *
 * 全关（0 / 0 / standard）时 [Params.active] 为假，[GlFrameReader] 照旧走直通着色器：不开美颜的页面一个像素都不变。
 * 着色器在 GL 线程上第一次用到时才编（[program]），编不过（极少：老 GPU 不认 highp 之外的写法）就退回直通、打日志。
 */
internal class FrameEffects {
    /** 一组效果参数（属性解析好的，GL 线程上读） */
    class Params(val beauty: Float, val white: Float, val nature: Boolean, val filter: FloatArray?) {
        val active: Boolean get() = beauty > 0f || white > 0f || filter != null
    }

    private var prog = 0
    private var failed = false
    var aPos = 0
        private set
    var aTex = 0
        private set
    private var uMatrix = 0
    private var uStep = 0
    private var uBeauty = 0
    private var uNature = 0
    private var uWhite = 0
    private var uColor = 0
    private var uOffset = 0
    private var uFilter = 0

    /** 编好的着色器（GL 线程上调）；编不过返回 0 */
    fun program(link: (String, String) -> Int): Int {
        if (prog != 0 || failed) return prog
        prog = link(GlFrameReader.VERTEX, FRAGMENT)
        if (prog == 0) {
            failed = true
            android.util.Log.w("mini-render", "📡 美颜着色器编不过：美颜 / 美白 / 滤镜不生效")
            return 0
        }
        aPos = GLES20.glGetAttribLocation(prog, "aPos")
        aTex = GLES20.glGetAttribLocation(prog, "aTex")
        uMatrix = GLES20.glGetUniformLocation(prog, "uMatrix")
        uStep = GLES20.glGetUniformLocation(prog, "uStep")
        uBeauty = GLES20.glGetUniformLocation(prog, "uBeauty")
        uNature = GLES20.glGetUniformLocation(prog, "uNature")
        uWhite = GLES20.glGetUniformLocation(prog, "uWhite")
        uColor = GLES20.glGetUniformLocation(prog, "uColor")
        uOffset = GLES20.glGetUniformLocation(prog, "uOffset")
        uFilter = GLES20.glGetUniformLocation(prog, "uFilter")
        return prog
    }

    /** 用上这组参数（[program] 之后、画之前）。[bufW]×[bufH] 是相机缓冲的尺寸：磨皮半径按它换成纹理坐标 */
    fun use(p: Params, matrix: FloatArray, bufW: Int, bufH: Int) {
        GLES20.glUseProgram(prog)
        GLES20.glUniformMatrix4fv(uMatrix, 1, false, matrix, 0)
        val w = bufW.coerceAtLeast(1)
        val h = bufH.coerceAtLeast(1)
        val radius = (1f + p.beauty * 3f) * (min(w, h) / 720f).coerceIn(0.5f, 2f)
        GLES20.glUniform2f(uStep, radius / w, radius / h)
        GLES20.glUniform1f(uBeauty, p.beauty)
        GLES20.glUniform1f(uNature, if (p.nature) 1f else 0f)
        GLES20.glUniform1f(uWhite, p.white)
        val f = p.filter ?: IDENTITY
        GLES20.glUniformMatrix4fv(uColor, 1, false, f, 0)
        GLES20.glUniform4f(uOffset, f[16], f[17], f[18], 0f)
        GLES20.glUniform1f(uFilter, if (p.filter != null) 1f else 0f)
    }

    fun release() {
        if (prog != 0) GLES20.glDeleteProgram(prog)
        prog = 0
    }

    companion object {
        /** 属性 → 参数；全关返回 null（走直通着色器） */
        fun parse(attrs: JSONObject): Params? {
            fun level(k: String) = (attrs.optString(k).trim().toFloatOrNull() ?: 0f).coerceIn(0f, 9f) / 9f
            val p = Params(
                level("beauty"),
                level("whiteness"),
                attrs.optString("beauty-style").trim() == "nature",
                FILTERS[attrs.optString("filter").trim()],
            )
            return p.takeIf { it.active }
        }

        /** 3×3 矩阵（按行写）+ 偏移 → GL 的列主序 mat4（前 16 个）+ 偏移（后 3 个） */
        private fun m(r: FloatArray, off: FloatArray): FloatArray {
            val o = FloatArray(19)
            for (row in 0 until 3) for (col in 0 until 3) o[col * 4 + row] = r[row * 3 + col]
            o[15] = 1f
            o[16] = off[0]
            o[17] = off[1]
            o[18] = off[2]
            return o
        }

        private fun diag(r: Float, g: Float, b: Float, off: FloatArray) = m(floatArrayOf(r, 0f, 0f, 0f, g, 0f, 0f, 0f, b), off)

        /** 饱和度 [s]（1 不变）：(1 − s)·亮度 + s·原色 */
        private fun saturation(s: Float): FloatArray {
            val l = floatArrayOf(0.299f, 0.587f, 0.114f)
            return FloatArray(9) { i -> (1 - s) * l[i % 3] + if (i / 3 == i % 3) s else 0f }
        }

        private val IDENTITY = diag(1f, 1f, 1f, floatArrayOf(0f, 0f, 0f))

        /** 官方 filter 的合法值（standard 不调色）。方向见类注释 */
        val FILTERS: Map<String, FloatArray> = run {
            val sepia = floatArrayOf(0.393f, 0.769f, 0.189f, 0.349f, 0.686f, 0.168f, 0.272f, 0.534f, 0.131f)
            val nostalgia = FloatArray(9) { i -> 0.7f * sepia[i] + if (i / 3 == i % 3) 0.3f else 0f }
            val fresh = saturation(1.15f)
            val soft = saturation(0.85f)
            mapOf(
                "pink" to diag(1.05f, 0.97f, 1.02f, floatArrayOf(0.03f, 0f, 0.04f)),
                "nostalgia" to m(nostalgia, floatArrayOf(0f, 0f, 0f)),
                "blues" to m(floatArrayOf(0.9f, 0f, 0f, 0f, 0.98f, 0.02f, 0f, 0.05f, 1.1f), floatArrayOf(0f, 0.01f, 0.04f)),
                "romantic" to m(floatArrayOf(1.08f, 0.02f, 0f, 0f, 0.98f, 0f, 0.02f, 0f, 1.04f), floatArrayOf(0.02f, 0f, 0.03f)),
                "cool" to m(floatArrayOf(0.92f, 0f, 0f, 0f, 1f, 0.02f, 0f, 0.02f, 1.08f), floatArrayOf(0f, 0.02f, 0.05f)),
                "fresher" to m(fresh, floatArrayOf(0f, 0.02f, 0.02f)),
                "solor" to diag(0.87f, 0.9f, 0.92f, floatArrayOf(0.05f, 0.06f, 0.07f)),
                "aestheticism" to m(soft, floatArrayOf(0.04f, 0.04f, 0.04f)),
                "whitening" to diag(1.05f, 1.05f, 1.05f, floatArrayOf(0.05f, 0.05f, 0.05f)),
                "cerisered" to diag(1.12f, 0.96f, 0.98f, floatArrayOf(0.02f, 0f, 0.01f)),
            )
        }

        const val FRAGMENT = """
            #extension GL_OES_EGL_image_external : require
            #ifdef GL_FRAGMENT_PRECISION_HIGH
            precision highp float;
            #else
            precision mediump float;
            #endif
            uniform samplerExternalOES uTex;
            uniform vec2 uStep;
            uniform float uBeauty;
            uniform float uNature;
            uniform float uWhite;
            uniform mat4 uColor;
            uniform vec4 uOffset;
            uniform float uFilter;
            varying vec2 vTex;

            void tap(vec2 d, vec3 o, inout vec3 sum, inout float ws) {
                vec3 c = texture2D(uTex, vTex + d * uStep).rgb;
                vec3 e = c - o;
                float w = exp(-dot(e, e) * 60.0);
                sum += c * w;
                ws += w;
            }

            void main() {
                vec3 o = texture2D(uTex, vTex).rgb;
                if (uBeauty > 0.0) {
                    vec3 sum = o;
                    float ws = 1.0;
                    tap(vec2(1.0, 0.0), o, sum, ws); tap(vec2(-1.0, 0.0), o, sum, ws);
                    tap(vec2(0.0, 1.0), o, sum, ws); tap(vec2(0.0, -1.0), o, sum, ws);
                    tap(vec2(0.7, 0.7), o, sum, ws); tap(vec2(-0.7, 0.7), o, sum, ws);
                    tap(vec2(0.7, -0.7), o, sum, ws); tap(vec2(-0.7, -0.7), o, sum, ws);
                    tap(vec2(2.0, 0.0), o, sum, ws); tap(vec2(-2.0, 0.0), o, sum, ws);
                    tap(vec2(0.0, 2.0), o, sum, ws); tap(vec2(0.0, -2.0), o, sum, ws);
                    tap(vec2(1.4, 1.4), o, sum, ws); tap(vec2(-1.4, 1.4), o, sum, ws);
                    tap(vec2(1.4, -1.4), o, sum, ws); tap(vec2(-1.4, -1.4), o, sum, ws);
                    vec3 smooth = sum / ws;
                    float cb = -0.1687 * o.r - 0.3313 * o.g + 0.5 * o.b;
                    float cr = 0.5 * o.r - 0.4187 * o.g - 0.0813 * o.b;
                    float skin = smoothstep(0.0, 0.03, cr) * (1.0 - smoothstep(0.17, 0.22, cr))
                        * smoothstep(-0.24, -0.19, cb) * (1.0 - smoothstep(0.0, 0.04, cb));
                    float mask = mix(max(skin, 0.5), skin * 0.7, uNature);
                    o = mix(o, smooth, clamp(uBeauty * 1.2, 0.0, 1.0) * mask);
                }
                if (uWhite > 0.0) {
                    float beta = 1.0 + uWhite * 4.0;
                    o = log(o * (beta - 1.0) + 1.0) / log(beta);
                }
                if (uFilter > 0.0) {
                    o = clamp((uColor * vec4(o, 1.0)).rgb + uOffset.rgb, 0.0, 1.0);
                }
                gl_FragColor = vec4(o, 1.0);
            }
        """
    }
}
