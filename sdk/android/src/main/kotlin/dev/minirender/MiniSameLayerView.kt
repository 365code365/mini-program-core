package dev.minirender

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.os.Handler
import android.os.Looper
import org.json.JSONObject
import java.io.File
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
 * 手势：默认拖动从框里开始照样滚页面（与同层渲染的视频一致），只把**点击**交给 [onTap]。
 * [wantsGestures] 为真的（SDK 自带的地图 [MiniMapView]）按在框里的整串触摸都交给 [onTouch]：拖地图、捏合缩放，页面不滚。
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

    /** 框的设备像素尺寸（`sameLayerViews` 的 `box`；[onBox] 之前已更新） */
    var boxWidth = 0
        internal set
    var boxHeight = 0
        internal set

    internal interface Host {
        fun invalidate(v: MiniSameLayerView)
        fun emit(v: MiniSameLayerView, name: String, detail: JSONObject)
        fun resolve(path: String): File?
        fun frame(v: MiniSameLayerView, w: Int, h: Int, buf: ByteBuffer)
        fun contextEvent(payload: JSONObject)
        val context: Context
    }

    /**
     * 自己出帧（相机这类每帧都在变、在自己线程上读回的）：`w×h` RGBA 紧密排列，任何线程都可以调。
     * 不经 [onDraw] / 主线程；交完让引擎出一帧（只重画这个框）
     */
    fun pushFrame(w: Int, h: Int, buf: ByteBuffer) {
        host?.frame(this, w, h, buf)
    }

    /**
     * 上下文对象的事件（`{context, contextId, event, data}`，逻辑层 `__dispatchContextEvent`）：
     * 相机录到上限的 `recordTimeout` 这类
     */
    fun dispatchContextEvent(payload: JSONObject) {
        host?.contextEvent(payload)
    }

    /** 框的设备像素尺寸变了（含第一次挂上，在 [onAttrs] 之前）。默认不管 */
    open fun onBox(w: Int, h: Int) {}

    /** 宿主进后台 / 回前台（相机要关掉再开）。默认不管 */
    open fun onPause() {}
    open fun onResume() {}

    /** 宿主的 Context（挂上之后才有；取缓存目录、加载图片用） */
    protected val context: Context? get() = host?.context

    /**
     * 自己处理手势（地图平移 / 捏合缩放）：建引擎时这个标签进 `sameLayerGestures`，按在框里的整串触摸交给 [onTouch]，
     * 页面不滚、也不再发 [onTap]（点击自己从触摸里判）。默认 false：拖动照样滚页面、点击走 [onTap]
     */
    open val wantsGestures: Boolean get() = false

    /**
     * 按在框里的一串触摸（[wantsGestures] 为真时）：[phase] 是 `start` / `move` / `end` / `cancel`，
     * [pointer] 0 是第一根手指、1 是第二根（捏合），坐标是框里的逻辑像素，[t] 是引擎时钟毫秒
     */
    open fun onTouch(phase: String, pointer: Int, x: Float, y: Float, t: Long) {}

    /**
     * 组件的上下文对象调了方法（`wx.createMapContext(id).getCenterLocation()` → [method] = `getCenterLocation`）。
     * 认得就处理、回包并返回 true；不认得返回 false（SDK 回 fail）。要让小程序把这些调用交过来，
     * 宿主得在 [contextMethods] 里列出方法名
     */
    open fun onContextCall(method: String, args: JSONObject, reply: MiniHostReply): Boolean = false

    /** 上下文对象交给宿主的方法（`MapContext.<名>`）。建引擎时并进 `hostApis` */
    open val contextMethods: List<String> get() = emptyList()

    /** 上下文对象的类名（`MapContext`）；与 [contextMethods] 配对 */
    open val contextClass: String get() = ""

    /**
     * 小程序里的路径 → 本地文件：`/images/a.png`（包根）、`images/a.png`（当前页目录，找不到再按包根）、
     * `wxfile://tmp/…`（下载 / 选图的临时文件，按引擎报的 wxfile 根目录）。网络地址返回 null
     */
    fun resolvePath(path: String): File? = host?.resolve(path)

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
    override val context: Context,
    /** 在引擎线程上执行（EngineSession.run / Flutter 插件的 EngineCore.run） */
    private val onEngine: ((Long) -> Unit) -> Unit,
    private val density: () -> Float,
) : MiniSameLayerView.Host {
    /** 标签 → 工厂（宿主登记） */
    val factories = LinkedHashMap<String, () -> MiniSameLayerView>()

    /** 打开的小程序目录与当前页（`page` 事件），解析图标路径用 */
    var appDir: File? = null
    var route: String = ""
    /** `wxfile://` 的真实根目录（`sameLayerViews` 事件的 `wxfileRoot`） */
    var wxfileRoot: File? = null

    private val probes = HashMap<String, MiniSameLayerView>()

    /** 不挂到页面上的样板实例（问它的手势 / 上下文方法；工厂只调一次） */
    private fun probe(tag: String): MiniSameLayerView? = probes[tag] ?: factories[tag]?.invoke()?.also { probes[tag] = it }

    /** 建引擎时交给 `sameLayerGestures` 选项的标签 */
    fun gestureTags(): List<String> = factories.keys.filter { probe(it)?.wantsGestures == true }

    /** 建引擎时并进 `hostApis` 的上下文方法（`MapContext.getCenterLocation` …） */
    fun contextApis(): List<String> = factories.keys.flatMap { t ->
        val p = probe(t) ?: return@flatMap emptyList()
        p.contextMethods.map { "${p.contextClass}.$it" }
    }

    /**
     * `hostCall` 里的 `<类>.<方法>`：按 `contextId`（组件里的节点是 `id@实例键`，取 `@` 前面）找到实例交给它。
     * 不是上下文方法返回 false
     */
    fun contextCall(call: MiniHostCall, reply: MiniHostReply): Boolean {
        val dot = call.api.indexOf('.')
        if (dot <= 0) return false
        val cls = call.api.substring(0, dot)
        val method = call.api.substring(dot + 1)
        val id = call.args.optString("contextId").substringBefore('@')
        val v = slots.values.map { it.view }.firstOrNull { it.contextClass == cls && (it.key == id || id.isEmpty()) }
            ?: slots.values.map { it.view }.firstOrNull { it.contextClass == cls && slots.size == 1 }
        if (v == null) {
            if (probes.values.none { it.contextClass == cls }) return false
            reply.fail("$method:fail 页面上没有 id 为 \"$id\" 的组件")
            return true
        }
        if (!v.onContextCall(method, call.args, reply)) reply.fail("$method:fail 当前宿主不支持")
        return true
    }

    override fun resolve(path: String): File? {
        val p = path.trim()
        // 下载 / 选图得到的临时文件与用户文件（`wxfile://tmp/…`、`wxfile://usr/…`）：引擎报来的 wxfile 根目录下找，
        // 不许 `..` 跳出去（与引擎 wxfile::to_path 同一条）。引擎库太旧、没报根目录时找不到（图标不画）
        if (p.startsWith("wxfile://")) {
            val rel = p.removePrefix("wxfile://")
            val root = wxfileRoot ?: return null
            if (rel.isEmpty() || rel.split('/').any { it == ".." || it == "." }) return null
            return File(root, rel).takeIf { it.isFile }
        }
        val dir = appDir ?: return null
        if (p.isEmpty() || p.contains("://")) return null
        if (p.startsWith("/")) return File(dir, p.trimStart('/')).takeIf { it.isFile }
        val pageDir = route.substringBeforeLast('/', "")
        return File(dir, "$pageDir/$p").takeIf { pageDir.isNotEmpty() && it.isFile } ?: File(dir, p).takeIf { it.isFile }
    }

    /** 主线程：`nativeViewTouch` 事件 */
    fun onTouch(ev: JSONObject) {
        slots[ev.optString("key")]?.view?.onTouch(
            ev.optString("phase"), ev.optInt("pointer"),
            ev.optDouble("x").toFloat(), ev.optDouble("y").toFloat(), ev.optLong("t"),
        )
    }

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
        ev.optString("wxfileRoot").takeIf { it.isNotEmpty() }?.let { wxfileRoot = File(it) }
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
                v.boxWidth = w
                v.boxHeight = h
                v.onBox(w, h)
            }
            slot.view.width = o.optDouble("width", 0.0).toFloat()
            slot.view.height = o.optDouble("height", 0.0).toFloat()
            val attrs = o.optJSONObject("attrs") ?: JSONObject()
            // 地图的自定义气泡图（引擎离屏画好的 PNG，engine_map_callouts.rs）：并进属性，变了照样走 onAttrs
            o.optJSONArray("callouts")?.let { attrs.put("__callouts", it.toString()) }
            if (fresh || attrs.toString() != slot.view.attrs.toString()) {
                slot.view.attrs = attrs
                slot.view.onAttrs(attrs)
            }
            // 框变了：按新尺寸重画（宿主按 box 画，引擎整行拷贝）
            if (slot.w != w || slot.h != h) {
                slot.w = w
                slot.h = h
                slot.view.boxWidth = w
                slot.view.boxHeight = h
                slot.view.onBox(w, h)
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

    override fun frame(v: MiniSameLayerView, w: Int, h: Int, buf: ByteBuffer) {
        val key = v.key
        buf.rewind()
        if (MiniEngine.nativeViewFrameRgba(key, w, h, w * 4, buf)) onEngine { }
    }

    override fun contextEvent(payload: JSONObject) {
        val s = payload.toString()
        onEngine { MiniEngine.nativeDispatchEvent(it, "contextEvent", s) }
    }

    /** 宿主进后台 / 回前台：转给每个实例 */
    fun pause() = slots.values.forEach { it.view.onPause() }
    fun resume() = slots.values.forEach { it.view.onResume() }

    override fun emit(v: MiniSameLayerView, name: String, detail: JSONObject) {
        val key = v.key
        val d = detail.toString()
        onEngine { MiniEngine.nativeNativeViewEvent(it, key, name, d) }
    }

    private var statAt = 0L
    private val stat = ArrayList<Double>()
    private var worst = ""

    /**
     * 主线程上画一帧的耗时：连续在画（拖地图）时每秒一行「N 帧 中位 / 最慢」，最慢那帧拆成画 / 读出 / 交给引擎。
     * 地图拖动跟不跟手先看它（`adb logcat -s mini-render | grep 🗺`）
     */
    private fun slowLog(k: String, s: Slot, t0: Long, t1: Long, t2: Long, t3: Long) {
        val ms = (t3 - t0) / 1e6
        if (stat.isEmpty() || ms > stat.max()) {
            worst = "画 %.1f / 读出 %.1f / 交给引擎 %.1f".format((t1 - t0) / 1e6, (t2 - t1) / 1e6, (t3 - t2) / 1e6)
        }
        if (stat.isEmpty()) statAt = t0
        stat += ms
        if (t3 - statAt < 1_000_000_000L) return
        val sorted = stat.sorted()
        android.util.Log.i("mini-render", "🗺 同层组件 $k %d 帧：中位 %.1fms 最慢 %.1fms（%s，%d×%d）".format(
            stat.size, sorted[sorted.size / 2], sorted.last(), worst, s.w, s.h))
        stat.clear()
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
            val t0 = System.nanoTime()
            bmp.eraseColor(0)
            // 画布不按位图密度缩放（DENSITY_NONE）：解出来的位图密度是 160，画布是屏幕的 dpi，
            // 不关的话 drawBitmap(b, x, y) 会悄悄放大三倍再贴（又慢又不对）
            val cv = Canvas(bmp).apply { density = Bitmap.DENSITY_NONE }
            s.view.onDraw(cv, s.w, s.h, density())
            val t1 = System.nanoTime()
            val buf = s.buffer!!
            buf.rewind()
            // ARGB_8888 在内存里是 R G B A（预乘）：与引擎要的 RGBA 同一个排列；页面里的这块是不透明的
            bmp.copyPixelsToBuffer(buf)
            buf.rewind()
            val t2 = System.nanoTime()
            if (MiniEngine.nativeViewFrameRgba(k, s.w, s.h, s.w * 4, buf)) {
                // 引擎只重画这个框
                onEngine { }
            }
            slowLog(k, s, t0, t1, t2, System.nanoTime())
        }
    }
}
