package dev.minirender.example

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Color
import android.graphics.Outline
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.VelocityTracker
import android.view.View
import android.view.ViewConfiguration
import android.view.ViewOutlineProvider
import android.view.animation.DecelerateInterpolator
import android.widget.FrameLayout
import kotlin.math.abs
import kotlin.math.min

/**
 * 微信式的下拉面板容器：[content]（首页）在上、[panel]（深色小程序面板）在后。
 * 首页滚到顶（[contentAtTop]）再往下拉，首页跟着手指下沉（阻尼 [DAMPING]）、露出后面的面板；
 * 松手时拉过 [OPEN_DP] 就展开（首页沉到底只露 [PEEK_DP] 那一截），否则弹回。
 * 展开后：点 / 上推露出的那一截收起；面板滚到底再往上推也跟手收起（[panelAtBottom]）。
 * 与 Flutter 示例 lib/store/store_drawer.dart 同一套阈值与手感。
 *
 * 拦截在 onInterceptTouchEvent 里做：拉动开始之前触摸照常交给首页的 ScrollView（点按、滚动都不受影响），
 * 判定是「在顶上往下拉」之后才接管 —— 同一次拖动里首页内容与面板只有一个在动。
 */
class PullDrawerLayout(
    ctx: Context,
    private val panel: View,
    private val content: View,
    private val contentAtTop: () -> Boolean,
    private val panelAtBottom: () -> Boolean,
) : FrameLayout(ctx) {
    /** 展开 / 收起（动画结束时，true = 展开）：入口页据此换状态栏图标颜色 */
    var onStateChanged: ((Boolean) -> Unit)? = null

    private val scrim = View(ctx).apply { setBackgroundColor(Color.BLACK); alpha = 0f; visibility = GONE }
    private val slop = ViewConfiguration.get(ctx).scaledTouchSlop
    private var progress = 0f
    private var bottomInset = 0
    private var anim: ValueAnimator? = null
    private var tracker: VelocityTracker? = null
    private var downX = 0f
    private var downY = 0f
    private var lastY = 0f
    private var mode = Mode.NONE
    private var moved = false

    private enum class Mode { NONE, PULL, PEEK, PANEL }

    val isOpen get() = progress > 0f

    init {
        addView(panel, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
        addView(content, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
        addView(scrim, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
        content.outlineProvider = object : ViewOutlineProvider() {
            override fun getOutline(view: View, outline: Outline) =
                outline.setRoundRect(0, 0, view.width, view.height, dp(16f) * progress)
        }
        content.clipToOutline = true
        panel.visibility = INVISIBLE
        setOnApplyWindowInsetsListener { _, insets ->
            @Suppress("DEPRECATION")
            bottomInset = insets.systemWindowInsetBottom
            dispatchInsets(insets)
            insets
        }
    }

    /** 两个孩子都要拿到同样的 insets（各自让出状态栏 / 导航条） */
    private fun dispatchInsets(insets: android.view.WindowInsets) {
        panel.dispatchApplyWindowInsets(insets)
        content.dispatchApplyWindowInsets(insets)
    }

    /** 首页露在底下的高度（面板的内容要让出这么多） */
    val peekPx get() = dp(PEEK_DP).toInt() + bottomInset
    private val travel get() = (height - peekPx).coerceAtLeast(1).toFloat()

    fun open() = animateTo(1f)

    fun close() = animateTo(0f)

    private fun set(v: Float) {
        progress = v.coerceIn(0f, 1f)
        val p = progress
        content.translationY = p * travel
        scrim.translationY = content.translationY
        scrim.alpha = 0.25f * p
        scrim.visibility = if (p > 0f) VISIBLE else GONE
        content.invalidateOutline()
        panel.visibility = if (p > 0f) VISIBLE else INVISIBLE
        panel.alpha = min(1f, p * 1.6f)
        panel.translationY = -dp(48f) * (1 - p)
        panel.scaleX = 0.94f + 0.06f * p
        panel.scaleY = panel.scaleX
    }

    private fun animateTo(target: Float) {
        anim?.cancel()
        if (target == 1f && progress < 1f) performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY)
        anim = ValueAnimator.ofFloat(progress, target).apply {
            duration = 300
            interpolator = DecelerateInterpolator(1.6f)
            addUpdateListener { set(it.animatedValue as Float) }
            addListener(object : android.animation.AnimatorListenerAdapter() {
                override fun onAnimationEnd(a: android.animation.Animator) {
                    if (anim === a) anim = null
                    onStateChanged?.invoke(progress > 0f)
                }
            })
            start()
        }
    }

    override fun onInterceptTouchEvent(ev: MotionEvent): Boolean {
        if (anim != null) return true   // 动画中吃掉触摸，等它走完
        when (ev.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                downX = ev.x; downY = ev.y; lastY = ev.y; moved = false
                mode = Mode.NONE
                // 展开时露在底下那一截：整块归这里（点 = 收起，拖 = 跟手收）
                if (progress > 0f && ev.y >= content.translationY) {
                    mode = Mode.PEEK
                    return true
                }
            }
            MotionEvent.ACTION_MOVE -> {
                val dy = ev.y - downY
                val vertical = abs(dy) > slop && abs(dy) > abs(ev.x - downX)
                if (progress == 0f && vertical && dy > 0 && contentAtTop()) mode = Mode.PULL
                else if (progress == 1f && vertical && dy < 0 && panelAtBottom()) mode = Mode.PANEL
                if (mode != Mode.NONE) {
                    lastY = ev.y
                    parent?.requestDisallowInterceptTouchEvent(true)
                    return true
                }
            }
        }
        return false
    }

    override fun onTouchEvent(ev: MotionEvent): Boolean {
        if (anim != null) return true
        if (mode == Mode.NONE) return super.onTouchEvent(ev)
        val t = tracker ?: VelocityTracker.obtain().also { tracker = it }
        t.addMovement(ev)
        when (ev.actionMasked) {
            MotionEvent.ACTION_MOVE -> {
                val d = ev.y - lastY
                lastY = ev.y
                if (abs(ev.y - downY) > slop) moved = true
                set(progress + d * (if (mode == Mode.PULL) DAMPING else 1f) / travel)
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                t.computeCurrentVelocity(1000)
                val vy = t.yVelocity
                t.recycle(); tracker = null
                when {
                    mode == Mode.PEEK && !moved -> close()
                    mode == Mode.PULL -> if (progress * travel >= dp(OPEN_DP) || vy > 1200) open() else close()
                    else -> if (vy < -300 || progress < 0.85f) close() else open()
                }
                mode = Mode.NONE
            }
        }
        return true
    }

    private fun dp(v: Float) = v * resources.displayMetrics.density

    companion object {
        /** 松手时拉过这么多（已经算上阻尼）就展开 */
        const val OPEN_DP = 72f
        const val PEEK_DP = 64f
        /** 手指走 1，页面走 0.55：拉起来发沉，与微信一样 */
        const val DAMPING = 0.55f
    }
}
