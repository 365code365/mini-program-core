package dev.minirender

import android.graphics.Color
import org.json.JSONArray
import org.json.JSONObject
import android.graphics.Bitmap
import android.graphics.DashPathEffect
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.atan
import kotlin.math.cos
import kotlin.math.exp
import kotlin.math.ln
import kotlin.math.pow
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlin.math.sqrt
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

    /**
     * `#RRGGBB` / `#RRGGBBAA`（微信的写法，透明度在后）/ `#RGB` / `#RGBA`（同样透明度在后）/
     * `rgb(r, g, b)` / `rgba(r, g, b, a)`（a 是 0~1 或百分比）→ ARGB。认不出的给 [def]
     */
    fun color(s: String?, def: Int): Int {
        val raw = s?.trim() ?: return def
        if (raw.startsWith("rgb", ignoreCase = true)) return rgbFunc(raw) ?: def
        val t = raw.removePrefix("#")
        return try {
            val v = t.toLong(16)
            when (t.length) {
                3 -> Color.rgb(hex(t[0]) * 17, hex(t[1]) * 17, hex(t[2]) * 17)
                4 -> Color.argb(hex(t[3]) * 17, hex(t[0]) * 17, hex(t[1]) * 17, hex(t[2]) * 17)
                6 -> (0xFF shl 24) or v.toInt()
                8 -> (((v and 0xFF) shl 24) or (v shr 8)).toInt()
                else -> def
            }
        } catch (e: NumberFormatException) {
            def
        }
    }

    private fun hex(c: Char) = Character.digit(c, 16).coerceAtLeast(0)

    /** `rgb(…)` / `rgba(…)`：逗号或空格分隔，通道可以写百分比 */
    private fun rgbFunc(s: String): Int? {
        val parts = s.substringAfter('(', "").substringBefore(')').split(',', ' ', '/').filter { it.isNotBlank() }
        if (parts.size < 3) return null
        return try {
            fun ch(v: String) = (if (v.endsWith("%")) v.dropLast(1).toFloat() * 2.55f else v.toFloat()).roundToInt().coerceIn(0, 255)
            val a = parts.getOrNull(3)?.let { if (it.endsWith("%")) it.dropLast(1).toFloat() / 100f else it.toFloat() } ?: 1f
            Color.argb((a.coerceIn(0f, 1f) * 255).roundToInt(), ch(parts[0]), ch(parts[1]), ch(parts[2]))
        } catch (e: NumberFormatException) {
            null
        }
    }

    /** 颜色数组（polyline 的 colorList）；不是数组给空 */
    fun colors(a: JSONArray?): List<Int> = (0 until (a?.length() ?: 0)).map { color(a!!.optString(it), Color.BLACK) }

    /**
     * WGS-84（GPS / 系统定位给的）→ GCJ-02（微信 map 与高德瓦片用的）。国外坐标原样返回。
     * 公开的标准近似算法（与高德 / 腾讯 SDK 的 coordinateConverter 结果在米级以内一致）
     */
    fun wgsToGcj(lat: Double, lng: Double): LatLng {
        if (lng < 72.004 || lng > 137.8347 || lat < 0.8293 || lat > 55.8271) return LatLng(lat, lng)
        val a = 6378245.0
        val ee = 0.00669342162296594323
        val x = lng - 105.0
        val y = lat - 35.0
        var dLat = -100.0 + 2.0 * x + 3.0 * y + 0.2 * y * y + 0.1 * x * y + 0.2 * sqrt(abs(x)) +
            (20.0 * sin(6.0 * x * PI) + 20.0 * sin(2.0 * x * PI)) * 2.0 / 3.0 +
            (20.0 * sin(y * PI) + 40.0 * sin(y / 3.0 * PI)) * 2.0 / 3.0 +
            (160.0 * sin(y / 12.0 * PI) + 320 * sin(y * PI / 30.0)) * 2.0 / 3.0
        var dLng = 300.0 + x + 2.0 * y + 0.1 * x * x + 0.1 * x * y + 0.1 * sqrt(abs(x)) +
            (20.0 * sin(6.0 * x * PI) + 20.0 * sin(2.0 * x * PI)) * 2.0 / 3.0 +
            (20.0 * sin(x * PI) + 40.0 * sin(x / 3.0 * PI)) * 2.0 / 3.0 +
            (150.0 * sin(x / 12.0 * PI) + 300.0 * sin(x / 30.0 * PI)) * 2.0 / 3.0
        val rad = lat / 180.0 * PI
        var magic = sin(rad)
        magic = 1 - ee * magic * magic
        val sm = sqrt(magic)
        dLat = (dLat * 180.0) / ((a * (1 - ee)) / (magic * sm) * PI)
        dLng = (dLng * 180.0) / (a / sm * cos(rad) * PI)
        return LatLng(lat + dLat, lng + dLng)
    }

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
    /** callout：气泡整体的偏移（向右 / 向下为正）；label：标签左上角相对标记锚点的位置 */
    val anchorX = o.optDouble("anchorX", 0.0).toFloat()
    val anchorY = o.optDouble("anchorY", 0.0).toFloat()
    /** 多行文字的对齐：left / center / right（官方没写默认值，按 left，待验） */
    val textAlign = o.optString("textAlign", "left").lowercase()

    /** 画好的气泡（MapBubbles 按 dpr 画一次，之后每帧只贴图）与气泡框的逻辑尺寸（不含阴影 / 尾巴） */
    var bitmap: Bitmap? = null
    var bitmapDensity = 0f
    var boxW = 0f
    var boxH = 0f

    companion object {
        /** marker 的 `title`（没有 callout / customCallout 时点标记显示）：白底黑字的默认气泡 */
        fun title(s: String) = MapBubble(JSONObject().put("content", s).put("padding", 6).put("borderRadius", 4).put("fontSize", 14))
    }
}

/** marker 的 `customCallout`：内容是 `<map>` 里 slot="callout" 的 cover-view（引擎离屏出图交过来） */
internal class MapCustomCallout(o: JSONObject) {
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
    /** 官方：customCallout 存在时忽略 callout 与 title */
    val custom = o.optJSONObject("customCallout")?.let(::MapCustomCallout)
    /** callout；没写 callout 时 title 当一个点了才出的气泡（官方：title 点击时显示，callout 存在时忽略） */
    val callout = if (custom != null) null
        else o.optJSONObject("callout")?.let(::MapBubble) ?: title.takeIf { it.isNotBlank() }?.let(MapBubble::title)
    val label = o.optJSONObject("label")?.let(::MapBubble)
    val zIndex = o.optInt("zIndex", 0)
}

internal class MapLine(o: JSONObject) {
    val points = MapGeo.points(o.optJSONArray("points"))
    val color = MapGeo.color(o.optString("color", "#000000"), Color.BLACK)
    /** 彩虹线：第 i 段用第 i 个颜色，不够的段用最后一个（官方）。存在时忽略 color */
    val colorList = MapGeo.colors(o.optJSONArray("colorList"))
    val width = o.optDouble("width", 2.0).toFloat()
    val dotted = o.optBoolean("dottedLine", false)
    /** 虚线的样式建一次（从前每帧每条线 new 一个） */
    val dash = if (dotted) DashPathEffect(floatArrayOf(width * 1.5f, width * 1.5f), 0f) else null
    val arrow = o.optBoolean("arrowLine", false)
    /** 换箭头图标（arrowLine 为真时生效） */
    val arrowIconPath = o.optString("arrowIconPath")
    val borderColor = MapGeo.color(o.optString("borderColor"), Color.TRANSPARENT)
    val borderWidth = o.optDouble("borderWidth", 0.0).toFloat()
    /**
     * 压盖关系（abovelabels / abovebuildings / aboveroads）：瓦片是栅格图，路名 / POI / 楼块都烧在图里，
     * 线只能画在整张瓦片之上 —— 三个取值画出来一样（照官方解析、不改顺序）
     */
    val level = o.optString("level", "abovelabels")
}

/** `controls`：不随地图移动的图标（官方：即将废弃，建议用 cover-view；照样支持） */
internal class MapControl(o: JSONObject) {
    val id: Long = o.optLong("id", 0)
    private val p = o.optJSONObject("position")
    val left = p?.optDouble("left", 0.0)?.toFloat() ?: 0f
    val top = p?.optDouble("top", 0.0)?.toFloat() ?: 0f
    /** 没写按图片尺寸（与 marker 同一个换算：图片像素 / 2） */
    val width = p?.optDouble("width", Double.NaN)?.toFloat() ?: Float.NaN
    val height = p?.optDouble("height", Double.NaN)?.toFloat() ?: Float.NaN
    val iconPath = o.optString("iconPath")
    /** 官方：默认不可点（不可点的控件不挡地图，点上去照样是地图的 tap） */
    val clickable = o.optBoolean("clickable", false)
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
    /** 多边形之间的上下（大的在上；同值按数组顺序） */
    val zIndex = o.optInt("zIndex", 0)
    /** 边线虚线 `[实线长, 空白长]`（官方默认 [0, 0] 为实线）：建一次 */
    val dash: DashPathEffect? = o.optJSONArray("dashArray")?.let { a ->
        val on = a.optDouble(0, 0.0).toFloat()
        val off = a.optDouble(1, on.toDouble()).toFloat()
        if (on > 0f && off > 0f) DashPathEffect(floatArrayOf(on, off), 0f) else null
    }
}
