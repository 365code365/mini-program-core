package dev.minirender.flutter

import android.content.Context
import android.view.ViewGroup
import dev.minirender.MiniEngine
import dev.minirender.MiniHostCall
import dev.minirender.MiniHostReply
import dev.minirender.MiniMapView
import dev.minirender.MiniWebViews
import dev.minirender.SameLayerViews
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.concurrent.atomic.AtomicBoolean

/**
 * 插件自带的原生组件（Android）：与原生 SDK 的 `MiniProgramView` 同一份实现（Kotlin 源码编进插件）——
 * - `<map>`：[MiniMapView]（瓦片地图，同层渲染，手势 / MapContext），Dart 的 `builtinMap`；
 * - `<web-view>`：系统 WebView 盖在引擎那块平台视图上（[MiniWebViews]，只在独立图层模式下有容器），Dart 的 `builtinWebView`。
 *
 * 引擎事件先过这里（主线程）：归这两样的（地图的 `sameLayerViews` / `nativeViewTap` / `nativeViewTouch`、
 * `MapContext.*` 的 hostCall、web-view 的 `nativeViews`）在这里处理并从交给 Dart 的列表里拿掉，
 * 免得 Dart 侧的 nativeViewBuilder 再给它们建一份控件。
 */
internal class NativeComponents(
    context: Context,
    private val onEngine: ((Long) -> Unit) -> Unit,
    private val map: Boolean,
    private val webView: Boolean,
    /** 盖 WebView 的容器（独立图层模式下 EngineView 的那层；纹理模式没有） */
    private val overlay: () -> ViewGroup?,
) {
    private val sameLayer = SameLayerViews(context, onEngine) { context.resources.displayMetrics.density }.also {
        if (map) it.factories["map"] = { MiniMapView() }
    }
    private var webViews: MiniWebViews? = null
    private var lastPage: JSONObject? = null
    /** 上一条交给 Dart 的 `nativeViews` 是空的（web-view 摘掉之后） */
    private var dartViewsEmpty = true

    /** 建引擎的选项里补上自带组件那几项（同层渲染、手势、MapContext 的 hostApis、盖上去的 web-view）；视频同层时直播也同层 */
    fun options(json: String): String = try {
        val o = JSONObject(json)
        fun add(k: String, vararg v: String) {
            val a = o.optJSONArray(k) ?: JSONArray().also { o.put(k, it) }
            val have = (0 until a.length()).map { a.optString(it) }.toSet()
            v.filter { it !in have }.forEach { a.put(it) }
        }
        val same = o.optJSONArray("sameLayer")
        if (same != null && (0 until same.length()).any { same.optString(it) == "video" }) add("sameLayer", "live-player")
        if (map) {
            add("sameLayer", "map")
            add("sameLayerGestures", "map")
            add("hostApis", *MAP_APIS.toTypedArray())
        }
        if (webView) add("nativeComponents", "web-view")
        o.toString()
    } catch (e: org.json.JSONException) {
        json
    }

    fun setAppDir(dir: String) {
        sameLayer.appDir = File(dir)
    }

    private fun web(): MiniWebViews? {
        if (!webView) return null
        webViews?.let { return it }
        val p = overlay() ?: return null
        return MiniWebViews(p, onEngine).also { w ->
            webViews = w
            lastPage?.let(w::onPage)
        }
    }

    /** 主线程：处理归这里的事件，返回剩下交给 Dart 的 */
    fun filter(list: List<String>): List<String> {
        if (!map && !webView) return list
        val out = ArrayList<String>(list.size)
        for (raw in list) {
            val type = typeOf(raw)
            when (type) {
                "page" -> {
                    val ev = JSONObject(raw)
                    sameLayer.route = ev.optString("route")
                    lastPage = ev
                    webViews?.onPage(ev)
                    out += raw
                }
                "sameLayerViews" -> {
                    val ev = JSONObject(raw)
                    sameLayer.onViews(ev)
                    out += strip(ev, "views") { it.optString("tag") in sameLayer.factories }
                }
                "nativeViewTap", "nativeViewTouch" -> {
                    val ev = JSONObject(raw)
                    if (ev.optString("tag") in sameLayer.factories) {
                        if (type == "nativeViewTap") sameLayer.onTap(ev) else sameLayer.onTouch(ev)
                    } else {
                        out += raw
                    }
                }
                "nativeViews" -> {
                    val ev = JSONObject(raw)
                    // 页面上真有 web-view 才建容器（盖在引擎画面上的那层 FrameLayout 第一次用时才挂）
                    (webViews ?: if (raw.contains("\"tag\":\"web-view\"")) web() else null)?.onViews(ev)
                    if (!webView) {
                        out += raw
                    } else {
                        // 摘掉 web-view 之后是空的、上一条交给 Dart 的也是空的：不交（Dart 侧没有自己盖的控件时一条都不收）。
                        // 实测（一加 PLY110，host-slow）：Dart 收到第一条 nativeViews（哪怕 views 是空的）之后，引擎每帧 GPU 提交
                        // 从 1ms 涨到 12~13ms，>8ms 的帧 0 → 150~190；不交就回到 2。Dart 那边为什么会这样还没查清
                        val kept = strip(ev, "views") { it.optString("tag") == "web-view" }
                        val empty = (ev.optJSONArray("views")?.length() ?: 0) == 0
                        if (!(empty && dartViewsEmpty)) out += kept
                        dartViewsEmpty = empty
                    }
                }
                "hostCall" -> {
                    val ev = JSONObject(raw)
                    if (ev.optString("api").startsWith("MapContext.") && map) contextCall(ev) else out += raw
                }
                else -> out += raw
            }
        }
        return out
    }

    private fun typeOf(raw: String): String {
        val i = raw.indexOf("\"type\":\"")
        if (i < 0) return ""
        val s = i + 8
        return raw.substring(s, raw.indexOf('"', s).coerceAtLeast(s))
    }

    private fun strip(ev: JSONObject, k: String, drop: (JSONObject) -> Boolean): String {
        val a = ev.optJSONArray(k) ?: return ev.toString()
        val kept = JSONArray()
        for (i in 0 until a.length()) a.optJSONObject(i)?.takeUnless(drop)?.let(kept::put)
        return ev.put(k, kept).toString()
    }

    private fun contextCall(ev: JSONObject) {
        val f = ev.optJSONObject("files")
        val call = MiniHostCall(
            ev.optInt("id"), ev.optString("api"), ev.optJSONObject("args") ?: JSONObject(), ev.optString("tempDir"),
            f?.keys()?.asSequence()?.associateWith { f.optString(it) }.orEmpty(),
        )
        val done = AtomicBoolean(false)
        val reply = object : MiniHostReply {
            override fun success(result: JSONObject) = send(true, result)
            override fun fail(errMsg: String, extra: JSONObject?) = send(false, JSONObject(extra?.toString() ?: "{}").put("errMsg", errMsg))
            private fun send(ok: Boolean, res: JSONObject) {
                if (!done.compareAndSet(false, true)) return
                val s = res.toString()
                onEngine { MiniEngine.nativeHostReply(it, call.id, ok, s) }
            }
        }
        if (!sameLayer.contextCall(call, reply)) reply.fail("${call.api}:fail 宿主没有实现")
    }

    /** 返回键：web-view 里的网页先后退 */
    fun goBack(): Boolean = webViews?.goBack() ?: false

    fun lifecycle(what: Int) {
        when (what) {
            0 -> webViews?.resume()
            1 -> webViews?.pause()
        }
    }

    fun dispose() {
        webViews?.destroy()
        webViews = null
    }

    companion object {
        /** 建引擎时并进 `hostApis` 的 MapContext 方法（与 [MiniMapView] 认的那一组相同） */
        val MAP_APIS: List<String> get() = MiniMapView().contextMethods.map { "MapContext.$it" }
    }
}
