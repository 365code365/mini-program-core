package dev.minirender

import android.content.Context
import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import android.os.Handler
import android.os.SystemClock
import org.json.JSONObject

/**
 * 小游戏的传感器（game-doc/04 G7.5，A7.3）：SDK 自己采集，作为宿主 API 接住
 * `wx.startAccelerometer / stopAccelerometer`、`startGyroscope / stopGyroscope`、`startCompass / stopCompass`、
 * `startDeviceMotionListening / stopDeviceMotionListening`（[MiniGameView] 建实例时把这几个名字并进 hostApis），
 * 采到的数据经 `mr_game_dispatch_event` 推成 `accelerometerChange` 等事件。
 *
 * - interval：`game` 20ms、`ui` 60ms、`normal`（缺省）200ms（官方）。系统给的采样率比要的高，这里按间隔抽样，
 *   回调频率就是 1000 / 间隔（A7.3：game 档 50±5Hz）；
 * - 加速度 `{x, y, z}` 以 g 为单位（Android 的 m/s² ÷ 9.80665），轴向沿用 Android 的设备坐标系（**与 iOS 的符号是否一致待取证**）；
 * - 陀螺仪 `{x, y, z}` rad/s；罗盘 `{direction, accuracy}`（方位角 0~360 度，accuracy 按系统精度给 high / medium / low / unreliable）；
 * - DeviceMotion `{alpha, beta, gamma}` 度（旋转矢量换算，alpha 0~360、beta −180~180、gamma −90~90）；
 * - 切后台全部停（[pauseAll]），回前台按原来的间隔恢复；进程里没有对应传感器时 start fail。
 * 所有方法在引擎线程上调，传感器回调也投递到同一个 [handler]。
 */
internal class GameSensors(
    context: Context,
    private val handler: Handler,
    private val emit: (name: String, json: String) -> Unit,
) {
    private val sm = context.getSystemService(Context.SENSOR_SERVICE) as SensorManager

    private class Running(val sensor: Sensor, val intervalMs: Long, val listener: SensorEventListener) {
        var last = 0L
    }

    private val running = HashMap<String, Running>()
    private var paused = false

    companion object {
        val APIS = listOf(
            "startAccelerometer", "stopAccelerometer", "startGyroscope", "stopGyroscope",
            "startCompass", "stopCompass", "startDeviceMotionListening", "stopDeviceMotionListening",
        )
        private val ACCURACY = mapOf(
            SensorManager.SENSOR_STATUS_ACCURACY_HIGH to "high", SensorManager.SENSOR_STATUS_ACCURACY_MEDIUM to "medium",
            SensorManager.SENSOR_STATUS_ACCURACY_LOW to "low",
        )
    }

    private fun intervalOf(args: JSONObject?): Long = when (args?.optString("interval")) {
        "game" -> 20L
        "ui" -> 60L
        else -> 200L
    }

    /** 宿主调用：处理了返回 (ok, 结果 JSON)，不归这里管返回 null */
    fun handle(api: String, args: JSONObject?): Pair<Boolean, String>? {
        if (api !in APIS) return null
        val kind = api.removePrefix("start").removePrefix("stop")
        val ok = { JSONObject().put("errMsg", "$api:ok").toString() }
        if (api.startsWith("stop")) {
            stop(kind)
            return true to ok()
        }
        val type = when (kind) {
            "Accelerometer" -> Sensor.TYPE_ACCELEROMETER
            "Gyroscope" -> Sensor.TYPE_GYROSCOPE
            else -> Sensor.TYPE_ROTATION_VECTOR
        }
        val s = sm.getDefaultSensor(type) ?: return false to JSONObject().put("errMsg", "$api:fail 设备没有这个传感器").toString()
        stop(kind)
        lateinit var r: Running
        val l = object : SensorEventListener {
            override fun onSensorChanged(e: SensorEvent) {
                val now = SystemClock.uptimeMillis()
                // 抽样：回调间隔不小于要的间隔（留 2ms 余量，系统给的采样时刻有抖动）
                if (now - r.last < r.intervalMs - 2) return
                r.last = now
                val (name, data) = convert(kind, e) ?: return
                emit(name, data.toString())
            }
            override fun onAccuracyChanged(sensor: Sensor?, accuracy: Int) {}
        }
        r = Running(s, intervalOf(args), l)
        running[kind] = r
        if (!paused) register(r)
        return true to ok()
    }

    private fun register(r: Running) {
        // 系统的档位（GAME 20ms / UI 60ms / NORMAL 200ms）再按要的间隔抽样。
        // 一加 PLY110（ColorOS）上注册之后约 5 秒才来第一个采样（交给主线程 looper、改用微秒周期都一样，是系统 / HAL 的行为），
        // 之后稳定 50Hz；记在验收文档
        val delay = when { r.intervalMs <= 20 -> SensorManager.SENSOR_DELAY_GAME; r.intervalMs <= 60 -> SensorManager.SENSOR_DELAY_UI; else -> SensorManager.SENSOR_DELAY_NORMAL }
        sm.registerListener(r.listener, r.sensor, delay, handler)
    }

    private fun stop(kind: String) {
        running.remove(kind)?.let { sm.unregisterListener(it.listener) }
    }

    private val rot = FloatArray(9)
    private val ori = FloatArray(3)

    private fun convert(kind: String, e: SensorEvent): Pair<String, JSONObject>? = when (kind) {
        "Accelerometer" -> "accelerometerChange" to JSONObject().put("x", e.values[0] / SensorManager.GRAVITY_EARTH)
            .put("y", e.values[1] / SensorManager.GRAVITY_EARTH).put("z", e.values[2] / SensorManager.GRAVITY_EARTH)
        "Gyroscope" -> "gyroscopeChange" to JSONObject().put("x", e.values[0]).put("y", e.values[1]).put("z", e.values[2])
        else -> {
            SensorManager.getRotationMatrixFromVector(rot, e.values)
            SensorManager.getOrientation(rot, ori)
            val deg = { r: Float -> Math.toDegrees(r.toDouble()) }
            if (kind == "Compass") {
                "compassChange" to JSONObject().put("direction", (deg(ori[0]) + 360.0) % 360.0).put("accuracy", ACCURACY[e.accuracy] ?: "unreliable")
            } else {
                "deviceMotionChange" to JSONObject().put("alpha", (deg(ori[0]) + 360.0) % 360.0).put("beta", -deg(ori[1])).put("gamma", deg(ori[2]).coerceIn(-90.0, 90.0))
            }
        }
    }

    fun pauseAll() {
        paused = true
        running.values.forEach { sm.unregisterListener(it.listener) }
    }

    fun resumeAll() {
        if (!paused) return
        paused = false
        running.values.forEach { register(it) }
    }

    fun release() {
        running.values.forEach { sm.unregisterListener(it.listener) }
        running.clear()
    }
}
