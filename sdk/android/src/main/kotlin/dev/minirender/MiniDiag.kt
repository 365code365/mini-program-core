package dev.minirender

import android.system.Os
import android.util.Log

/**
 * 诊断开关的原生 SDK 入口：`adb shell setprop debug.mini.damage_log 1` → 建引擎前 `Os.setenv("MINI_DAMAGE_LOG", "1")`。
 *
 * 引擎在创建时读一次进程环境（`HostEnv::from_process`），而 Android 的应用进程不继承 shell 环境：
 * 没有这一步，原生示例上 `MINI_FPS` / `MINI_DAMAGE_LOG` 全都开不了，「滑起来顿」只能靠肉眼。
 * 系统属性 `debug.*` 不用 root 就能设，改完重开小程序生效。
 *
 * 与 Flutter 插件的 `dev.minirender.flutter.DiagProps` 是同一张表（那边还多两个只给 Dart 用的开关）。
 * 只读 `getprop` 一次（一次 exec，几毫秒）：反射 `SystemProperties` 在新系统上会被隐藏 API 限制挡掉。
 */
internal object MiniDiag {
    /** 属性名后缀 → 环境变量（与 README 的诊断开关表同一组） */
    private val KEYS = mapOf(
        "fps" to "MINI_FPS",
        "damage_log" to "MINI_DAMAGE_LOG",
        "no_damage" to "MINI_NO_DAMAGE",
        "no_style_memo" to "MINI_NO_STYLE_MEMO",
        "no_incremental_layout" to "MINI_NO_INCREMENTAL_LAYOUT",
        "layout_verify" to "MINI_LAYOUT_VERIFY",
        "mem_log" to "MINI_MEM_LOG",
        "scroll_log" to "MINI_SCROLL_LOG",
        "layout_log" to "MINI_LAYOUT_LOG",
        "input_log" to "MINI_INPUT_LOG",
        "backend" to "MINI_BACKEND",
        "no_resample" to "MINI_NO_RESAMPLE",
        "pace" to "MINI_PACE",
        "cpu_pin" to "MINI_CPU_PIN",
    )

    /** 建引擎（以及 `nativeWarmup`）之前调 */
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
            Log.i("mini-render", "诊断开关 $env=$v（来自 debug.mini.${m.groupValues[1]}）")
        }
    }
}
