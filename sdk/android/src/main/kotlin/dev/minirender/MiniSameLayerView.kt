package dev.minirender

import android.graphics.Bitmap
import android.graphics.Canvas
import android.os.Handler
import android.os.Looper
import org.json.JSONObject
import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * 同层渲染的原生组件（`<map>` 这类）：宿主**画**它，引擎把画好的那一帧画进页面（[MiniProgramView.sameLayerViews]）。
 *
 * ## 为什么不把原生控件盖在画面上
 * 盖上去的控件与引擎画面走两条上屏路：引擎画到自己的 SurfaceView、系统合成器直接取；控件要等事件
 * （每帧的位置）→ 主线程 → 下一帧，晚 2~4 个 vsync，而且每帧晚几个不一样 —— 一滑页面地图框就跟页面错开、
 * 一顿一顿。画进页面以后它就是页面画布里的一块像素：滚动时与文字在同一张图里挪，宿主一帧都不用重画。
 * 这正是微信「同层渲染」的结果（见 doc/滑动顿挫避坑清单.md）。
 *
 * ## 怎么写
 * ```kotlin
 * class MyMap : MiniSameLayerView() {
 *     override fun onAttrs(attrs: JSONObject) { lat = attrs.optDouble("latitude"); invalidate() }
 *     override fun onDraw(canvas: Canvas, width: Int, height: Int, density: Float) { /* 画地图 */ }
 *     override fun onTap(x: Float, y: Float) { if (hitMarker(x, y)) emit("markertap", JSONObject().put("markerId", 1)) }
 * }
 * view.sameLayerViews["map"] = { MyMap() }
 * ```
 * 都在主线程上调。内容变了调 [invalidate]（下一帧重画、交给引擎，只重画页面上的这一块）；
 * 不变就不用管 —— 滚动、页面别处重画都不需要它重画。
 *
 * 取舍：拖动从框里开始照样滚页面（与同层渲染的视频一致），只把**点击**交给它；地图自己的平移 / 缩放手势没有转。
 * 真地图 SDK（高德 / Google）要把地图画进这块 Canvas，用它们的截图 / TextureView 接口；
 * 做不到就照旧用盖控件那条路（`nativeComponents`），代价是滑动时跟不住页面。
 */
abstract class MiniSameLayerView {
    /** 节点的键（`id`，没有就是标签名） */
    lateinit var key: String
        internal set

    /** 框的逻辑尺寸（[onTap] 的坐标系；[onDraw] 拿到的是设备像素） */
    var width = 0f
        internal set
    var height = 0f
        internal set

    /** 节点属性（模板求值之后），[onAttrs] 之前已更新 */
    var attrs: JSONObject = JSONObject()
        internal set

    internal var host: Host? = null

    internal interface Host {
        fun invalidate(v: MiniSameLayerView)
        fun emit(v: MiniSameLayerView, name: String, detail: JSONObject)
    }

    /** 属性变了（含第一次挂上）。默认重画 */
    open fun onAttrs(attrs: JSONObject) = invalidate()

    /** 画一帧：[width] × [height] 是框的设备像素（一个像素对屏幕一个像素），[density] = dpr */
    abstract fun onDraw(canvas: Canvas, width: Int, height: Int, density: Float)

    /** 点在框里（视图本地的**逻辑**像素） */
    open fun onTap(x: Float, y: Float) {}

    /** 从页面上拿掉了（换页 / wx:if 关掉） */
    open fun onDetach() {}

    /** 内容变了：下一帧重画、交给引擎 */
    fun invalidate() {
        host?.invalidate(this)
    }

    /** 回传组件事件：派发给节点上的 `bind<name>`（如 `bindmarkertap`） */
    fun emit(name: String, detail: JSONObject = JSONObject()) {
        host?.emit(this, name, detail)
    }
}

/**
 * [MiniProgramView] 的同层组件那一片：按 `sameLayerViews` 事件建 / 改 / 放实例，主线程上画进 Bitmap，
 * 像素交给引擎（`nativeViewFrameRgba`，任何线程都行），再让引擎线程出一帧；`nativeViewTap` 转给实例。
 */
internal class SameLayerViews(
    private val session: EngineSession,
    private val density: () -> Float,
) : MiniSameLayerView.Host {
    /** 标签 → 工厂（宿主登记） */
    val factories = LinkedHashMap<String, () -> MiniSameLayerView>()

    private class Slot(val view: MiniSameLayerView, var w: Int, var h: Int) {
        var bitmap: Bitmap? = null
        var buffer: ByteBuffer? = null
    }

    private val slots = HashMap<String, Slot>()
    private val dirty = LinkedHashSet<String>()
    private val main = Handler(Looper.getMainLooper())
    private var scheduled = false

    /** 建引擎时交给 `sameLayer` 选项的标签 */
    fun tags(): List<String> = factories.keys.toList()

    /** 主线程：`sameLayerViews` 事件 */
    fun onViews(ev: JSONObject) {
        val list = ev.optJSONArray("views") ?: return
        val seen = HashSet<String>()
        for (i in 0 until list.length()) {
            val o = list.optJSONObject(i) ?: continue
            val key = o.optString("key")
            val box = o.optJSONArray("box")
            val w = box?.optInt(0) ?: 0
            val h = box?.optInt(1) ?: 0
            if (key.isEmpty() || w <= 0 || h <= 0) continue
            seen += key
            var slot = slots[key]
            val fresh = slot == null
            if (slot == null) {
                val v = factories[o.optString("tag")]?.invoke() ?: continue
                v.key = key
                v.host = this
                slot = Slot(v, w, h)
                slots[key] = slot
            }
            slot.view.width = o.optDouble("width", 0.0).toFloat()
            slot.view.height = o.optDouble("height", 0.0).toFloat()
            val attrs = o.optJSONObject("attrs") ?: JSONObject()
            if (fresh || attrs.toString() != slot.view.attrs.toString()) {
                slot.view.attrs = attrs
                slot.view.onAttrs(attrs)
            }
            // 框变了：按新尺寸重画（宿主按 box 画，引擎整行拷贝）
            if (slot.w != w || slot.h != h) {
                slot.w = w
                slot.h = h
                slot.view.invalidate()
            }
        }
        for (k in slots.keys - seen) {
            slots.remove(k)?.let { s ->
                s.view.host = null
                s.view.onDetach()
                s.bitmap?.recycle()
            }
            dirty -= k
        }
    }

    /** 主线程：`nativeViewTap` 事件 */
    fun onTap(ev: JSONObject) {
        slots[ev.optString("key")]?.view?.onTap(ev.optDouble("x").toFloat(), ev.optDouble("y").toFloat())
    }

    override fun invalidate(v: MiniSameLayerView) {
        if (slots[v.key]?.view !== v) return
        dirty += v.key
        if (scheduled) return
        scheduled = true
        // 同一帧里多次 invalidate 只画一次
        main.post(::drawDirty)
    }

    override fun emit(v: MiniSameLayerView, name: String, detail: JSONObject) {
        val key = v.key
        session.run { MiniEngine.nativeNativeViewEvent(it, key, name, detail.toString()) }
    }

    private fun drawDirty() {
        scheduled = false
        val keys = dirty.toList()
        dirty.clear()
        for (k in keys) {
            val s = slots[k] ?: continue
            var bmp = s.bitmap
            if (bmp == null || bmp.width != s.w || bmp.height != s.h) {
                bmp?.recycle()
                bmp = Bitmap.createBitmap(s.w, s.h, Bitmap.Config.ARGB_8888)
                s.bitmap = bmp
                s.buffer = ByteBuffer.allocateDirect(s.w * s.h * 4).order(ByteOrder.nativeOrder())
            }
            bmp.eraseColor(0)
            s.view.onDraw(Canvas(bmp), s.w, s.h, density())
            val buf = s.buffer!!
            buf.rewind()
            // ARGB_8888 在内存里是 R G B A（预乘）：与引擎要的 RGBA 同一个排列；页面里的这块是不透明的
            bmp.copyPixelsToBuffer(buf)
            buf.rewind()
            if (MiniEngine.nativeViewFrameRgba(k, s.w, s.h, s.w * 4, buf)) {
                // 引擎只重画这个框
                session.run { }
            }
        }
    }
}
