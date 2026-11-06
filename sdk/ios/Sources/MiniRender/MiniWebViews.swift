import Foundation
import UIKit
import WebKit
import MiniRenderFFI

/// `<web-view>`：WKWebView **盖在**引擎画面上（创建选项 `nativeComponents` 含 `web-view`，位置跟 `nativeViews` 事件）。
/// 与 Android `MiniWebViews.kt` 同一套约定（改一边对照另一边）：
/// - JSSDK：文档开始时注入 [shim]（`wx.miniProgram.*`、`WeixinJSBridge.invoke('invokeMiniProgramAPI')`、`__wxjs_environment`），
///   调用经 `miniBridge` 消息回到这里，再转成组件事件（`message` / `miniProgram` / `load` / `error` / `titlechange`）；
/// - 页面栈：按 (层数, 路由, 键) 认一个 WebView，被压住时藏起来、出栈 / 被替换时销毁；
/// - 返回：网页能后退先后退；网页里也能左边缘侧滑后退（WKWebView 自己的手势）；
/// - 遮挡：`clip` 为空藏起来，`mask` > 0 盖一层黑、不接触摸。
final class MiniWebViews: NSObject, WKScriptMessageHandler, WKNavigationDelegate {
    private weak var parent: UIView?
    private let run: (@escaping (OpaquePointer) -> Void) -> Void
    var appDir: URL?

    private final class Slot {
        let id: String, key: String, depth: Int, route: String
        let frame = UIView()
        let dim = UIView()
        let web: WKWebView
        var url = ""
        var src = ""
        var titleObs: NSKeyValueObservation?
        init(id: String, key: String, depth: Int, route: String, web: WKWebView) {
            self.id = id; self.key = key; self.depth = depth; self.route = route; self.web = web
        }
    }
    private var slots: [String: Slot] = [:]
    private var order: [String] = []
    private var depth = 0
    private var route = ""

    init(parent: UIView, run: @escaping (@escaping (OpaquePointer) -> Void) -> Void) {
        self.parent = parent
        self.run = run
    }

    private func emit(_ s: Slot, _ name: String, _ detail: [String: Any]) {
        let key = s.key
        let json = (try? JSONSerialization.data(withJSONObject: detail)).flatMap { String(data: $0, encoding: .utf8) } ?? "{}"
        run { _ = mr_app_native_view_event($0, key, name, json) }
    }

    /// 主线程：`page` 事件
    func onPage(_ ev: [String: Any]) {
        depth = (ev["depth"] as? NSNumber)?.intValue ?? depth
        route = ev["route"] as? String ?? route
        for s in slots.values where s.depth > depth || (s.depth == depth && s.route != route) { destroy(s) }
    }

    /// 主线程：`nativeViews` 事件里的 web-view
    func onViews(_ ev: [String: Any]) {
        var seen = Set<String>()
        for v in ev["views"] as? [[String: Any]] ?? [] where v["tag"] as? String == "web-view" {
            let key = v["key"] as? String ?? ""
            let id = "\(depth)|\(route)|\(key)"
            seen.insert(id)
            let s = slots[id] ?? make(id: id, key: key)
            place(s, v)
            let src = (v["attrs"] as? [String: Any])?["src"] as? String ?? ""
            let url = (v["url"] as? String).flatMap { $0.isEmpty ? nil : $0 } ?? src
            if url != s.url {
                s.url = url
                s.src = src
                load(s, url)
            }
        }
        for s in slots.values where !seen.contains(s.id) { s.frame.isHidden = true }
    }

    private func make(id: String, key: String) -> Slot {
        let cfg = WKWebViewConfiguration()
        cfg.allowsInlineMediaPlayback = true
        cfg.mediaTypesRequiringUserActionForPlayback = []
        // 微信的 web-view 里 UA 带 miniProgram，网页常靠它判环境
        cfg.applicationNameForUserAgent = "miniProgram MiniRender"
        let uc = WKUserContentController()
        uc.addUserScript(WKUserScript(source: Self.shim, injectionTime: .atDocumentStart, forMainFrameOnly: true))
        uc.add(WeakHandler(self), name: "miniBridge")
        cfg.userContentController = uc
        let web = WKWebView(frame: .zero, configuration: cfg)
        web.allowsBackForwardNavigationGestures = true
        web.navigationDelegate = self
        let s = Slot(id: id, key: key, depth: depth, route: route, web: web)
        s.frame.clipsToBounds = true
        web.frame = s.frame.bounds
        web.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        s.frame.addSubview(web)
        s.dim.backgroundColor = .black
        s.dim.isHidden = true
        s.dim.frame = s.frame.bounds
        s.dim.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        s.frame.addSubview(s.dim)
        s.titleObs = web.observe(\.title, options: [.new]) { [weak self, weak s] w, _ in
            guard let self, let s, let t = w.title?.trimmingCharacters(in: .whitespaces), !t.isEmpty,
                  !t.hasPrefix("http"), !t.hasPrefix("file:"), !t.hasPrefix("/") else { return }
            self.emit(s, "titlechange", ["title": t])
        }
        parent?.addSubview(s.frame)
        slots[id] = s
        order.append(id)
        return s
    }

    private func place(_ s: Slot, _ v: [String: Any]) {
        func f(_ o: Any?, _ k: String) -> CGFloat { CGFloat(((o as? [String: Any])?[k] as? NSNumber)?.doubleValue ?? 0) }
        let r = CGRect(x: f(v, "x"), y: f(v, "y"), width: f(v, "width"), height: f(v, "height"))
        if s.frame.frame != r { s.frame.frame = r }
        let c = v["clip"]
        let clip = CGRect(x: f(c, "x") - r.minX, y: f(c, "y") - r.minY, width: f(c, "width"), height: f(c, "height"))
        if clip.width < 1 || clip.height < 1 {
            s.frame.isHidden = true
            return
        }
        s.frame.isHidden = false
        if clip.contains(s.frame.bounds) {
            s.frame.layer.mask = nil
        } else {
            let m = (s.frame.layer.mask as? CAShapeLayer) ?? CAShapeLayer()
            m.path = UIBezierPath(rect: clip).cgPath
            s.frame.layer.mask = m
        }
        let mask = CGFloat((v["mask"] as? NSNumber)?.doubleValue ?? 0)
        s.dim.isHidden = mask <= 0
        s.dim.alpha = mask
        s.web.isUserInteractionEnabled = mask <= 0
    }

    private func load(_ s: Slot, _ url: String) {
        if url.hasPrefix("/") {
            // 包内的离线页：引擎给的是本地绝对路径，`?query` / `#hash` 接在后面
            let cut = url.firstIndex(where: { $0 == "?" || $0 == "#" }) ?? url.endIndex
            let file = URL(fileURLWithPath: String(url[..<cut]))
            var comps = URLComponents(url: file, resolvingAgainstBaseURL: false)
            let rest = String(url[cut...])
            if rest.hasPrefix("?") {
                let q = rest.dropFirst().split(separator: "#", maxSplits: 1)
                comps?.percentEncodedQuery = q.first.map(String.init)
                if q.count > 1 { comps?.percentEncodedFragment = String(q[1]) }
            } else if rest.hasPrefix("#") {
                comps?.percentEncodedFragment = String(rest.dropFirst())
            }
            s.web.loadFileURL(comps?.url ?? file, allowingReadAccessTo: appDir ?? file.deletingLastPathComponent())
        } else if let u = URL(string: url) {
            s.web.load(URLRequest(url: u))
        }
    }

    private func slot(of web: WKWebView) -> Slot? { slots.values.first { $0.web === web } }

    // ── JSSDK ──
    func userContentController(_ uc: WKUserContentController, didReceive message: WKScriptMessage) {
        guard let web = message.webView, let s = slot(of: web),
              let raw = message.body as? String, let d = raw.data(using: .utf8),
              let o = try? JSONSerialization.jsonObject(with: d) as? [String: Any] else { return }
        let id = (o["id"] as? NSNumber)?.intValue ?? 0
        let api = o["api"] as? String ?? ""
        let arg = o["arg"] as? [String: Any] ?? [:]
        var res = "\(api):ok"
        switch api {
        case "postMessage": emit(s, "message", ["data": arg["data"] ?? NSNull()])
        case "navigateTo", "navigateBack", "redirectTo", "switchTab", "reLaunch": emit(s, "miniProgram", ["api": api, "arg": arg])
        default:
            NSLog("mini-render web-view：网页调了不支持的 wx.miniProgram.\(api)")
            res = "\(api):fail 不支持"
        }
        if id > 0 { web.evaluateJavaScript("window.__miniBridgeReply && __miniBridgeReply(\(id), {errMsg: \"\(res)\"})") }
    }

    func webView(_ web: WKWebView, didFinish navigation: WKNavigation!) {
        guard let s = slot(of: web) else { return }
        emit(s, "load", ["src": s.src.isEmpty ? (web.url?.absoluteString ?? "") : s.src])
    }

    func webView(_ web: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) { fail(web, error) }
    func webView(_ web: WKWebView, didFail navigation: WKNavigation!, withError error: Error) { fail(web, error) }

    private func fail(_ web: WKWebView, _ error: Error) {
        guard let s = slot(of: web) else { return }
        let e = error as NSError
        if e.code == NSURLErrorCancelled { return }
        // 官方 web-view binderror：detail = {url, fullUrl}（src 属性上的地址 / 真正加载失败的完整地址）；src、errMsg、errCode 照留
        let full = (e.userInfo[NSURLErrorFailingURLStringErrorKey] as? String) ?? web.url?.absoluteString ?? s.src
        emit(s, "error", ["url": s.src.isEmpty ? full : s.src, "fullUrl": full, "src": s.src, "errMsg": e.localizedDescription, "errCode": e.code])
    }

    func webView(_ web: WKWebView, decidePolicyFor action: WKNavigationAction, decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
        let scheme = action.request.url?.scheme ?? ""
        if ["http", "https", "file", "about", "data", "blob"].contains(scheme) { return decisionHandler(.allow) }
        // tel: / weixin:// 这类交给系统
        if let u = action.request.url { UIApplication.shared.open(u) }
        decisionHandler(.cancel)
    }

    /// 返回：当前页的网页能后退就后退
    func goBack() -> Bool {
        guard let s = slots.values.first(where: { $0.depth == depth && $0.route == route && !$0.frame.isHidden }), s.web.canGoBack else { return false }
        s.web.goBack()
        return true
    }

    private func destroy(_ s: Slot) {
        s.titleObs?.invalidate()
        s.web.stopLoading()
        s.web.configuration.userContentController.removeAllScriptMessageHandlers()
        s.frame.removeFromSuperview()
        slots[s.id] = nil
        order.removeAll { $0 == s.id }
    }

    func destroyAll() { slots.values.forEach(destroy) }

    /// `userContentController.add` 强引用 handler：中间隔一层弱引用，不然 WebView 与这里互相拿着谁都放不掉
    private final class WeakHandler: NSObject, WKScriptMessageHandler {
        weak var target: WKScriptMessageHandler?
        init(_ t: WKScriptMessageHandler) { target = t }
        func userContentController(_ uc: WKUserContentController, didReceive m: WKScriptMessage) { target?.userContentController(uc, didReceive: m) }
    }

    /// 网页里的 JSSDK（与 Android `MiniWebViews.kt` 里那份同一段，只有发消息的方式不同）
    static let shim = """
    (function(){
      if (window.__miniShim) return; window.__miniShim = true;
      var send = function(s){ window.webkit.messageHandlers.miniBridge.postMessage(s); };
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
    """
}
