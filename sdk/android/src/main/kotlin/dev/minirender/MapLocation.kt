package dev.minirender

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Bundle
import android.os.Looper
import android.util.Log

/**
 * `<map show-location>` 的定位蓝点（官方：显示带有方向的当前定位点，要用户位置授权）。
 *
 * - **不申请权限**：SDK 的清单不声明定位权限（见 AndroidManifest.xml 的说明）。宿主声明了
 *   `ACCESS_FINE_LOCATION` / `ACCESS_COARSE_LOCATION` 并且用户已经同意，才开始定位；没有就不显示蓝点、不报错
 *   （微信 3.13.2 起会自动弹授权，这里留给宿主自己弹）。
 * - 系统定位给的是 WGS-84，地图是 GCJ-02：画之前换算（[MapGeo.wgsToGcj]）。
 * - 方向用 `Location.bearing`（移动中才有）；静止时没有方向，只画圆点。没接罗盘（要传感器，耗电）。
 * - 只在主线程上调；[stop] 在地图拿掉 / 关掉 show-location 时调，不留后台定位。
 */
internal class MapLocation(private val context: Context, private val changed: () -> Unit) {
    /** 最近一次的位置（GCJ-02）、精度（米）、方向（度，没有为 NaN） */
    var fix: LatLng? = null
        private set
    var accuracy = 0f
        private set
    var bearing = Float.NaN
        private set
    private var lm: LocationManager? = null

    private val listener = object : LocationListener {
        override fun onLocationChanged(l: Location) = take(l)
        @Deprecated("API 29 起不再调")
        override fun onStatusChanged(provider: String?, status: Int, extras: Bundle?) {}
        override fun onProviderEnabled(provider: String) {}
        override fun onProviderDisabled(provider: String) {}
    }

    private fun take(l: Location) {
        fix = MapGeo.wgsToGcj(l.latitude, l.longitude)
        accuracy = if (l.hasAccuracy()) l.accuracy else 0f
        bearing = if (l.hasBearing()) l.bearing else Float.NaN
        changed()
    }

    @SuppressLint("MissingPermission") // 上面 granted() 判过；运行中被收回走 SecurityException
    fun start() {
        if (lm != null || !granted(context)) return
        val m = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager ?: return
        try {
            for (p in listOf(LocationManager.GPS_PROVIDER, LocationManager.NETWORK_PROVIDER)) {
                if (!m.isProviderEnabled(p)) continue
                m.getLastKnownLocation(p)?.let { if (fix == null) take(it) }
                m.requestLocationUpdates(p, 1000L, 1f, listener, Looper.getMainLooper())
            }
            lm = m
        } catch (e: SecurityException) {
            // 运行中被收回了权限：不显示，不崩
            Log.w("mini-render", "🗺 定位权限不可用：$e")
            runCatching { m.removeUpdates(listener) }
        } catch (e: IllegalArgumentException) {
            Log.w("mini-render", "🗺 定位源不可用：$e")
        }
    }

    fun stop() {
        lm?.let { runCatching { it.removeUpdates(listener) } }
        lm = null
    }

    companion object {
        fun granted(c: Context): Boolean =
            c.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED ||
                c.checkSelfPermission(Manifest.permission.ACCESS_COARSE_LOCATION) == PackageManager.PERMISSION_GRANTED

        /** 没开 show-location 时 `moveToLocation()` 用：系统缓存的最后位置（GCJ-02），没权限 / 没有给 null */
        @SuppressLint("MissingPermission")
        fun lastKnown(c: Context): LatLng? {
            if (!granted(c)) return null
            val m = c.getSystemService(Context.LOCATION_SERVICE) as? LocationManager ?: return null
            return try {
                listOf(LocationManager.GPS_PROVIDER, LocationManager.NETWORK_PROVIDER)
                    .mapNotNull { runCatching { m.getLastKnownLocation(it) }.getOrNull() }
                    .maxByOrNull { it.time }
                    ?.let { MapGeo.wgsToGcj(it.latitude, it.longitude) }
            } catch (e: SecurityException) {
                null
            }
        }
    }
}
