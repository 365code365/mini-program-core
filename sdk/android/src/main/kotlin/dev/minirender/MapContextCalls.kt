package dev.minirender

import android.animation.ValueAnimator
import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import org.json.JSONArray
import org.json.JSONObject

/**
 * `wx.createMapContext(id)` 的方法（经 hostCall 交给 [MiniMapView]，主线程）。回包字段照官方：
 * `getCenterLocation {latitude, longitude}`、`getScale {scale}`、`getRegion {southwest, northeast}`、
 * `toScreenLocation {x, y}`、`fromScreenLocation {latitude, longitude}`。
 * `moveToLocation` 不给坐标时官方是挪到「当前定位」：有定位权限时用定位（[MapLocation]），没有回到属性里的中心点。
 * `openMapApp` 走 `geo:` Intent（iOS 走 `maps://`）。
 */
internal object MapContextCalls {
    fun handle(m: MiniMapView, method: String, a: JSONObject, reply: MiniHostReply): Boolean {
        when (method) {
            "getCenterLocation" -> reply.success(LatLng(m.centerLat, m.centerLng).json())
            "getScale" -> reply.success(JSONObject().put("scale", (m.zoom * 100).toLong() / 100.0))
            "getRegion" -> reply.success(m.region())
            "getRotate" -> reply.success(JSONObject().put("rotate", 0))
            "getSkew" -> reply.success(JSONObject().put("skew", 0))
            "moveToLocation" -> {
                val lat = a.optDouble("latitude")
                val lng = a.optDouble("longitude")
                // 不给坐标：官方挪到当前定位点。有定位权限时用蓝点的位置（没开 show-location 用系统缓存的最后位置），
                // 没权限 / 还没定到位时退回属性里的中心点
                val t = if (!lat.isNaN() && !lng.isNaN()) LatLng(lat, lng)
                    else m.location?.fix ?: m.hostContext?.let { MapLocation.lastKnown(it) } ?: attrCenter(m)
                if (t == null || !t.valid) return reply.fail("moveToLocation:fail 没有可挪去的位置").let { true }
                m.moveTo(t, m.zoom, animate = true)
                reply.success()
            }
            "openMapApp" -> openMapApp(m, a, reply)
            "includePoints" -> {
                val pts = MapGeo.points(a.optJSONArray("points"))
                if (pts.isEmpty()) return reply.fail("includePoints:fail points 为空").let { true }
                val p = a.optJSONArray("padding")
                val pad = FloatArray(4) { i -> MiniMapView.arr(p, i, if (p != null && p.length() == 1) MiniMapView.arr(p, 0, 0f) else 0f) }
                m.fit(pts, pad, animate = true)
                reply.success()
            }
            "translateMarker" -> {
                val mk = find(m, a.optLong("markerId", -1)) ?: return reply.fail("translateMarker:fail 没有这个 markerId").let { true }
                val d = a.optJSONObject("destination")
                val to = LatLng(d?.optDouble("latitude") ?: Double.NaN, d?.optDouble("longitude") ?: Double.NaN)
                if (!to.valid) return reply.fail("translateMarker:fail destination 不对").let { true }
                if (a.has("rotate")) mk.rotate = a.optDouble("rotate").toFloat()
                slide(m, mk, listOf(mk.pos, to), a.optLong("duration", 1000))
                reply.success()
            }
            "moveAlong" -> {
                val mk = find(m, a.optLong("markerId", -1)) ?: return reply.fail("moveAlong:fail 没有这个 markerId").let { true }
                val path = MapGeo.points(a.optJSONArray("path"))
                if (path.size < 2) return reply.fail("moveAlong:fail path 至少两个点").let { true }
                slide(m, mk, path, a.optLong("duration", 1000))
                reply.success()
            }
            "addMarkers" -> {
                if (a.optBoolean("clear")) {
                    m.markers = emptyList()
                    m.extraMarkers.clear()
                }
                val list = a.optJSONArray("markers") ?: JSONArray()
                for (i in 0 until list.length()) {
                    val mk = list.optJSONObject(i)?.let(::MapMarker)?.takeIf { it.pos.valid } ?: continue
                    m.extraMarkers.removeAll { it.id == mk.id }
                    m.extraMarkers += mk
                }
                m.invalidate()
                reply.success()
            }
            "removeMarkers" -> {
                val ids = a.optJSONArray("markerIds") ?: JSONArray()
                val set = (0 until ids.length()).map { ids.optLong(it) }.toSet()
                m.extraMarkers.removeAll { it.id in set }
                m.markers = m.markers.filter { it.id !in set }
                m.invalidate()
                reply.success()
            }
            "toScreenLocation" -> {
                val (x, y) = m.toScreen(LatLng(a.optDouble("latitude"), a.optDouble("longitude")))
                reply.success(JSONObject().put("x", x.toDouble()).put("y", y.toDouble()))
            }
            "fromScreenLocation" -> reply.success(m.fromScreen(a.optDouble("x").toFloat(), a.optDouble("y").toFloat()).json())
            "setCenterOffset" -> {
                val o = a.optJSONArray("offset")
                m.offsetX = MiniMapView.clamp01(MiniMapView.arr(o, 0, 0.5f)).toDouble().coerceIn(0.25, 0.75)
                m.offsetY = MiniMapView.clamp01(MiniMapView.arr(o, 1, 0.5f)).toDouble().coerceIn(0.25, 0.75)
                m.invalidate()
                reply.success()
            }
            else -> return false
        }
        return true
    }

    /** preferApplication → 包名（装了就直接交给它，没装退回系统选择） */
    private val PACKAGES = mapOf(
        "amap" to "com.autonavi.minimap", "baidu" to "com.baidu.BaiduMap", "tencent" to "com.tencent.map",
        "google" to "com.google.android.apps.maps", "petal" to "com.huawei.maps.app",
    )

    /**
     * `openMapApp {latitude, longitude, destination, preferApplication}`：`geo:` Intent 交给系统，装了几个地图 App 由系统弹选择。
     * 坐标原样给（GCJ-02）：高德 / 腾讯 / 百度按 GCJ-02 认没有偏差，Google 地图在国内也按 GCJ-02 显示；国外坐标两者相同
     */
    private fun openMapApp(m: MiniMapView, a: JSONObject, reply: MiniHostReply) {
        val lat = a.optDouble("latitude")
        val lng = a.optDouble("longitude")
        if (!LatLng(lat, lng).valid) return reply.fail("openMapApp:fail latitude / longitude 不对")
        val ctx = m.hostContext ?: return reply.fail("openMapApp:fail 地图还没挂上")
        val name = a.optString("destination")
        val uri = Uri.parse("geo:$lat,$lng?q=$lat,$lng" + if (name.isNotEmpty()) "(${Uri.encode(name)})" else "")
        fun intent(pkg: String?) = Intent(Intent.ACTION_VIEW, uri).apply {
            if (pkg != null) setPackage(pkg)
            if (ctx !is Activity) addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        val pkg = PACKAGES[a.optString("preferApplication")]
        for (p in listOf(pkg, null).distinct()) {
            try {
                ctx.startActivity(intent(p))
                android.util.Log.i("mini-render", "🗺 openMapApp ${p ?: "系统选择"} $uri")
                return reply.success()
            } catch (e: ActivityNotFoundException) {
                // 指定的 App 没装：退回系统选择
            } catch (e: SecurityException) {
                break
            }
        }
        reply.fail("openMapApp:fail 没有可用的地图 App")
    }

    private fun attrCenter(m: MiniMapView): LatLng? {
        val lat = MapGeo.num(m.attrs, "latitude") ?: return null
        val lng = MapGeo.num(m.attrs, "longitude") ?: return null
        return LatLng(lat, lng)
    }

    private fun find(m: MiniMapView, id: Long) = (m.extraMarkers + m.markers).firstOrNull { it.id == id }

    /** 标记沿折线匀速走过去（按段长分配时间） */
    private fun slide(m: MiniMapView, mk: MapMarker, path: List<LatLng>, ms: Long) {
        val seg = path.zipWithNext { p, q -> Math.hypot(q.lat - p.lat, q.lng - p.lng) }
        val total = seg.sum().takeIf { it > 0 } ?: return
        ValueAnimator.ofFloat(0f, 1f).apply {
            duration = ms.coerceAtLeast(16)
            addUpdateListener {
                var d = (it.animatedValue as Float) * total
                var i = 0
                while (i < seg.size - 1 && d > seg[i]) { d -= seg[i]; i++ }
                val f = if (seg[i] > 0) (d / seg[i]).coerceIn(0.0, 1.0) else 1.0
                val p = path[i]
                val q = path[i + 1]
                mk.pos = LatLng(p.lat + (q.lat - p.lat) * f, p.lng + (q.lng - p.lng) * f)
                m.invalidate()
            }
            start()
        }
    }
}
