package dev.minirender.flutter

import android.system.Os
import android.util.Log

/**
 * 诊断开关的 Android 入口：`adb shell setprop debug.mini.fps 1` → 引擎创建前 `Os.setenv("MINI_FPS", "1")`。
 *
 * 引擎在创建时读一次进程环境（`HostEnv::from_process`），桌面上 `MINI_FPS=1 cargo run` 就行；
 * Android 的应用进程不继承 shell 环境，从前文档只写了「要在 nativeCreateEx 之前 Os.setenv」，
 * 插件里没人做 —— 真机上「滑起来顿」只能靠肉眼。系统属性 `debug.*` 不用 root 就能设，改完重开小程序生效。
 *
 * 只读 `getprop` 一次（一次 exec，几毫秒）：反射 `SystemProperties` 在新系统上会被隐藏 API 限制挡掉。
 */
internal object DiagProps {
    /** 属性名后缀 → 环境变量（与 README 的诊断开关表同一组） */
    private val KEYS = mapOf(
        "fps" to "MINI_FPS",
        "damage_log" to "MINI_DAMAGE_LOG",
        "no_damage" to "MINI_NO_DAMAGE",
        "no_style_memo" to "MINI_NO_STYLE_MEMO",
        "no_incremental_layout" to "MINI_NO_INCREMENTAL_LAYOUT",
        "no_scroll_tiles" to "MINI_NO_SCROLL_TILES",
        "layout_verify" to "MINI_LAYOUT_VERIFY",
        "mem_log" to "MINI_MEM_LOG",
        "scroll_log" to "MINI_SCROLL_LOG",
        "layout_log" to "MINI_LAYOUT_LOG",
        "input_log" to "MINI_INPUT_LOG",
        "backend" to "MINI_BACKEND",
        "no_resample" to "MINI_NO_RESAMPLE",
        "pace" to "MINI_PACE",
        "cpu_pin" to "MINI_CPU_PIN",
        "draw_log" to "MINI_DRAW_LOG",
        "img_log" to "MINI_IMG_LOG",
        "no_prefetch" to "MINI_NO_PREFETCH",
        "js_system_alloc" to "MINI_JS_SYSTEM_ALLOC",
        "no_js_cache" to "MINI_NO_JS_CACHE",
    )

    /** `debug.mini.fps` 开着：宿主这边也每秒打一行上屏耗时（[EngineHost.present]） */
    @Volatile var fps = false
        private set

    /**
     * `debug.mini.pace` 开着：引擎每交出一帧打一行 `🎞 <vsync µs> <交出 µs>`（System.nanoTime 那个钟）。
     * 与 Dart 侧 `FrameTiming` 的 rasterStart 对起来，就知道 Flutter 每个 vsync 取到的是引擎的哪一帧
     * （`tools/pace-check.py`）。帧耗时看着都达标、画面照样一顿一顿时，看的就是这个
     */
    @Volatile var pace = false
        private set

    /**
     * `debug.mini.native_lag <帧数>`：宿主承载的原生组件比引擎画面晚上屏几个 vsync（Dart 的
     * `MiniProgramController.nativeViewLag`）。不是引擎的环境变量，交给 Dart（textureInfo 的 `nativeLag`），
     * 真机校准滑动时视频框跟不跟页面用：改完重开小程序生效
     */
    @Volatile var nativeLag: Double? = null
        private set

    fun apply() {
        val props = try {
            Runtime.getRuntime().exec(arrayOf("getprop")).inputStream.bufferedReader().use { it.readText() }
        } catch (e: Exception) {
            return
        }
        // `[debug.mini.fps]: [1]`
        val re = Regex("""^\[debug\.mini\.([a-z_]+)]: \[(.*)]$""")
        nativeLag = null
        for (line in props.lineSequence()) {
            val m = re.find(line.trim()) ?: continue
            val v = m.groupValues[2]
            if (m.groupValues[1] == "native_lag") {
                // 负数要写成 `' -9'`（setprop 会把开头的 `-` 当成选项）：先去空白
                nativeLag = v.trim().toDoubleOrNull()
                if (nativeLag != null) Log.i("mini-render", "诊断开关 nativeLag=$v（来自 debug.mini.native_lag）")
                continue
            }
            val env = KEYS[m.groupValues[1]] ?: continue
            if (v.isEmpty()) continue
            Os.setenv(env, v, true)
            if (env == "MINI_FPS") fps = true
            if (env == "MINI_PACE") pace = true
            Log.i("mini-render", "诊断开关 $env=$v（来自 debug.mini.${m.groupValues[1]}）")
        }
    }
}
