package dev.minirender

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.Rect
import android.graphics.RectF
import kotlin.math.atan2
import kotlin.math.floor
import kotlin.math.hypot
import kotlin.math.ln
import kotlin.math.max
import kotlin.math.pow

/**
 * [MiniMapView] 画一帧：底色 → 瓦片（缺的拿上一级拉伸顶着）→ 多边形 → 圆 → 折线 → 定位蓝点 → 标记 → 标签 →
 * 气泡（callout / title / customCallout）→ 控件 → 比例尺 → 版权。气泡、标签、控件、蓝点在 `MapBubbles.kt`（缓存好的位图）。
 * 画布先按 density 缩放，下面全用逻辑像素。顺手把标记 / 气泡 / 标签 / 控件的位置记进 `hits`（点击按它判）。
 */
internal object MapDraw {
    /** 瓦片没到时的底色（也是 tools/android-native-align.py 认地图的颜色） */
    const val BACKGROUND = 0xFFE8EEF5.toInt()

    private val fill = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
    private val stroke = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; strokeJoin = Paint.Join.ROUND; strokeCap = Paint.Cap.ROUND }
    private val bmpPaint = Paint(Paint.FILTER_BITMAP_FLAG or Paint.ANTI_ALIAS_FLAG)
    private val text = Paint(Paint.ANTI_ALIAS_FLAG)

    /** 返回 true 表示这一帧是缩放中的快路（不滤波），调用方过一会儿要再画一次 */
    fun draw(m: MiniMapView, canvas: Canvas, wPx: Int, hPx: Int, density: Float): Boolean {
        canvas.drawColor(BACKGROUND)
        // 缩放级别在变（捏合 / 缩放动画）：瓦片走不滤波的快路，停下来再按缩好的画一遍
        val moving = m.zoom != m.lastDrawZoom && !m.lastDrawZoom.isNaN()
        m.lastDrawZoom = m.zoom
        // 瓦片按设备像素画（画布缩放之前）：一张瓦片整像素对齐、原样贴
        tiles(m, canvas, density, moving)
        canvas.save()
        canvas.scale(density, density)
        m.hits.clear()
        // polygons 在解析时已按 zIndex 排好（稳定排序，同值按数组顺序）
        for (p in m.polygons) polygon(m, canvas, p)
        for (c in m.circles) circle(m, canvas, c)
        for (l in m.lines) line(m, canvas, l)
        if (m.showLocation) MapBubbles.location(m, canvas)
        val all = (m.markers + m.extraMarkers).sortedBy { it.zIndex }
        for (mk in all) marker(m, canvas, mk)
        for (mk in all) mk.label?.let { MapBubbles.label(m, canvas, mk, it, density) }
        for (mk in all) {
            val cc = mk.custom
            if (cc != null) {
                if (cc.always || mk.id in m.openCallouts) MapBubbles.custom(m, canvas, mk, cc)
            } else mk.callout?.takeIf { it.always || mk.id in m.openCallouts }?.let { MapBubbles.callout(m, canvas, mk, it, density) }
        }
        // 控件不随地图动、画在最上面（点击也最先判）
        MapBubbles.controls(m, canvas)
        if (m.showScale) scaleBar(m, canvas)
        attribution(m, canvas)
        canvas.restore()
        return moving
    }

    private val fastPaint = Paint()

    private fun tiles(m: MiniMapView, canvas: Canvas, density: Float, moving: Boolean) {
        val t = m.tiles ?: return
        val src = MiniMapView.tileSource
        // 瓦片级别：让一张瓦片落到屏上 256~512 个设备像素（高分屏上用高一级的瓦片，不糊）
        val tz = floor(m.zoom + ln(density.toDouble()) / ln(2.0)).toInt().coerceIn(0, src.maxZoom)
        // 以下全是设备像素
        val size = MapGeo.TILE * 2.0.pow(m.zoom - tz) * density
        val cx = (MapGeo.x(m.centerLng, m.zoom) - m.width * m.offsetX) * density
        val cy = (MapGeo.y(m.centerLat, m.zoom) - m.height * m.offsetY) * density
        val x0 = floor(cx / size).toInt()
        val y0 = floor(cy / size).toInt()
        val x1 = floor((cx + m.width * density) / size).toInt()
        val y1 = floor((cy + m.height * density) / size).toInt()
        // 缩好的瓦片多 1 像素：各自取整摆放时相邻两张最多叠 1 像素，不会漏出底色缝
        val px = kotlin.math.ceil(size).toInt() + 1
        var missing = false
        val dst = RectF()
        val srcRect = Rect()
        for (ty in y0..y1) for (tx in x0..x1) {
            val left = Math.round(tx * size - cx).toFloat()
            val top = Math.round(ty * size - cy).toFloat()
            if (!moving) {
                val sb = t.getScaled(tx, ty, tz, m.satellite, px)
                if (sb != null) {
                    canvas.drawBitmap(sb, left, top, null)
                    continue
                }
            } else {
                val b = t.get(tx, ty, tz, m.satellite)
                if (b != null) {
                    dst.set(left, top, left + px, top + px)
                    canvas.drawBitmap(b, null, dst, fastPaint)
                    continue
                }
            }
            missing = true
            val (pb, d) = t.parent(tx, ty, tz, m.satellite) ?: continue
            val n = 1 shl d
            val part = pb.width / n
            val ox = Math.floorMod(tx, n) * part
            val oy = Math.floorMod(ty, n) * part
            srcRect.set(ox, oy, ox + part, oy + part)
            dst.set(left, top, left + px, top + px)
            canvas.drawBitmap(pb, srcRect, dst, if (moving) fastPaint else bmpPaint)
        }
        m.missing = missing
    }

    private fun path(m: MiniMapView, pts: List<LatLng>, close: Boolean): Path {
        val p = Path()
        pts.forEachIndexed { i, ll ->
            val (x, y) = m.toScreen(ll)
            if (i == 0) p.moveTo(x, y) else p.lineTo(x, y)
        }
        if (close) p.close()
        return p
    }

    private fun polygon(m: MiniMapView, c: Canvas, g: MapPolygon) {
        if (g.points.size < 3) return
        val p = path(m, g.points, true)
        fill.color = g.fill
        c.drawPath(p, fill)
        if (g.stroke > 0) {
            stroke.color = g.strokeColor; stroke.strokeWidth = g.stroke; stroke.pathEffect = g.dash
            c.drawPath(p, stroke)
            stroke.pathEffect = null
        }
    }

    private fun circle(m: MiniMapView, c: Canvas, k: MapCircle) {
        if (!k.center.valid || k.radius <= 0) return
        val (x, y) = m.toScreen(k.center)
        val r = (k.radius / MapGeo.metersPerPx(k.center.lat, m.zoom)).toFloat()
        fill.color = k.fill
        c.drawCircle(x, y, r, fill)
        if (k.stroke > 0) {
            stroke.color = k.color; stroke.strokeWidth = k.stroke; stroke.pathEffect = null
            c.drawCircle(x, y, r, stroke)
        }
    }

    private fun line(m: MiniMapView, c: Canvas, l: MapLine) {
        if (l.points.size < 2) return
        val p = path(m, l.points, false)
        if (l.borderWidth > 0) {
            stroke.color = l.borderColor; stroke.strokeWidth = l.width + l.borderWidth * 2; stroke.pathEffect = null
            c.drawPath(p, stroke)
        }
        stroke.strokeWidth = l.width
        stroke.pathEffect = l.dash
        if (l.colorList.isEmpty()) {
            stroke.color = l.color
            c.drawPath(p, stroke)
        } else {
            // 彩虹线：一段一个颜色，不够的段用最后一个（官方）
            for (i in 0 until l.points.size - 1) {
                val (ax, ay) = m.toScreen(l.points[i])
                val (bx, by) = m.toScreen(l.points[i + 1])
                stroke.color = l.colorList[minOf(i, l.colorList.size - 1)]
                c.drawLine(ax, ay, bx, by, stroke)
            }
        }
        stroke.pathEffect = null
        if (!l.arrow) return
        val icon = l.arrowIconPath.takeIf { it.isNotBlank() }?.let { p -> MapTiles.icon(p, m.resolvePath(p)) { m.invalidate() } }
        if (icon != null) return arrowIcons(m, c, l, icon)
        // 箭头：每隔 60 逻辑像素一个白色「>」，朝走向
        stroke.color = Color.WHITE
        stroke.strokeWidth = max(1.2f, l.width / 3)
        val s = l.width * 0.6f
        for ((a, b) in l.points.zipWithNext()) {
            val (ax, ay) = m.toScreen(a)
            val (bx, by) = m.toScreen(b)
            val len = hypot(bx - ax, by - ay)
            val ang = atan2(by - ay, bx - ax)
            var d = 30f
            while (d < len) {
                val x = ax + (bx - ax) * d / len
                val y = ay + (by - ay) * d / len
                c.save()
                c.rotate(Math.toDegrees(ang.toDouble()).toFloat(), x, y)
                c.drawLine(x - s / 2, y - s, x + s / 2, y, stroke)
                c.drawLine(x + s / 2, y, x - s / 2, y + s, stroke)
                c.restore()
                d += 60f
            }
        }
    }

    private fun marker(m: MiniMapView, c: Canvas, mk: MapMarker) {
        val (x, y) = m.toScreen(mk.pos)
        val icon: Bitmap? = mk.iconPath.takeIf { it.isNotBlank() }?.let { p -> MapTiles.icon(p, m.resolvePath(p)) { m.invalidate() } }
        val w = if (!mk.width.isNaN()) mk.width else icon?.let { it.width / 2f } ?: 28f
        val h = if (!mk.height.isNaN()) mk.height else icon?.let { it.height / 2f } ?: 36f
        val r = RectF(x - w * mk.anchorX, y - h * mk.anchorY, x - w * mk.anchorX + w, y - h * mk.anchorY + h)
        if (r.right < -50 || r.left > m.width + 50 || r.bottom < -50 || r.top > m.height + 50) return
        c.save()
        if (mk.rotate != 0f) c.rotate(mk.rotate, x, y)
        if (icon != null) {
            bmpPaint.alpha = (mk.alpha * 255).toInt()
            c.drawBitmap(icon, null, r, bmpPaint)
            bmpPaint.alpha = 255
        } else pin(c, r, mk.alpha)
        c.restore()
        m.hits += MapHit(MapHit.MARKER, mk.id, RectF(r).apply { inset(-6f, -6f) })
    }

    /** 默认标记：红色水滴 + 白点 */
    private fun pin(c: Canvas, r: RectF, alpha: Float) {
        val cx = r.centerX()
        val rad = r.width() / 2
        val p = Path()
        p.moveTo(cx, r.bottom)
        p.cubicTo(cx - rad * 0.3f, r.bottom - r.height() * 0.3f, r.left, r.top + rad * 1.4f, r.left, r.top + rad)
        p.arcTo(RectF(r.left, r.top, r.right, r.top + rad * 2), 180f, 180f)
        p.cubicTo(r.right, r.top + rad * 1.4f, cx + rad * 0.3f, r.bottom - r.height() * 0.3f, cx, r.bottom)
        fill.color = Color.argb((alpha * 255).toInt(), 0xF5, 0x3B, 0x3B)
        c.drawPath(p, fill)
        fill.color = Color.argb((alpha * 255).toInt(), 255, 255, 255)
        c.drawCircle(cx, r.top + rad, rad * 0.38f, fill)
    }

    private val arrowDst = RectF()

    /** arrowIconPath：图标朝走向、每隔 60 逻辑像素一个，高度跟线宽（至少 8） */
    private fun arrowIcons(m: MiniMapView, c: Canvas, l: MapLine, icon: Bitmap) {
        val s = max(8f, l.width)
        val w = s * icon.width / icon.height.coerceAtLeast(1)
        for (i in 0 until l.points.size - 1) {
            val (ax, ay) = m.toScreen(l.points[i])
            val (bx, by) = m.toScreen(l.points[i + 1])
            val len = hypot(bx - ax, by - ay)
            val deg = Math.toDegrees(atan2(by - ay, bx - ax).toDouble()).toFloat()
            var d = 30f
            while (d < len) {
                val x = ax + (bx - ax) * d / len
                val y = ay + (by - ay) * d / len
                c.save()
                c.rotate(deg, x, y)
                arrowDst.set(x - w / 2, y - s / 2, x + w / 2, y + s / 2)
                c.drawBitmap(icon, null, arrowDst, bmpPaint)
                c.restore()
                d += 60f
            }
        }
    }

    private val NICE = longArrayOf(5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10_000, 20_000, 50_000, 100_000, 200_000, 500_000, 1_000_000)

    private fun scaleBar(m: MiniMapView, c: Canvas) {
        val mpp = MapGeo.metersPerPx(m.centerLat, m.zoom)
        val meters = NICE.lastOrNull { it / mpp <= 90 } ?: NICE[0]
        val w = (meters / mpp).toFloat()
        val x = 12f
        val y = m.height - 14f
        stroke.color = 0xFF333333.toInt(); stroke.strokeWidth = 2f; stroke.pathEffect = null
        c.drawLine(x, y, x + w, y, stroke)
        c.drawLine(x, y - 4, x, y, stroke)
        c.drawLine(x + w, y - 4, x + w, y, stroke)
        text.color = 0xFF333333.toInt()
        text.textSize = 11f
        c.drawText(if (meters >= 1000) "${meters / 1000} 公里" else "$meters 米", x, y - 6, text)
    }

    private fun attribution(m: MiniMapView, c: Canvas) {
        val s = MiniMapView.tileSource.attribution.takeIf { it.isNotEmpty() } ?: return
        text.textSize = 9f
        text.color = 0x99333333.toInt()
        c.drawText(s, m.width - text.measureText(s) - 6f, m.height - 5f, text)
    }
}
