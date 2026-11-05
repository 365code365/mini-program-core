package dev.minirender

import org.json.JSONObject

/**
 * [MiniMapView] 的属性落地（官方 component/map 的属性表）。iOS 同一套在 `MiniMapView.swift` 的 `onAttrs`。
 *
 * - 视野：改了 latitude / longitude / scale 才挪（没改的属性不把用户拖过的视野拉回去）；
 * - **`setting`**：官方「仅设置的属性会生效，其它的不受影响」—— 里面写了的键盖过同名属性，没写的看属性。
 *   这里认 showLocation / showScale / enableZoom / enableScroll / enableSatellite；
 *   skew / rotate / showCompass / enable3D / enableOverlooking / enableTraffic / subKey / layerStyle 没有对应能力（栅格瓦片），忽略；
 * - 数组属性（markers / polyline / circles / polygons / controls）变了才重新解析；
 * - `__callouts`：不是小程序写的属性，是 SameLayerViews 把引擎交来的自定义气泡图（`callouts`）塞进来的（变了才重新读）。
 */
internal object MapAttrs {
    fun apply(m: MiniMapView, attrs: JSONObject) {
        val prev = m.prev
        val changed = { k: String -> attrs.optString(k) != prev.optString(k) || !prev.has(k) }
        val raw = attrs.optString("setting").trim()
        val setting = (if (raw.startsWith("{")) runCatching { JSONObject(raw) }.getOrNull() else null) ?: JSONObject()
        fun flag(attr: String, key: String, def: Boolean) =
            if (setting.has(key)) setting.optBoolean(key, def) else MapGeo.flag(attrs, attr, def)

        m.minZoom = MapGeo.num(attrs, "min-scale")?.coerceIn(3.0, 20.0) ?: 3.0
        m.maxZoom = MapGeo.num(attrs, "max-scale")?.coerceIn(m.minZoom, 20.0) ?: 20.0
        if (changed("latitude") || changed("longitude")) {
            val lat = MapGeo.num(attrs, "latitude")
            val lng = MapGeo.num(attrs, "longitude")
            if (lat != null && lng != null && LatLng(lat, lng).valid) {
                m.centerLat = lat
                m.centerLng = lng
            }
        }
        if (changed("scale")) m.zoom = (MapGeo.num(attrs, "scale") ?: 16.0).coerceIn(m.minZoom, m.maxZoom)
        m.zoom = m.zoom.coerceIn(m.minZoom, m.maxZoom)
        m.satellite = flag("enable-satellite", "enableSatellite", false)
        m.showScale = flag("show-scale", "showScale", false)
        m.scrollEnabled = flag("enable-scroll", "enableScroll", true)
        m.zoomEnabled = flag("enable-zoom", "enableZoom", true)
        m.applyShowLocation(flag("show-location", "showLocation", false))
        if (changed("markers")) {
            val a = MapGeo.array(attrs, "markers")
            m.markers = (0 until a.length()).mapNotNull { a.optJSONObject(it)?.let(::MapMarker)?.takeIf { mk -> mk.pos.valid } }
        }
        if (changed("polyline")) m.lines = MapGeo.array(attrs, "polyline").let { a -> (0 until a.length()).mapNotNull { a.optJSONObject(it)?.let(::MapLine) } }
        if (changed("circles")) m.circles = MapGeo.array(attrs, "circles").let { a -> (0 until a.length()).mapNotNull { a.optJSONObject(it)?.let(::MapCircle) } }
        if (changed("polygons")) {
            m.polygons = MapGeo.array(attrs, "polygons").let { a -> (0 until a.length()).mapNotNull { a.optJSONObject(it)?.let(::MapPolygon) } }
                .sortedBy { it.zIndex }
        }
        if (changed("controls")) m.controls = MapGeo.array(attrs, "controls").let { a -> (0 until a.length()).mapNotNull { a.optJSONObject(it)?.let(::MapControl) } }
        if (changed("__callouts")) {
            val a = MapGeo.array(attrs, "__callouts")
            m.customImages = (0 until a.length()).mapNotNull { a.optJSONObject(it) }.associate {
                it.optLong("markerId") to MapCalloutImage(it.optString("path"), it.optDouble("width").toFloat(), it.optDouble("height").toFloat())
            }
        }
        if (changed("include-points")) {
            val pts = MapGeo.points(MapGeo.array(attrs, "include-points"))
            if (pts.isNotEmpty()) m.fit(pts, FloatArray(4) { 30f }, animate = false)
        }
        m.prev = JSONObject(attrs.toString())
        m.invalidate()
    }
}

/** 引擎离屏画好的一个自定义气泡（PNG 路径 + 逻辑尺寸） */
internal class MapCalloutImage(val path: String, val w: Float, val h: Float)
