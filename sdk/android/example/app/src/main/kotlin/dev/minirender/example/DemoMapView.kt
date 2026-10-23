package dev.minirender.example

import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import dev.minirender.MiniSameLayerView
import org.json.JSONObject
import kotlin.math.hypot

/**
 * 示例的「地图」：同层渲染的 `<map>`（与 Flutter 示例的 `_FakeMap` 同一个样子：网格、两条路、中间一个标记、
 * 底部经纬度）。真实 App 在这里画高德 / Google 地图的那一帧（它们的截图 / TextureView 接口）。
 *
 * 画进页面而不是盖在画面上：滑动时它跟页面在同一张图里走，一个像素都不错开，宿主也一帧都不用重画。
 * 点中间的标记回传 `markertap {markerId: 1}`（页面的 `bindmarkertap`）。
 */
class DemoMapView : MiniSameLayerView() {
    private var lat = 0.0
    private var lng = 0.0
    private val bg = Paint().apply { color = 0xFFE8EEF5.toInt() }
    private val grid = Paint().apply { color = 0xFFCAD6E3.toInt() }
    private val road = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.WHITE }
    private val pin = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = 0xFFE64340.toInt() }
    private val label = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = 0x8A000000.toInt() }

    override fun onAttrs(attrs: JSONObject) {
        lat = attrs.optString("latitude").toDoubleOrNull() ?: 0.0
        lng = attrs.optString("longitude").toDoubleOrNull() ?: 0.0
        invalidate()
    }

    override fun onDraw(canvas: Canvas, width: Int, height: Int, density: Float) {
        val (w, h) = width.toFloat() to height.toFloat()
        // 圆角由引擎按节点的 border-radius 裁（这里画满就行）
        canvas.drawRect(0f, 0f, w, h, bg)
        grid.strokeWidth = density
        val step = 24 * density
        var x = 0f
        while (x < w) { canvas.drawLine(x, 0f, x, h, grid); x += step }
        var y = 0f
        while (y < h) { canvas.drawLine(0f, y, w, y, grid); y += step }
        road.strokeWidth = 8 * density
        canvas.drawLine(0f, h * 0.62f, w, h * 0.38f, road)
        canvas.drawLine(w * 0.3f, 0f, w * 0.45f, h, road)
        // 标记：一个水滴形的钉子
        val (cx, cy, r) = Triple(w / 2, h / 2, 12 * density)
        val drop = Path().apply {
            addArc(RectF(cx - r, cy - 2 * r, cx + r, cy), 150f, 240f)
            lineTo(cx, cy + r * 0.9f)
            close()
        }
        canvas.drawPath(drop, pin)
        canvas.drawCircle(cx, cy - r, r * 0.4f, Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.WHITE })
        label.textSize = 11 * density
        canvas.drawText("宿主原生 map（同层渲染） %.4f, %.4f".format(lat, lng), 8 * density, h - 6 * density, label)
    }

    /** 点在标记附近（逻辑像素 24 以内）算点中 */
    override fun onTap(x: Float, y: Float) {
        if (hypot(x - width / 2, y - (height / 2 - 12)) <= 24) emit("markertap", JSONObject().put("markerId", 1))
    }
}
