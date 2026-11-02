package dev.minirender

import android.animation.ValueAnimator
import android.graphics.Canvas
import android.view.animation.DecelerateInterpolator
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.abs
import kotlin.math.exp
import kotlin.math.hypot
import kotlin.math.ln
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow

/**
 * SDK 自带的 `<map>`（官方 component/map.html）：瓦片地图 + 标记 / 气泡 / 标签 / 折线 / 圆 / 多边形 / 比例尺，
 * **同层渲染**（画进 [MiniSameLayerView] 的 Canvas，页面滚动时就是页面里的一块像素）。
 *
 * - 手势：按在地图上的整串触摸交给它（[wantsGestures]）：单指拖动、惯性、双指捏合缩放、双击放大；
 *   `enable-scroll` / `enable-zoom` 都关掉的地图不接手势（拖动滚页面，点击照样认）。
 * - 事件：`tap {latitude, longitude}`、`markertap` / `callouttap` / `labeltap {markerId}`、
 *   `regionchange`（`begin` / `end`，`causedBy` 是 gesture / drag / scale / update，end 带 scale、centerLocation、region）、`updated`。
 * - MapContext：[contextMethods] 里那些（经 hostCall 交过来，见 [onContextCall]）。
 * - 没做：show-location 的定位蓝点（要定位权限，宿主自己接）、旋转 / 倾斜、路况、个性化样式、点聚合。
 *
 * 绘制在 `MapDraw.kt`，瓦片在 `MapTiles.kt`，数据与投影在 `MapGeo.kt`。iOS 同一套在 `MiniMapView.swift`。
 */
class MiniMapView : MiniSameLayerView() {
    // ── 视野（逻辑像素坐标系，缩放级别可以是小数）──
    internal var centerLat = 39.908823
    internal var centerLng = 116.39747
    internal var zoom = 16.0
    internal var minZoom = 3.0
    internal var maxZoom = 20.0
    internal var satellite = false
    internal var showScale = false
    /** `setCenterOffset`：中心点在框里的位置（比例） */
    internal var offsetX = 0.5
    internal var offsetY = 0.5

    // ── 覆盖物 ──
    internal var markers: List<MapMarker> = emptyList()
    internal val extraMarkers = ArrayList<MapMarker>()
    internal var lines: List<MapLine> = emptyList()
    internal var circles: List<MapCircle> = emptyList()
    internal var polygons: List<MapPolygon> = emptyList()
    /** 点开了气泡的标记（BYCLICK） */
    internal val openCallouts = HashSet<Long>()
    /** 上一次绘制时各个可点的东西的位置（逻辑像素），点击按它判 */
    internal val hits = ArrayList<MapHit>()

    private var gestures = true
    private var prev = JSONObject()
    private var tilesRef: MapTiles? = null
    internal val tiles: MapTiles?
        get() {
            if (tilesRef == null) {
                val c = context ?: return null
                tilesRef = MapTiles(c) { tileSource }.also { it.onLoaded { invalidate() } }
            }
            return tilesRef
        }
    /** 上一帧缺瓦片：补齐的那一帧发 `updated` */
    internal var missing = true
    /** 上一帧画的缩放级别（MapDraw 判「在缩放中」） */
    internal var lastDrawZoom = Double.NaN
    private var updatedSent = false

    override val wantsGestures get() = true
    override val contextClass get() = "MapContext"
    override val contextMethods get() = METHODS

    override fun onAttrs(attrs: JSONObject) {
        val changed = { k: String -> attrs.optString(k) != prev.optString(k) || !prev.has(k) }
        minZoom = MapGeo.num(attrs, "min-scale")?.coerceIn(3.0, 20.0) ?: 3.0
        maxZoom = MapGeo.num(attrs, "max-scale")?.coerceIn(minZoom, 20.0) ?: 20.0
        // 官方：改了 latitude / longitude / scale 才挪视野；没改的属性不把用户拖过的视野拉回去
        if (changed("latitude") || changed("longitude")) {
            val lat = MapGeo.num(attrs, "latitude")
            val lng = MapGeo.num(attrs, "longitude")
            if (lat != null && lng != null && LatLng(lat, lng).valid) {
                centerLat = lat
                centerLng = lng
            }
        }
        if (changed("scale")) zoom = (MapGeo.num(attrs, "scale") ?: 16.0).coerceIn(minZoom, maxZoom)
        zoom = zoom.coerceIn(minZoom, maxZoom)
        satellite = MapGeo.flag(attrs, "enable-satellite", false)
        showScale = MapGeo.flag(attrs, "show-scale", false)
        gestures = MapGeo.flag(attrs, "enable-scroll", true) || MapGeo.flag(attrs, "enable-zoom", true)
        if (changed("markers")) {
            val a = MapGeo.array(attrs, "markers")
            markers = (0 until a.length()).mapNotNull { a.optJSONObject(it)?.let(::MapMarker)?.takeIf { m -> m.pos.valid } }
        }
        if (changed("polyline")) lines = MapGeo.array(attrs, "polyline").let { a -> (0 until a.length()).mapNotNull { a.optJSONObject(it)?.let(::MapLine) } }
        if (changed("circles")) circles = MapGeo.array(attrs, "circles").let { a -> (0 until a.length()).mapNotNull { a.optJSONObject(it)?.let(::MapCircle) } }
        if (changed("polygons")) polygons = MapGeo.array(attrs, "polygons").let { a -> (0 until a.length()).mapNotNull { a.optJSONObject(it)?.let(::MapPolygon) } }
        if (changed("include-points")) {
            val pts = MapGeo.points(MapGeo.array(attrs, "include-points"))
            if (pts.isNotEmpty()) fit(pts, FloatArray(4) { 30f }, animate = false)
        }
        prev = JSONObject(attrs.toString())
        invalidate()
    }

    private val main = android.os.Handler(android.os.Looper.getMainLooper())
    private val settle = Runnable { invalidate() }

    override fun onDraw(canvas: Canvas, width: Int, height: Int, density: Float) {
        // 缩放中那几帧瓦片不滤波（快）；停下 120ms 后按缩好的瓦片再画一遍
        main.removeCallbacks(settle)
        if (MapDraw.draw(this, canvas, width, height, density)) main.postDelayed(settle, 120)
        if (!missing && !updatedSent) {
            updatedSent = true
            emit("updated")
        }
        if (missing) updatedSent = false
    }

    override fun onDetach() {
        anim?.cancel()
    }

    // ───────────────────────── 投影 ─────────────────────────

    internal fun toScreen(p: LatLng): Pair<Float, Float> {
        val x = MapGeo.x(p.lng, zoom) - MapGeo.x(centerLng, zoom) + width * offsetX
        val y = MapGeo.y(p.lat, zoom) - MapGeo.y(centerLat, zoom) + height * offsetY
        return x.toFloat() to y.toFloat()
    }

    internal fun fromScreen(x: Float, y: Float): LatLng {
        val wx = MapGeo.x(centerLng, zoom) + (x - width * offsetX)
        val wy = MapGeo.y(centerLat, zoom) + (y - height * offsetY)
        return LatLng(MapGeo.lat(wy, zoom), MapGeo.lng(wx, zoom))
    }

    /** 把屏上 (x, y) 处的那一点挪到 (x + dx, y + dy) */
    private fun panBy(dx: Float, dy: Float) {
        val c = fromScreen(width * offsetX.toFloat() - dx, height * offsetY.toFloat() - dy)
        centerLat = c.lat.coerceIn(-85.0, 85.0)
        centerLng = ((c.lng + 540.0) % 360.0) - 180.0
    }

    /** 缩放到 [z]，屏上 (ax, ay) 处的那一点不动 */
    private fun zoomAround(z: Double, ax: Float, ay: Float) {
        val anchor = fromScreen(ax, ay)
        zoom = z.coerceIn(minZoom, maxZoom)
        val (sx, sy) = toScreen(anchor)
        panBy(ax - sx, ay - sy)
    }

    internal fun region(): JSONObject {
        val sw = fromScreen(0f, height)
        val ne = fromScreen(width, 0f)
        return JSONObject().put("southwest", sw.json()).put("northeast", ne.json())
    }

    // ───────────────────────── 事件 ─────────────────────────

    private var regionOpen = false

    private fun regionBegin(cause: String) {
        if (regionOpen) return
        regionOpen = true
        emit("regionchange", JSONObject().put("type", "begin").put("causedBy", cause))
    }

    private fun regionEnd(cause: String) {
        if (!regionOpen) return
        regionOpen = false
        emit("regionchange", JSONObject().put("type", "end").put("causedBy", cause)
            .put("rotate", 0).put("skew", 0).put("scale", (zoom * 100).toLong() / 100.0)
            .put("centerLocation", LatLng(centerLat, centerLng).json()).put("region", region()))
    }

    override fun onTap(x: Float, y: Float) = tapAt(x, y)

    private fun tapAt(x: Float, y: Float) {
        // 后画的在上面：倒着找
        val hit = hits.lastOrNull { it.rect.contains(x, y) }
        when (hit?.kind) {
            MapHit.CALLOUT -> emit("callouttap", JSONObject().put("markerId", hit.id))
            MapHit.LABEL -> emit("labeltap", JSONObject().put("markerId", hit.id))
            MapHit.MARKER -> {
                val m = (markers + extraMarkers).firstOrNull { it.id == hit.id }
                if (m?.callout != null && !m.callout.always && !openCallouts.remove(m.id)) openCallouts += m.id
                emit("markertap", JSONObject().put("markerId", hit.id))
                invalidate()
            }
            else -> {
                val p = fromScreen(x, y)
                emit("tap", p.json())
            }
        }
    }

    // ───────────────────────── 手势 ─────────────────────────

    private enum class G { IDLE, PENDING, DRAG, PINCH }
    private var g = G.IDLE
    private var p0x = 0f
    private var p0y = 0f
    private var p1x = 0f
    private var p1y = 0f
    private var downX = 0f
    private var downY = 0f
    private var downT = 0L
    private var lastTapT = -1000L
    private var lastTapX = 0f
    private var lastTapY = 0f
    private var pinchDist = 1f
    private var pinchZoom = 16.0
    private var pinched = false
    private val samples = ArrayDeque<Triple<Long, Float, Float>>()

    override fun onTouch(phase: String, pointer: Int, x: Float, y: Float, t: Long) {
        if (!gestures) {
            if (pointer == 0 && phase == "end" && hypot(x - downX, y - downY) < SLOP) tapAt(x, y)
            if (pointer == 0 && phase == "start") { downX = x; downY = y }
            return
        }
        if (pointer == 1) return onSecond(phase, x, y)
        when (phase) {
            "start" -> {
                anim?.cancel()
                regionEnd("drag")
                g = G.PENDING
                p0x = x; p0y = y; downX = x; downY = y; downT = t
                pinched = false
                samples.clear()
                samples.add(Triple(t, x, y))
            }
            "move" -> {
                if (g == G.PENDING && hypot(x - downX, y - downY) > SLOP) {
                    g = G.DRAG
                    regionBegin("gesture")
                }
                when (g) {
                    G.DRAG -> {
                        panBy(x - p0x, y - p0y)
                        invalidate()
                    }
                    G.PINCH -> {
                        p0x = x; p0y = y
                        applyPinch()
                        return
                    }
                    else -> {}
                }
                p0x = x; p0y = y
                samples.add(Triple(t, x, y))
                while (samples.size > 2 && t - samples.first().first > 100) samples.removeFirst()
            }
            "end" -> {
                when (g) {
                    G.PENDING -> if (!pinched) tapOrDouble(x, y, t)
                    G.DRAG -> fling(t)
                    G.PINCH -> regionEnd("scale")
                    G.IDLE -> {}
                }
                g = G.IDLE
            }
            else -> {
                regionEnd(if (pinched) "scale" else "drag")
                g = G.IDLE
            }
        }
    }

    private fun onSecond(phase: String, x: Float, y: Float) {
        when (phase) {
            "start" -> {
                if (g == G.IDLE) return
                anim?.cancel()
                p1x = x; p1y = y
                g = G.PINCH
                pinched = true
                pinchDist = hypot(p1x - p0x, p1y - p0y).coerceAtLeast(1f)
                pinchZoom = zoom
                regionBegin("gesture")
            }
            "move" -> if (g == G.PINCH) {
                p1x = x; p1y = y
                applyPinch()
            }
            else -> if (g == G.PINCH) {
                // 第二根抬起：剩下那根接着拖
                g = G.DRAG
                samples.clear()
            }
        }
    }

    private fun applyPinch() {
        val d = hypot(p1x - p0x, p1y - p0y).coerceAtLeast(1f)
        val mx = (p0x + p1x) / 2
        val my = (p0y + p1y) / 2
        zoomAround(pinchZoom + ln(d / pinchDist.toDouble()) / ln(2.0), mx, my)
        invalidate()
    }

    private fun tapOrDouble(x: Float, y: Float, t: Long) {
        if (t - lastTapT < DOUBLE_TAP_MS && hypot(x - lastTapX, y - lastTapY) < 30f) {
            lastTapT = -1000L
            regionBegin("scale")
            animate(300, onEnd = { regionEnd("scale") }) { f, from ->
                zoomAround(from.zoom + f, x, y)
            }
            return
        }
        lastTapT = t; lastTapX = x; lastTapY = y
        tapAt(x, y)
    }

    private fun fling(t: Long) {
        val first = samples.firstOrNull()
        val last = samples.lastOrNull()
        val dt = if (first != null && last != null) (last.first - first.first).toFloat() else 0f
        if (first == null || last == null || dt < 8f || t - last.first > 80) return regionEnd("drag")
        val vx = (last.second - first.second) / dt
        val vy = (last.third - first.third) / dt
        if (hypot(vx, vy) < 0.25f) return regionEnd("drag")
        // 指数衰减：总位移 v·τ（与 Android OverScroller 的手感接近）
        val tau = 325f
        var done = 0f
        animate(1100, interp = false, onEnd = { regionEnd("drag") }) { f, _ ->
            val k = 1f - exp(-f * 1100f / tau)
            val step = k - done
            done = k
            panBy(vx * tau * step, vy * tau * step)
        }
    }

    // ───────────────────────── 动画 ─────────────────────────

    internal class From(val lat: Double, val lng: Double, val zoom: Double)
    private var anim: ValueAnimator? = null

    internal fun animate(ms: Long, interp: Boolean = true, onEnd: () -> Unit = {}, step: (Float, From) -> Unit) {
        anim?.cancel()
        val from = From(centerLat, centerLng, zoom)
        var ended = false
        anim = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = ms
            interpolator = if (interp) DecelerateInterpolator() else null
            addUpdateListener {
                step(it.animatedValue as Float, from)
                invalidate()
            }
            addListener(object : android.animation.AnimatorListenerAdapter() {
                override fun onAnimationEnd(a: android.animation.Animator) {
                    if (!ended) { ended = true; onEnd() }
                }
                override fun onAnimationCancel(a: android.animation.Animator) {
                    if (!ended) { ended = true; onEnd() }
                }
            })
            start()
        }
    }

    /** 让这些点都落在框里（[pad] 上右下左，逻辑像素） */
    internal fun fit(pts: List<LatLng>, pad: FloatArray, animate: Boolean, done: () -> Unit = {}) {
        if (pts.isEmpty() || width <= 0f || height <= 0f) return done()
        val minLat = pts.minOf { it.lat }
        val maxLat = pts.maxOf { it.lat }
        val minLng = pts.minOf { it.lng }
        val maxLng = pts.maxOf { it.lng }
        val aw = (width - pad[1] - pad[3]).coerceAtLeast(20f)
        val ah = (height - pad[0] - pad[2]).coerceAtLeast(20f)
        // 在第 0 级上量跨度，再按框换算级别
        val dx = abs(MapGeo.x(maxLng, 0.0) - MapGeo.x(minLng, 0.0)).coerceAtLeast(1e-9)
        val dy = abs(MapGeo.y(minLat, 0.0) - MapGeo.y(maxLat, 0.0)).coerceAtLeast(1e-9)
        val z = min(ln(aw / dx) / ln(2.0), ln(ah / dy) / ln(2.0)).coerceIn(minZoom, min(maxZoom, 18.0))
        val cx = (MapGeo.x(minLng, z) + MapGeo.x(maxLng, z)) / 2 + (pad[1] - pad[3]) / 2.0
        val cy = (MapGeo.y(maxLat, z) + MapGeo.y(minLat, z)) / 2 + (pad[2] - pad[0]) / 2.0
        val target = LatLng(MapGeo.lat(cy, z), MapGeo.lng(cx, z))
        moveTo(target, z, animate, done)
    }

    internal fun moveTo(target: LatLng, z: Double, animate: Boolean, done: () -> Unit = {}) {
        if (!animate) {
            centerLat = target.lat; centerLng = target.lng; zoom = z.coerceIn(minZoom, maxZoom)
            invalidate()
            return done()
        }
        regionBegin("update")
        animate(500, onEnd = { regionEnd("update"); done() }) { f, from ->
            centerLat = from.lat + (target.lat - from.lat) * f
            centerLng = from.lng + (target.lng - from.lng) * f
            zoom = (from.zoom + (z - from.zoom) * f).coerceIn(minZoom, maxZoom)
        }
    }

    // ───────────────────────── MapContext ─────────────────────────

    override fun onContextCall(method: String, args: JSONObject, reply: MiniHostReply): Boolean =
        MapContextCalls.handle(this, method, args, reply)

    companion object {
        /** 瓦片源（全局，建第一张地图之前设）。默认高德公开瓦片 */
        @JvmStatic
        var tileSource: MiniMapTileSource = MiniMapTileSource.AUTONAVI

        private const val SLOP = 8f
        private const val DOUBLE_TAP_MS = 300L

        internal val METHODS = listOf(
            "getCenterLocation", "getRegion", "getScale", "getRotate", "getSkew", "moveToLocation", "includePoints",
            "translateMarker", "moveAlong", "addMarkers", "removeMarkers", "toScreenLocation", "fromScreenLocation",
            "setCenterOffset", "openMapApp",
        )

        internal fun pow2(z: Double) = 2.0.pow(z)
        internal fun clamp01(v: Float) = max(0f, min(1f, v))
        internal fun arr(a: JSONArray?, i: Int, def: Float) = a?.optDouble(i, def.toDouble())?.toFloat() ?: def
    }
}

/** 上一帧画出来的一个可点的东西 */
internal class MapHit(val kind: Int, val id: Long, val rect: android.graphics.RectF) {
    companion object {
        const val MARKER = 0
        const val CALLOUT = 1
        const val LABEL = 2
    }
}
