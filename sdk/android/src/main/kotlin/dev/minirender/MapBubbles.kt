package dev.minirender

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import java.io.File
import kotlin.math.ceil

/**
 * [MapDraw] 里「贴在地图上的小块」：气泡（callout / title）、标签（label）、自定义气泡（customCallout）、
 * 控件（controls）、定位蓝点（show-location）。
 *
 * **每帧只贴图**：气泡 / 标签的文字、圆角、阴影按 dpr 画进一张位图（挂在 [MapBubble.bitmap] 上，属性变了随对象一起换），
 * 拖动 / 缩放的每一帧只是一次 drawBitmap —— 从前每帧每个气泡量一遍字、画一遍阴影和字。
 * 控件与自定义气泡本来就是图（[MapTiles.icon] 的缓存），定位蓝点是几个圆。这里没有每帧分配位图的路径。
 */
internal object MapBubbles {
    /** 位图四周给阴影留的边（逻辑像素） */
    private const val SHADOW = 4f
    private const val TAIL = 6f

    private val fill = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
    private val stroke = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE }
    private val text = Paint(Paint.ANTI_ALIAS_FLAG)
    private val bmpPaint = Paint(Paint.FILTER_BITMAP_FLAG or Paint.ANTI_ALIAS_FLAG)
    private val dst = RectF()

    /** 气泡画成位图（同一个 dpr 只画一次） */
    private fun ensure(b: MapBubble, density: Float, tail: Boolean): Bitmap? {
        b.bitmap?.let { if (b.bitmapDensity == density) return it }
        text.textSize = b.fontSize
        val lines = b.content.split('\n')
        val widths = lines.map { text.measureText(it) }
        val pad = b.padding.coerceAtLeast(4f)
        val w = (widths.maxOrNull() ?: 0f) + pad * 2
        val h = b.fontSize * 1.3f * lines.size + pad * 2
        val bw = ceil((w + SHADOW * 2) * density).toInt()
        val bh = ceil((h + SHADOW * 2 + if (tail) TAIL else 0f) * density).toInt()
        // 一个气泡最多 2000×2000 设备像素（16MB）：再大是写错了的内容，不画
        if (bw <= 0 || bh <= 0 || bw.toLong() * bh > 4_000_000L) return null
        val bmp = Bitmap.createBitmap(bw, bh, Bitmap.Config.ARGB_8888)
        val c = Canvas(bmp).apply { this.density = Bitmap.DENSITY_NONE }
        c.scale(density, density)
        val r = RectF(SHADOW, SHADOW, SHADOW + w, SHADOW + h)
        fill.color = b.bg
        fill.setShadowLayer(3f, 0f, 1f, 0x33000000)
        c.drawRoundRect(r, b.radius, b.radius, fill)
        fill.clearShadowLayer()
        if (tail) {
            val p = Path()
            p.moveTo(r.centerX() - 6, r.bottom - 0.5f); p.lineTo(r.centerX(), r.bottom + TAIL); p.lineTo(r.centerX() + 6, r.bottom - 0.5f); p.close()
            c.drawPath(p, fill)
        }
        if (b.borderWidth > 0) {
            stroke.color = b.borderColor; stroke.strokeWidth = b.borderWidth
            c.drawRoundRect(r, b.radius, b.radius, stroke)
        }
        text.color = b.color
        lines.forEachIndexed { i, s ->
            val x = when (b.textAlign) {
                "center" -> r.left + (w - widths[i]) / 2
                "right" -> r.right - pad - widths[i]
                else -> r.left + pad
            }
            c.drawText(s, x, r.top + pad + b.fontSize * (1.3f * i + 1f), text)
        }
        b.bitmap = bmp
        b.bitmapDensity = density
        b.boxW = w
        b.boxH = h
        return bmp
    }

    /** 位图贴到气泡框 (left, top) 处（框外是阴影边） */
    private fun blit(c: Canvas, bmp: Bitmap, left: Float, top: Float, density: Float) {
        dst.set(left - SHADOW, top - SHADOW, left - SHADOW + bmp.width / density, top - SHADOW + bmp.height / density)
        c.drawBitmap(bmp, null, dst, bmpPaint)
    }

    private fun markerH(mk: MapMarker) = if (!mk.height.isNaN()) mk.height else 36f

    /** callout / title：标记上方居中，再按 anchorX / anchorY 偏移 */
    fun callout(m: MiniMapView, c: Canvas, mk: MapMarker, b: MapBubble, density: Float) {
        if (b.content.isEmpty()) return
        val bmp = ensure(b, density, tail = true) ?: return
        val (x, y) = m.toScreen(mk.pos)
        val left = x - b.boxW / 2 + b.anchorX
        val top = y - markerH(mk) * mk.anchorY - 8f - b.boxH + b.anchorY
        if (offscreen(m, left, top, b.boxW, b.boxH)) return
        blit(c, bmp, left, top, density)
        m.hits += MapHit(MapHit.CALLOUT, mk.id, RectF(left, top, left + b.boxW, top + b.boxH))
    }

    /** label：官方 anchorX / anchorY 是标签左上角相对标记锚点的偏移 */
    fun label(m: MiniMapView, c: Canvas, mk: MapMarker, b: MapBubble, density: Float) {
        if (b.content.isEmpty()) return
        val bmp = ensure(b, density, tail = false) ?: return
        val (x, y) = m.toScreen(mk.pos)
        val left = x + b.anchorX
        val top = y + b.anchorY
        if (offscreen(m, left, top, b.boxW, b.boxH)) return
        blit(c, bmp, left, top, density)
        m.hits += MapHit(MapHit.LABEL, mk.id, RectF(left, top, left + b.boxW, top + b.boxH))
    }

    /** customCallout：引擎把 slot="callout" 里 marker-id 对上的 cover-view 画成了 PNG（[MiniMapView.customImages]） */
    fun custom(m: MiniMapView, c: Canvas, mk: MapMarker, cc: MapCustomCallout) {
        val img = m.customImages[mk.id] ?: return
        val bmp = MapTiles.icon(img.path, File(img.path)) { m.invalidate() } ?: return
        val (x, y) = m.toScreen(mk.pos)
        val left = x - img.w / 2 + cc.anchorX
        val top = y - markerH(mk) * mk.anchorY - 4f - img.h + cc.anchorY
        if (offscreen(m, left, top, img.w, img.h)) return
        dst.set(left, top, left + img.w, top + img.h)
        c.drawBitmap(bmp, null, dst, bmpPaint)
        m.hits += MapHit(MapHit.CALLOUT, mk.id, RectF(dst))
    }

    /** controls：按 position 钉在框里（不随地图动），可点的记进 hits */
    fun controls(m: MiniMapView, c: Canvas) {
        for (k in m.controls) {
            val icon = k.iconPath.takeIf { it.isNotBlank() }?.let { p -> MapTiles.icon(p, m.resolvePath(p)) { m.invalidate() } }
            val w = if (!k.width.isNaN()) k.width else icon?.let { it.width / 2f } ?: continue
            val h = if (!k.height.isNaN()) k.height else icon?.let { it.height / 2f } ?: continue
            dst.set(k.left, k.top, k.left + w, k.top + h)
            if (icon != null) c.drawBitmap(icon, null, dst, bmpPaint)
            if (k.clickable) m.hits += MapHit(MapHit.CONTROL, k.id, RectF(dst))
        }
    }

    private const val BLUE = 0xFF1A73E8.toInt()
    private val arrow = Path()

    /** show-location：精度圈 + 白边蓝点（有方向时加一个朝向的小三角） */
    fun location(m: MiniMapView, c: Canvas) {
        val loc = m.location ?: return
        val f = loc.fix ?: return
        val (x, y) = m.toScreen(f)
        if (x < -60 || y < -60 || x > m.width + 60 || y > m.height + 60) return
        val r = (loc.accuracy / MapGeo.metersPerPx(f.lat, m.zoom)).toFloat()
        if (r > 10f) {
            fill.color = 0x221A73E8
            c.drawCircle(x, y, r, fill)
            stroke.color = 0x661A73E8; stroke.strokeWidth = 1f
            c.drawCircle(x, y, r, stroke)
        }
        if (!loc.bearing.isNaN()) {
            c.save()
            c.rotate(loc.bearing, x, y)
            arrow.reset()
            arrow.moveTo(x, y - 15f); arrow.lineTo(x - 6f, y - 7f); arrow.lineTo(x + 6f, y - 7f); arrow.close()
            fill.color = BLUE
            c.drawPath(arrow, fill)
            c.restore()
        }
        fill.color = 0xFFFFFFFF.toInt()
        fill.setShadowLayer(2f, 0f, 0.5f, 0x44000000)
        c.drawCircle(x, y, 8f, fill)
        fill.clearShadowLayer()
        fill.color = BLUE
        c.drawCircle(x, y, 6f, fill)
    }

    private fun offscreen(m: MiniMapView, l: Float, t: Float, w: Float, h: Float) =
        l + w < -20 || t + h < -20 || l > m.width + 20 || t > m.height + 20
}
