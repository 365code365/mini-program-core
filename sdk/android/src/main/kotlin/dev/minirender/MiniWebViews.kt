package dev.minirender

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.Rect
import android.graphics.drawable.ColorDrawable
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.webkit.JavascriptInterface
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout
import org.json.JSONObject

/**
 * `<web-view>`：系统 WebView **盖在**引擎画面上（创建选项 `nativeComponents` 含 `web-view`，位置跟 `nativeViews` 事件）。
 *
 * 为什么它不走同层渲染：网页要自己滚、自己接输入、自己放视频，离屏一帧帧画进页面又慢又丢交互；而官方的
 * web-view 本来就铺满整页、盖住页面上的其他组件，页面自己不滚 —— 盖上去不会有「滑动时跟页面错开」的问题。
 *
 * - 网页里的 JSSDK：页面开始 / 加载完各注入一次 [SHIM]（幂等）：`wx.miniProgram.*`、`WeixinJSBridge.invoke('invokeMiniProgramAPI')`、
 *   `window.__wxjs_environment = 'miniprogram'`，注入后派 `WeixinJSBridgeReady`。调用经 `__miniBridgeNative` 回到这里，
 *   再按引擎的约定转成组件事件（`message` / `miniProgram` / `load` / `error` / `titlechange`，见引擎 host/engine_webview.rs）。
 * - 页面栈：按 (层数, 路由, 键) 认一个 WebView。上面压了新页时藏起来（不销毁，回来时网页状态还在），
 *   页面出栈 / 被替换时销毁。
 * - 返回键：网页里能后退先后退（与微信一致），退到头才交给小程序。
 * - 遮挡：`clip` 为空就藏起来；`mask` > 0（Modal 这类蒙层）时盖一层半透明黑、不接触摸。
 *
 * iOS 那份是 `MiniWebViews.swift`（WKWebView），改一边对照另一边。
 */
internal class MiniWebViews(
    private val parent: ViewGroup,
    /** 在引擎线程上执行 */
    private val onEngine: ((Long) -> Unit) -> Unit,
) {
    private val context: Context get() = parent.context
    private val main = Handler(Looper.getMainLooper())

    private inner class Slot(val id: String, val key: String, val depth: Int, val route: String) {
        val frame = Shield(context)
        val web = WebView(context)
        var url = ""
        var src = ""

        init {
            setup(this)
            frame.addView(web, FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
            parent.addView(frame, FrameLayout.LayoutParams(0, 0))
        }

        fun emit(name: String, detail: JSONObject) {
            val k = key
            val d = detail.toString()
            onEngine { MiniEngine.nativeNativeViewEvent(it, k, name, d) }
        }

        fun destroy() {
            parent.removeView(frame)
            web.stopLoading()
            web.destroy()
        }
    }

    /** 盖在 WebView 上的那层：蒙层弹着时吃掉触摸、盖一层黑 */
    private class Shield(c: Context) : FrameLayout(c) {
        var masked = false
        override fun onInterceptTouchEvent(ev: MotionEvent) = masked
        @SuppressLint("ClickableViewAccessibility")
        override fun onTouchEvent(ev: MotionEvent) = masked
    }

    private val slots = LinkedHashMap<String, Slot>()
    private var depth = 0
    private var route = ""

    /** 主线程：`page` 事件。出栈 / 被替换的页上的 WebView 销毁 */
    fun onPage(ev: JSONObject) {
        depth = ev.optInt("depth", depth)
        route = ev.optString("route", route)
        val dead = slots.values.filter { it.depth > depth || (it.depth == depth && it.route != route) }
        dead.forEach { it.destroy(); slots.remove(it.id) }
    }

    /** 主线程：`nativeViews` 事件里的 web-view（别的标签不归这里） */
    fun onViews(ev: JSONObject) {
        val list = ev.optJSONArray("views") ?: return
        val density = context.resources.displayMetrics.density
        val seen = HashSet<String>()
        for (i in 0 until list.length()) {
            val v = list.optJSONObject(i) ?: continue
            if (v.optString("tag") != "web-view") continue
            val key = v.optString("key")
            val id = "$depth|$route|$key"
            seen += id
            val s = slots.getOrPut(id) { Slot(id, key, depth, route) }
            place(s, v, density)
            val src = v.optJSONObject("attrs")?.optString("src").orEmpty()
            val url = v.optString("url").ifEmpty { src }
            if (url != s.url) {
                s.url = url
                s.src = src
                load(s, url)
            }
        }
        // 这一页没报的（压在下面的页、wx:if 关掉的）：藏起来，状态留着
        for (s in slots.values) if (s.id !in seen) s.frame.visibility = View.GONE
    }

    private fun place(s: Slot, v: JSONObject, d: Float) {
        val x = (v.optDouble("x") * d).toInt()
        val y = (v.optDouble("y") * d).toInt()
        val w = (v.optDouble("width") * d).toInt()
        val h = (v.optDouble("height") * d).toInt()
        val lp = s.frame.layoutParams as FrameLayout.LayoutParams
        if (lp.leftMargin != x || lp.topMargin != y || lp.width != w || lp.height != h) {
            lp.leftMargin = x; lp.topMargin = y; lp.width = w; lp.height = h
            s.frame.layoutParams = lp
        }
        val c = v.optJSONObject("clip")
        val cw = (c?.optDouble("width") ?: 0.0) * d
        val ch = (c?.optDouble("height") ?: 0.0) * d
        if (cw < 1 || ch < 1) {
            s.frame.visibility = View.INVISIBLE
            return
        }
        s.frame.visibility = View.VISIBLE
        val cx = ((c!!.optDouble("x") * d) - x).toInt()
        val cy = ((c.optDouble("y") * d) - y).toInt()
        s.frame.clipBounds = Rect(cx, cy, cx + cw.toInt(), cy + ch.toInt()).takeUnless { it.left <= 0 && it.top <= 0 && it.right >= w && it.bottom >= h }
        val mask = v.optDouble("mask", 0.0)
        s.frame.masked = mask > 0
        s.frame.foreground = if (mask > 0) ColorDrawable(Color.argb((mask * 255).toInt().coerceIn(0, 255), 0, 0, 0)) else null
    }

    /** 包内的离线页是本地绝对路径（引擎解析好的 `url`），网络地址原样 */
    private fun load(s: Slot, url: String) {
        val u = when {
            url.startsWith("/") -> Uri.fromFile(java.io.File(url.substringBefore('?').substringBefore('#'))).toString() + url.substring(url.indexOfAny(charArrayOf('?', '#')).let { if (it < 0) url.length else it })
            else -> url
        }
        if (u.isBlank()) return
        s.web.loadUrl(u)
    }

    @SuppressLint("SetJavaScriptEnabled")
    private fun setup(s: Slot) {
        val w = s.web
        w.setBackgroundColor(Color.WHITE)
        w.settings.apply {
            javaScriptEnabled = true
            domStorageEnabled = true
            mediaPlaybackRequiresUserGesture = false
            // 包内的离线 H5 是 file://（引擎解析的本地路径）
            allowFileAccess = true
            mixedContentMode = WebSettings.MIXED_CONTENT_COMPATIBILITY_MODE
            // 微信的 web-view 里 UA 带 miniProgram，网页常靠它判环境
            userAgentString = "$userAgentString miniProgram MiniRender"
        }
        w.addJavascriptInterface(Bridge(s), "__miniBridgeNative")
        w.webViewClient = object : WebViewClient() {
            override fun onPageStarted(view: WebView, url: String, favicon: Bitmap?) {
                view.evaluateJavascript(SHIM, null)
            }

            override fun onPageFinished(view: WebView, url: String) {
                view.evaluateJavascript(SHIM, null)
                s.emit("load", JSONObject().put("src", s.src.ifEmpty { url }))
            }

            override fun onReceivedError(view: WebView, req: WebResourceRequest, err: WebResourceError) {
                if (!req.isForMainFrame) return
                s.emit("error", JSONObject().put("src", s.src.ifEmpty { req.url.toString() }).put("errMsg", "${err.description}").put("errCode", err.errorCode))
            }

            override fun shouldOverrideUrlLoading(view: WebView, req: WebResourceRequest): Boolean {
                val scheme = req.url.scheme ?: return false
                // 网页里点了 tel: / weixin:// 这类：WebView 打不开，交给系统（没有能处理的应用就算了）
                if (scheme == "http" || scheme == "https" || scheme == "file" || scheme == "about" || scheme == "data") return false
                runCatching { context.startActivity(android.content.Intent(android.content.Intent.ACTION_VIEW, req.url).addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK)) }
                return true
            }
        }
        w.webChromeClient = object : WebChromeClient() {
            override fun onReceivedTitle(view: WebView, title: String?) {
                val t = title?.trim().orEmpty()
                // 还没读到 <title> 时 WebView 报的是地址本身：不当标题
                if (t.isNotEmpty() && !t.startsWith("http") && !t.startsWith("file:") && !t.startsWith("/")) s.emit("titlechange", JSONObject().put("title", t))
            }
        }
    }

    /** 网页里的 JSSDK 调到这里（JavaBridge 线程）→ 主线程处理 */
    private inner class Bridge(private val s: Slot) {
        @JavascriptInterface
        fun postMessage(raw: String) {
            main.post { handle(s, raw) }
        }
    }

    private fun handle(s: Slot, raw: String) {
        val o = runCatching { JSONObject(raw) }.getOrNull() ?: return
        val id = o.optInt("id")
        val api = o.optString("api")
        val arg = o.optJSONObject("arg") ?: JSONObject()
        val res = JSONObject().put("errMsg", "$api:ok")
        when (api) {
            "postMessage" -> s.emit("message", JSONObject().put("data", arg.opt("data") ?: JSONObject.NULL))
            "navigateTo", "navigateBack", "redirectTo", "switchTab", "reLaunch" -> s.emit("miniProgram", JSONObject().put("api", api).put("arg", arg))
            else -> {
                Log.w(TAG, "web-view：网页调了不支持的 wx.miniProgram.$api")
                res.put("errMsg", "$api:fail 不支持")
            }
        }
        if (id > 0) s.web.evaluateJavascript("window.__miniBridgeReply && __miniBridgeReply($id, $res)", null)
    }

    /** 返回键：当前页的网页能后退就后退，返回 true */
    fun goBack(): Boolean {
        val s = slots.values.lastOrNull { it.depth == depth && it.route == route && it.frame.visibility == View.VISIBLE } ?: return false
        if (!s.web.canGoBack()) return false
        s.web.goBack()
        return true
    }

    fun pause() = slots.values.forEach { it.web.onPause() }
    fun resume() = slots.values.forEach { it.web.onResume() }

    fun destroy() {
        slots.values.forEach { it.destroy() }
        slots.clear()
    }

    companion object {
        private const val TAG = "mini-render"

        /** 网页里的 JSSDK（与 iOS `MiniWebViews.swift` 里那份同一段，只有第一行发消息的方式不同） */
        val SHIM = """
(function(){
  if (window.__miniShim) return; window.__miniShim = true;
  var send = function(s){ window.__miniBridgeNative.postMessage(s); };
  window.__wxjs_environment = 'miniprogram';
  var cbs = {}, seq = 0;
  function call(api, arg, cb){
    if (api === 'getEnv') { if (cb) cb({ miniprogram: true }); return; }
    var a = {}; for (var k in (arg || {})) if (typeof arg[k] !== 'function') a[k] = arg[k];
    var id = ++seq; if (cb) cbs[id] = cb;
    send(JSON.stringify({ id: id, api: api, arg: a }));
  }
  window.__miniBridgeReply = function(id, res){ var c = cbs[id]; delete cbs[id]; if (c) c(res); };
  function api(name){ return function(o){ o = o || {}; call(name, o, function(res){
    var ok = /:ok$/.test(res && res.errMsg || '');
    if (ok && o.success) o.success(res); if (!ok && o.fail) o.fail(res); if (o.complete) o.complete(res); }); }; }
  if (!window.WeixinJSBridge) window.WeixinJSBridge = {
    invoke: function(name, arg, cb){
      if (name === 'invokeMiniProgramAPI') return call(arg && arg.name, (arg && arg.arg) || {}, function(res){ if (cb) cb({ err_msg: (res && res.errMsg) || '' }); });
      if (cb) cb({ err_msg: name + ':fail' });
    },
    on: function(){}, call: function(){}
  };
  window.wx = window.wx || {};
  if (!window.wx.miniProgram) window.wx.miniProgram = {
    navigateTo: api('navigateTo'), navigateBack: api('navigateBack'), redirectTo: api('redirectTo'),
    switchTab: api('switchTab'), reLaunch: api('reLaunch'),
    postMessage: function(o){ call('postMessage', { data: o && o.data }); },
    getEnv: function(cb){ if (cb) cb({ miniprogram: true }); }
  };
  try { document.dispatchEvent(new Event('WeixinJSBridgeReady')); } catch (e) {}
})();
""".trimIndent()
    }
}
