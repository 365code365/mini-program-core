package dev.minirender

import android.graphics.Color
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.PI
import kotlin.math.atan
import kotlin.math.cos
import kotlin.math.exp
import kotlin.math.ln
import kotlin.math.pow
import kotlin.math.tan

/**
 * [MiniMapView] 的数据与投影：Web 墨卡托（256 逻辑像素一张瓦片），`<map>` 属性里的
 * markers / polyline / circles / polygons 按官方字段解析。坐标原样用（微信 map 默认 GCJ-02，
 * 默认瓦片源高德也是 GCJ-02，两边对得上）。iOS 那份是 `MapGeo.swift`，改一边对照另一边。
 */
internal object MapGeo {
    const val TILE = 256.0

    /** 经度 → 世界像素 x（缩放级别 [z]，可以是小数） */
    fun x(lng: Double, z: Double) = (lng + 180.0) / 360.0 * TILE * 2.0.pow(z)

    /** 纬度 → 世界像素 y */
    fun y(lat: Double, z: Double): Double {
        val r = Math.toRadians(lat.coerceIn(-85.05112878, 85.05112878))
        return (1.0 - ln(tan(r) + 1.0 / cos(r)) / PI) / 2.0 * TILE * 2.0.pow(z)
    }

    fun lng(x: Double, z: Double) = x / (TILE * 2.0.pow(z)) * 360.0 - 180.0

    fun lat(y: Double, z: Double): Double {
        val n = PI - 2.0 * PI * y / (TILE * 2.0.pow(z))
        return Math.toDegrees(atan(0.5 * (exp(n) - exp(-n))))
    }

    /** 这个纬度上一个逻辑像素多少米 */
    fun metersPerPx(lat: Double, z: Double) = 156543.03392 * cos(Math.toRadians(lat)) / 2.0.pow(z)

    /** `#RRGGBB` / `#RRGGBBAA`（微信的写法，透明度在后）/ `#RGB` → ARGB */
    fun color(s: String?, def: Int): Int {
        val t = s?.trim()?.removePrefix("#") ?: return def
        return try {
            when (t.length) {
                3 -> Color.rgb(hex(t[0]) * 17, hex(t[1]) * 17, hex(t[2]) * 17)
                6 -> (0xFF shl 24) or t.toLong(16).toInt()
                8 -> {
                    val v = t.toLong(16)
                    (((v and 0xFF) shl 24) or (v shr 8)).toInt()
                }
                else -> def
            }
        } catch (e: NumberFormatException) {
            def
        }
    }

    private fun hex(c: Char) = Character.digit(c, 16).coerceAtLeast(0)

    /** 属性里的 JSON（引擎给原生组件的数组属性是标准 JSON）；空的 / 坏的当空数组 */
    fun array(attrs: JSONObject, k: String): JSONArray {
        val raw = attrs.optString(k).trim()
        if (!raw.startsWith("[")) return JSONArray()
        return try {
            JSONArray(raw)
        } catch (e: Exception) {
            JSONArray()
        }
    }

    fun points(a: JSONArray?): List<LatLng> = (0 until (a?.length() ?: 0)).mapNotNull { i ->
        a!!.optJSONObject(i)?.let { LatLng(it.optDouble("latitude"), it.optDouble("longitude")) }?.takeIf { it.valid }
    }

    /** 布尔属性：写了且不是 false / 0 */
    fun flag(attrs: JSONObject, k: String, def: Boolean): Boolean {
        if (!attrs.has(k)) return def
        return attrs.optString(k).trim() !in listOf("false", "0", "{{false}}")
    }

    fun num(attrs: JSONObject, k: String): Double? = attrs.optString(k).trim().toDoubleOrNull()
}

internal data class LatLng(val lat: Double, val lng: Double) {
    val valid get() = !lat.isNaN() && !lng.isNaN() && lat in -90.0..90.0 && lng in -180.0..180.0
    fun json(): JSONObject = JSONObject().put("latitude", lat).put("longitude", lng)
}

/** 气泡（callout）或标签（label） */
internal class MapBubble(o: JSONObject) {
    val content = o.optString("content")
    val color = MapGeo.color(o.optString("color", "#000000"), Color.BLACK)
    val bg = MapGeo.color(o.optString("bgColor", "#ffffff"), Color.WHITE)
    val fontSize = o.optDouble("fontSize", 14.0).toFloat()
    val padding = o.optDouble("padding", 0.0).toFloat()
    val radius = o.optDouble("borderRadius", 0.0).toFloat()
    val borderWidth = o.optDouble("borderWidth", 0.0).toFloat()
    val borderColor = MapGeo.color(o.optString("borderColor"), Color.TRANSPARENT)
    /** callout：`ALWAYS` 常显，`BYCLICK` 点标记才出 */
    val always = o.optString("display", "BYCLICK").equals("ALWAYS", true)
    val anchorX = o.optDouble("anchorX", 0.0).toFloat()
    val anchorY = o.optDouble("anchorY", 0.0).toFloat()
}

internal class MapMarker(o: JSONObject) {
    /** 官方：markerId 是 number */
    val id: Long = o.optLong("id", 0)
    var pos = LatLng(o.optDouble("latitude"), o.optDouble("longitude"))
    val title = o.optString("title")
    val iconPath = o.optString("iconPath")
    val width = o.optDouble("width", Double.NaN).toFloat()
    val height = o.optDouble("height", Double.NaN).toFloat()
    var rotate = o.optDouble("rotate", 0.0).toFloat()
    val alpha = o.optDouble("alpha", 1.0).toFloat().coerceIn(0f, 1f)
    val anchorX = o.optJSONObject("anchor")?.optDouble("x", 0.5)?.toFloat() ?: 0.5f
    val anchorY = o.optJSONObject("anchor")?.optDouble("y", 1.0)?.toFloat() ?: 1f
    val callout = o.optJSONObject("callout")?.let(::MapBubble)
    val label = o.optJSONObject("label")?.let(::MapBubble)
    val zIndex = o.optInt("zIndex", 0)
}

internal class MapLine(o: JSONObject) {
    val points = MapGeo.points(o.optJSONArray("points"))
    val color = MapGeo.color(o.optString("color", "#000000"), Color.BLACK)
    val width = o.optDouble("width", 2.0).toFloat()
    val dotted = o.optBoolean("dottedLine", false)
    val arrow = o.optBoolean("arrowLine", false)
    val borderColor = MapGeo.color(o.optString("borderColor"), Color.TRANSPARENT)
    val borderWidth = o.optDouble("borderWidth", 0.0).toFloat()
}

internal class MapCircle(o: JSONObject) {
    val center = LatLng(o.optDouble("latitude"), o.optDouble("longitude"))
    val radius = o.optDouble("radius", 0.0)
    val color = MapGeo.color(o.optString("color"), Color.TRANSPARENT)
    val fill = MapGeo.color(o.optString("fillColor"), Color.TRANSPARENT)
    val stroke = o.optDouble("strokeWidth", 0.0).toFloat()
}

internal class MapPolygon(o: JSONObject) {
    val points = MapGeo.points(o.optJSONArray("points"))
    val stroke = o.optDouble("strokeWidth", 0.0).toFloat()
    val strokeColor = MapGeo.color(o.optString("strokeColor"), Color.TRANSPARENT)
    val fill = MapGeo.color(o.optString("fillColor"), Color.TRANSPARENT)
}
