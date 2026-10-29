package dev.minirender

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.view.MotionEvent
import android.view.View

/**
 * 小游戏右上角的胶囊「••• | ⊙」（与小程序引擎画的那个同一套尺寸与配色，src/host/capsule.rs；iOS 是 GameCapsule.swift）。
 *
 * 原生 View 叠在游戏 SurfaceView 上面，不画进游戏画面：WebGL 直接上屏时引擎往帧里叠东西要每帧多一次整帧合成
 * （game/webgl/window.rs 的覆盖层路径），原生 View 由系统合成，游戏那一帧不多花一点。
 * 位置由 [MiniGameView] 按引擎给的矩形摆（`nativeMenuButton`，与 wx.getMenuButtonBoundingClientRect 同一个数）。
 * 落在胶囊里的触摸归胶囊，不交给游戏（与微信一致）。[dark]：wx.setMenuStyle('dark')（深底白图标）。
 */
internal class GameCapsule(context: Context, private val onTap: (action: String) -> Unit) : View(context) {
    private val density = resources.displayMetrics.density
    var dark = false
        set(v) { field = v; invalidate() }
    /** 按着的那一半：0 = 没按，1 = 「•••」，2 = 「⊙」 */
    private var pressedHalf = 0
    private val fill = Paint(Paint.ANTI_ALIAS_FLAG)
    private val stroke = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE }
    private val r = RectF()

    /** 按逻辑像素矩形摆（父视图是 MiniGameView，坐标同视图内逻辑像素） */
    fun place(x: Float, y: Float, w: Float, h: Float) {
        if (w <= 0f || h <= 0f) { visibility = GONE; return }
        visibility = VISIBLE
        val lp = layoutParams as? android.widget.FrameLayout.LayoutParams ?: android.widget.FrameLayout.LayoutParams(0, 0)
        lp.width = (w * density).toInt()
        lp.height = (h * density).toInt()
        lp.leftMargin = (x * density).toInt()
        lp.topMargin = (y * density).toInt()
        layoutParams = lp
    }

    private fun c(a: Int, rgb: Int) = Color.argb(a, rgb, rgb, rgb)

    override fun onDraw(canvas: Canvas) {
        val w = width.toFloat()
        val h = height.toFloat()
        val s = density
        val hair = (0.5f * s).coerceAtLeast(1f)
        val bg = if (dark) c(38, 0) else c(153, 255)
        val line = if (dark) c(64, 255) else c(20, 0)
        val divider = if (dark) c(64, 255) else c(51, 0)
        val icon = if (dark) Color.WHITE else Color.BLACK
        val press = if (dark) c(46, 255) else c(26, 0)
        val rad = h / 2f
        r.set(0f, 0f, w, h)
        fill.color = bg
        canvas.drawRoundRect(r, rad, rad, fill)
        if (pressedHalf != 0) {
            canvas.save()
            if (pressedHalf == 1) canvas.clipRect(0f, 0f, w / 2f, h) else canvas.clipRect(w / 2f, 0f, w, h)
            fill.color = press
            canvas.drawRoundRect(r, rad, rad, fill)
            canvas.restore()
        }
        stroke.color = line
        stroke.strokeWidth = hair
        r.set(hair / 2f, hair / 2f, w - hair / 2f, h - hair / 2f)
        canvas.drawRoundRect(r, rad - hair / 2f, rad - hair / 2f, stroke)
        // 竖分隔线：高度占 18/32
        fill.color = divider
        val dh = h * 18f / 32f
        canvas.drawRect(w / 2f - hair / 2f, (h - dh) / 2f, w / 2f + hair / 2f, (h + dh) / 2f, fill)
        // 左半：三个点，中间那个大一圈
        fill.color = icon
        val cy = h / 2f
        val mx = w / 4f
        canvas.drawCircle(mx - 6.2f * s, cy, 1.7f * s, fill)
        canvas.drawCircle(mx, cy, 2.4f * s, fill)
        canvas.drawCircle(mx + 6.2f * s, cy, 1.7f * s, fill)
        // 右半：圆环 + 圆心
        val cx = w * 0.75f
        stroke.color = icon
        stroke.strokeWidth = 1.9f * s
        canvas.drawCircle(cx, cy, (7.4f - 1.9f / 2f) * s, stroke)
        canvas.drawCircle(cx, cy, 2.7f * s, fill)
    }

    private fun halfAt(x: Float, y: Float): Int =
        if (x < 0 || y < 0 || x >= width || y >= height) 0 else if (x < width / 2f) 1 else 2

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(e: MotionEvent): Boolean {
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN -> pressedHalf = halfAt(e.x, e.y)
            MotionEvent.ACTION_MOVE -> {
                // 移出按下的那一半就不算点击（与引擎画的胶囊同一条规则）
                if (pressedHalf != 0 && halfAt(e.x, e.y) != pressedHalf) pressedHalf = 0
            }
            MotionEvent.ACTION_UP -> {
                val h = pressedHalf
                pressedHalf = 0
                if (h != 0 && halfAt(e.x, e.y) == h) onTap(if (h == 1) "menu" else "close")
            }
            MotionEvent.ACTION_CANCEL -> pressedHalf = 0
        }
        invalidate()
        return true
    }
}
