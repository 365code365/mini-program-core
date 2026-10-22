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
        "scroll_log" to "MINI_SCROLL_LOG",
        "layout_log" to "MINI_LAYOUT_LOG",
        "input_log" to "MINI_INPUT_LOG",
        "backend" to "MINI_BACKEND",
        "no_resample" to "MINI_NO_RESAMPLE",
    )

    /** `debug.mini.fps` 开着：宿主这边也每秒打一行上屏耗时（[EngineHost.present]） */
    @Volatile var fps = false
        private set

    fun apply() {
        val props = try {
            Runtime.getRuntime().exec(arrayOf("getprop")).inputStream.bufferedReader().use { it.readText() }
        } catch (e: Exception) {
            return
        }
        // `[debug.mini.fps]: [1]`
        val re = Regex("""^\[debug\.mini\.([a-z_]+)]: \[(.*)]$""")
        for (line in props.lineSequence()) {
            val m = re.find(line.trim()) ?: continue
            val env = KEYS[m.groupValues[1]] ?: continue
            val v = m.groupValues[2]
            if (v.isEmpty()) continue
            Os.setenv(env, v, true)
            if (env == "MINI_FPS") fps = true
            Log.i("mini-render", "诊断开关 $env=$v（来自 debug.mini.${m.groupValues[1]}）")
        }
    }
}
