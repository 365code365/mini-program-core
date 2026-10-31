import UIKit
import WebKit
#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

/// 小游戏的**高性能模式**宿主视图（对标微信 game.json 的 `iOSHighPerformance`，game-doc/12）：游戏 JS 跑在 WKWebView 里 ——
/// JavaScriptCore 带 JIT、WebGL / Canvas 2D 是 WebKit 自己的 GPU 实现。iOS 不许 App 自己 JIT，WKWebView 是唯一的路
/// （审核指南 4.7：HTML5 / JS 小游戏可以下发，跑在 WebKit 里）。
///
/// 这里只做传话，逻辑全在 Rust（src/game/web）与页面里的基础库：
/// - `wxgame://app/…`（WKURLSchemeHandler，后台线程）→ `mr_wg_resource`：包内文件、启动页、wxfile、远程资源代理；
/// - 页面的同步调用 `prompt('__wg', 消息)`（WKUIDelegate）→ `mr_wg_call`；异步消息 `webkit.messageHandlers.wg` → `mr_wg_post`；
/// - `mr_wg_take_js` 的结果 `evaluateJavaScript` 回页面（网络回包、宿主回包、前后台、转屏），有活时每个 vsync 取一次；
/// - `mr_wg_take_event` 的宿主事件与 [MiniGameView] 同一套（hostCall、键盘、振动、退出……），对外接口也相同。
///
/// 与 QuickJS 模式（[MiniGameView]）共用：存档、wxfile、网络、FileSystemManager、系统信息 / 胶囊的算法、键盘 / 视频 / 传感器 / 胶囊的原生部分。
/// 不支持：开放数据域、引擎插件（[MiniGameViewController] 看到就回落 QuickJS 模式）。
public final class MiniWebGameView: UIView, WKScriptMessageHandler, WKUIDelegate, WKURLSchemeHandler, WKNavigationDelegate {
    public var onHostEvent: ((_ type: String, _ event: [String: Any]) -> Void)?
    public var onFirstFrame: (() -> Void)?
    public var onOrientationChange: ((UIInterfaceOrientationMask) -> Void)?
    public private(set) var orientationMask: UIInterfaceOrientationMask = .portrait

    public let gameDir: String
    public let dataDir: String
    private let options: [String: Any]
    private var handle: OpaquePointer?
    private var web: WKWebView?
    private var started = false
    private var closed = false
    private var wantLandscape = false
    private var orientationWaitOver = false
    private var lastSize = CGSize.zero
    private var reportedInsets: UIEdgeInsets?
    private var capsule: GameCapsule?
    private var link: CADisplayLink?
    private var firstFrameSent = false
    /// 协议处理器在后台线程回包；页面停掉了的任务不能再回（会崩），按任务对象记着
    private let tasksLock = NSLock()
    private var liveTasks = Set<ObjectIdentifier>()
    private let ioQueue = DispatchQueue(label: "mini-game-web.io", qos: .userInitiated, attributes: .concurrent)
    private lazy var keyboard = GameKeyboard(host: self) { [weak self] n, j in self?.dispatchEvent(n, json: j) }
    private lazy var video = GameVideo(host: self, dir: gameDir) { [weak self] n, j in self?.dispatchEvent(n, json: j) }
    private lazy var sensors = GameSensors(post: { b in DispatchQueue.main.async(execute: b) }) { [weak self] n, j in self?.dispatchEvent(n, json: j) }

    public init(gameDir: String, dataDir: String? = nil, options: [String: Any] = [:]) {
        self.gameDir = gameDir
        self.dataDir = dataDir ?? MiniGame.defaultDataDir
        self.options = options
        super.init(frame: .zero)
        backgroundColor = .black
        if options["menuButton"] as? Bool ?? true {
            let c = GameCapsule(frame: .zero)
            c.isHidden = true
            c.onTap = { [weak self] a in self?.onHostEvent?("menuButton", ["type": "menuButton", "action": a]) }
            addSubview(c)
            capsule = c
        }
        try? FileManager.default.createDirectory(atPath: self.dataDir, withIntermediateDirectories: true)
        let o = MiniGameView.gameOrientation(gameDir)
        setOrientation(o)
        wantLandscape = o.hasPrefix("landscape")
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { [weak self] in
            self?.orientationWaitOver = true
            self?.startIfReady()
        }
        let nc = NotificationCenter.default
        nc.addObserver(self, selector: #selector(onForeground), name: UIApplication.didBecomeActiveNotification, object: nil)
        nc.addObserver(self, selector: #selector(onBackground), name: UIApplication.willResignActiveNotification, object: nil)
        nc.addObserver(self, selector: #selector(onMemoryWarning), name: UIApplication.didReceiveMemoryWarningNotification, object: nil)
        nc.addObserver(self, selector: #selector(onThermal), name: ProcessInfo.thermalStateDidChangeNotification, object: nil)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    deinit {
        NotificationCenter.default.removeObserver(self)
        destroyHandle()
    }

    /// 销毁实例：排在后台取资源的任务之后（barrier），它们用完句柄才释放
    private func destroyHandle() {
        guard let h = handle else { return }
        handle = nil
        let raw = UInt(bitPattern: h)
        ioQueue.async(flags: .barrier) { mr_wg_destroy(OpaquePointer(bitPattern: raw)) }
    }

    // ───────────────────────── 对外（与 MiniGameView 相同） ─────────────────────────

    public func hostReply(_ id: Int, ok: Bool, result: [String: Any] = [:]) {
        let json = GameSensors.json(result)
        main { [self] in
            guard let h = handle else { return }
            mr_wg_host_reply(h, UInt32(max(0, id)), ok ? 1 : 0, json)
            pump()
        }
    }

    public func dispatchEvent(_ name: String, json: String = "{}") {
        main { [self] in
            guard let h = handle else { return }
            mr_wg_dispatch_event(h, name, json)
            pump()
        }
    }

    public func eval(_ js: String, _ done: @escaping (String?) -> Void) {
        main { [self] in
            web?.evaluateJavaScript(js) { r, _ in done(r.map { "\($0)" }) }
        }
    }

    public func close() {
        guard !closed else { return }
        closed = true
        link?.invalidate()
        link = nil
        keyboard.hide()
        video.release()
        sensors.release()
        if let w = web {
            w.configuration.userContentController.removeScriptMessageHandler(forName: "wg")
            w.stopLoading()
            w.removeFromSuperview()
        }
        web = nil
        destroyHandle()
    }

    private func main(_ b: @escaping () -> Void) {
        if Thread.isMainThread { b() } else { DispatchQueue.main.async(execute: b) }
    }

    // ───────────────────────── 建实例与布局 ─────────────────────────

    public override func layoutSubviews() {
        super.layoutSubviews()
        web?.frame = bounds
        keyboard.layout()
        video.relayout()
        let s = bounds.size
        guard s.width > 0, s.height > 0, s != lastSize else { return }
        lastSize = s
        if !started { return startIfReady() }
        guard let h = handle else { return }
        mr_wg_resize(h, UInt32(s.width), UInt32(s.height), Float(window?.screen.scale ?? UIScreen.main.scale), currentOrientation())
        placeCapsule()
        pump()
    }

    private func startIfReady() {
        guard !started, !closed, bounds.width > 0, bounds.height > 0 else { return }
        if !orientationWaitOver && (bounds.width > bounds.height) != wantLandscape { return }
        started = true
        let dpr = Float(window?.screen.scale ?? UIScreen.main.scale)
        let t0 = CACurrentMediaTime()
        guard let h = mr_wg_create(gameDir, dataDir, UInt32(bounds.width), UInt32(bounds.height), dpr, optionsJson()) else {
            NSLog("❌ mini-game（高性能模式）：建实例失败 \(gameDir)")
            return
        }
        handle = h
        let cfg = WKWebViewConfiguration()
        cfg.setURLSchemeHandler(self, forURLScheme: "wxgame")
        cfg.userContentController.add(WeakHandler(self), name: "wg")
        // 帧统计（-gameStats 1）：页面每 2 秒一行，与 QuickJS 模式同一格式（tools/ios-game-sweep.sh 两种模式一起比）
        if GameFrameStats.enabled {
            cfg.userContentController.addUserScript(WKUserScript(source: "window.__wgStats = true;", injectionTime: .atDocumentStart, forMainFrameOnly: true))
        }
        cfg.allowsInlineMediaPlayback = true
        cfg.mediaTypesRequiringUserActionForPlayback = []
        cfg.suppressesIncrementalRendering = false
        if #available(iOS 14.0, *) { cfg.defaultWebpagePreferences.allowsContentJavaScript = true }
        let w = WKWebView(frame: bounds, configuration: cfg)
        w.isOpaque = true
        w.backgroundColor = .black
        w.scrollView.backgroundColor = .black
        w.scrollView.isScrollEnabled = false
        w.scrollView.bounces = false
        w.scrollView.contentInsetAdjustmentBehavior = .never
        w.scrollView.pinchGestureRecognizer?.isEnabled = false
        w.allowsLinkPreview = false
        w.uiDelegate = self
        w.navigationDelegate = self
        if #available(iOS 16.4, *) { w.isInspectable = options["inspectable"] as? Bool ?? true }
        insertSubview(w, at: 0)
        web = w
        NSLog("🎮 建实例（高性能模式）%.0fms（%d×%d@%.1f，%@）", (CACurrentMediaTime() - t0) * 1000, Int(bounds.width), Int(bounds.height), dpr, gameDir)
        w.load(URLRequest(url: URL(string: "wxgame://app/__wg.html")!))
        placeCapsule()
        let l = CADisplayLink(target: WeakTarget(self), selector: #selector(WeakTarget.tick))
        l.add(to: .main, forMode: .common)
        link = l
    }

    private func optionsJson() -> String {
        var o = options
        var apis = o["hostApis"] as? [String] ?? []
        apis.append(contentsOf: GameSensors.apis.filter { !apis.contains($0) })
        o["hostApis"] = apis
        let i = safeAreaInsets
        reportedInsets = i
        if o["safeArea"] == nil { o["safeArea"] = ["top": i.top, "right": i.right, "bottom": i.bottom, "left": i.left] }
        var si = o["systemInfo"] as? [String: Any] ?? [:]
        si["brand"] = si["brand"] ?? "Apple"
        si["model"] = si["model"] ?? MiniGame.model
        si["system"] = si["system"] ?? "iOS \(UIDevice.current.systemVersion)"
        if si["benchmarkLevel"] == nil {
            let gb = ProcessInfo.processInfo.physicalMemory >> 30
            let cores = ProcessInfo.processInfo.activeProcessorCount
            let (lv, ml) = cores >= 6 && gb >= 7 ? (40, 1) : cores >= 6 && gb >= 3 ? (20, 2) : (8, 3)
            si["benchmarkLevel"] = lv
            si["modelLevel"] = ml
        }
        o["systemInfo"] = si
        return GameSensors.json(o)
    }

    public override func safeAreaInsetsDidChange() {
        super.safeAreaInsetsDidChange()
        let i = safeAreaInsets
        guard let h = handle, reportedInsets != i else { return }
        reportedInsets = i
        mr_wg_set_safe_area(h, Float(i.top), Float(i.right), Float(i.bottom), Float(i.left))
        placeCapsule()
        pump()
    }

    private func placeCapsule() {
        guard let c = capsule, let h = handle else { return }
        var x: Float = 0, y: Float = 0, w: Float = 0, hh: Float = 0
        mr_wg_menu_button(h, &x, &y, &w, &hh)
        c.frame = CGRect(x: CGFloat(x), y: CGFloat(y), width: CGFloat(w), height: CGFloat(hh))
        c.isHidden = w <= 0
        bringSubviewToFront(c)
    }

    // ───────────────────────── 方向 ─────────────────────────

    private func setOrientation(_ value: String) {
        let m: UIInterfaceOrientationMask
        switch value {
        case "landscape": m = .landscape
        case "landscapeLeft": m = .landscapeLeft
        case "landscapeRight": m = .landscapeRight
        default: m = .portrait
        }
        orientationMask = m
        onOrientationChange?(m)
    }

    private func currentOrientation() -> String {
        switch window?.windowScene?.interfaceOrientation ?? .unknown {
        case .landscapeRight: return "landscape"
        case .landscapeLeft: return "landscapeReverse"
        default: return bounds.width > bounds.height ? "landscape" : "portrait"
        }
    }

    // ───────────────────────── 页面 ↔ 原生 ─────────────────────────

    /// 异步消息（webkit.messageHandlers.wg.postMessage）
    public func userContentController(_ uc: WKUserContentController, didReceive message: WKScriptMessage) {
        guard let h = handle, let s = message.body as? String else { return }
        mr_wg_post(h, s)
        pump()
    }

    /// 同步调用（prompt('__wg', 消息)）：WKUIDelegate 的 completionHandler 回什么，页面里的 prompt 就返回什么
    public func webView(_ webView: WKWebView, runJavaScriptTextInputPanelWithPrompt prompt: String, defaultText: String?,
                        initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping (String?) -> Void) {
        guard prompt == "__wg", let h = handle else { return completionHandler(nil) }
        let p = mr_wg_call(h, defaultText ?? "")
        let r = p.map { String(cString: $0) } ?? ""
        mr_wg_free_string(p)
        completionHandler(r)
        pump()
    }

    // alert / confirm：游戏里偶尔有（调试遗留），给个系统弹窗，别让页面卡死在等回调上
    public func webView(_ webView: WKWebView, runJavaScriptAlertPanelWithMessage message: String, initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping () -> Void) {
        NSLog("[mini-game] alert: %@", message)
        completionHandler()
    }
    public func webView(_ webView: WKWebView, runJavaScriptConfirmPanelWithMessage message: String, initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping (Bool) -> Void) {
        completionHandler(true)
    }

    /// 只许页面待在 wxgame://app 上（游戏代码设 location 跳走的，拦下）
    public func webView(_ webView: WKWebView, decidePolicyFor action: WKNavigationAction, decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
        let ok = action.request.url?.scheme == "wxgame" || action.targetFrame?.isMainFrame == false
        if !ok { NSLog("⚠️ [mini-game] 拦下页面跳转 %@", action.request.url?.absoluteString ?? "") }
        decisionHandler(ok ? .allow : .cancel)
    }

    /// WebContent 进程被系统杀了（内存超限）：与微信一样重启这一局（宿主收到 restart 事件）
    public func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
        NSLog("⚠️ [mini-game] WebContent 进程退出（内存超限？），重启小游戏")
        onHostEvent?("restart", ["type": "restart", "reason": "webContentTerminated"])
    }

    public func webView(_ webView: WKWebView, start task: WKURLSchemeTask) {
        guard let h = handle, let url = task.request.url else { return task.didFailWithError(URLError(.badURL)) }
        let key = ObjectIdentifier(task)
        tasksLock.lock(); liveTasks.insert(key); tasksLock.unlock()
        var path = url.path.isEmpty ? "/" : url.path
        if let q = url.query { path += "?" + q }
        // 包内文件、远程代理在后台线程取（代理会阻塞在网络上）
        ioQueue.async { [weak self] in
            var len = 0
            var mime: UnsafePointer<CChar>?
            let ptr = mr_wg_resource(h, path, &len, &mime)
            let data = ptr.map { Data(bytes: $0, count: len) }
            mr_wg_free_bytes(ptr, len)
            let type = mime.map { String(cString: $0) } ?? "application/octet-stream"
            DispatchQueue.main.async {
                guard let self else { return }
                self.tasksLock.lock()
                let live = self.liveTasks.remove(key) != nil
                self.tasksLock.unlock()
                guard live else { return }
                let headers = ["Content-Type": type, "Content-Length": "\(data?.count ?? 0)", "Access-Control-Allow-Origin": "*", "Cache-Control": "no-store"]
                let resp = HTTPURLResponse(url: url, statusCode: data == nil ? 404 : 200, httpVersion: "HTTP/1.1", headerFields: headers)!
                task.didReceive(resp)
                if let d = data { task.didReceive(d) }
                task.didFinish()
            }
        }
    }

    public func webView(_ webView: WKWebView, stop task: WKURLSchemeTask) {
        tasksLock.lock(); liveTasks.remove(ObjectIdentifier(task)); tasksLock.unlock()
    }

    /// 取完原生交回来的 JS 与宿主事件。有异步活在等（网络请求在飞）时 DisplayLink 每个 vsync 来一次，没活就停
    fileprivate func pump() {
        guard let h = handle, !closed else { return }
        while let p = mr_wg_take_event(h) {
            let s = String(cString: p)
            mr_wg_free_string(p)
            if let ev = (try? JSONSerialization.jsonObject(with: Data(s.utf8))) as? [String: Any] { handleEvent(ev, raw: s) }
        }
        if let p = mr_wg_take_js(h) {
            let js = String(cString: p)
            mr_wg_free_string(p)
            web?.evaluateJavaScript(js) { _, err in if let e = err { NSLog("⚠️ [mini-game] 回页面的 JS 执行出错：%@", e.localizedDescription) } }
        }
        link?.isPaused = mr_wg_pending(h) == 0
    }

    private func handleEvent(_ ev: [String: Any], raw: String) {
        let type = ev["type"] as? String ?? ""
        switch type {
        case "log":
            (ev["text"] as? String ?? "").split(separator: "\n", omittingEmptySubsequences: false).forEach { NSLog("[mini-game] %@", String($0)) }
        case "error": NSLog("⚠️ [mini-game] %@", raw)
        case "started": NSLog("🎮 game.js 执行完（高性能模式）")
        case "firstFrame":
            guard !firstFrameSent else { return }
            firstFrameSent = true
            NSLog("🎮 第一帧上屏（高性能模式）")
            onFirstFrame?()
        case "showKeyboard": keyboard.show(ev)
        case "updateKeyboard": keyboard.update(ev)
        case "hideKeyboard": keyboard.hide()
        case "hostRequest" where ev["kind"] as? String == "hideKeyboard": keyboard.hide()
        case "video": video.handle(ev)
        case "menuStyle": capsule?.dark = ev["style"] as? String == "dark"
        case "deviceOrientation": setOrientation(ev["value"] as? String ?? "portrait")
        case "clipboard":
            UIPasteboard.general.string = ev["data"] as? String ?? ""
            onHostEvent?(type, ev)
        case "vibrate":
            (ev["style"] as? String == "heavy" ? UIImpactFeedbackGenerator(style: .heavy) : UIImpactFeedbackGenerator(style: .light)).impactOccurred()
            onHostEvent?(type, ev)
        case "hostCall":
            let id = (ev["id"] as? NSNumber)?.intValue ?? 0
            if let r = sensors.handle(api: ev["api"] as? String ?? "", args: ev["args"] as? [String: Any]) {
                guard let h = handle else { return }
                mr_wg_host_reply(h, UInt32(id), r.ok ? 1 : 0, r.json)
            } else if let cb = onHostEvent {
                cb(type, ev)
            } else {
                hostReply(id, ok: false, result: ["errMsg": "\(ev["api"] ?? ""):fail 宿主没有实现"])
            }
        default:
            onHostEvent?(type, ev)
        }
    }

    // ───────────────────────── 生命周期 ─────────────────────────

    @objc private func onForeground() {
        guard let h = handle, !closed else { return }
        sensors.resumeAll()
        mr_wg_set_background(h, 0)
        pump()
    }

    @objc private func onBackground() {
        guard let h = handle, !closed else { return }
        sensors.pauseAll()
        mr_wg_set_background(h, 1)
        pump()
    }

    @objc private func onMemoryWarning() { dispatchEvent("memoryWarning", json: "{\"level\":10}") }

    /// 发热档：页面的帧统计行带上（与 GameFrameStats 同一组词）
    @objc private func onThermal() {
        let t = ["正常", "温", "热", "过热"][min(3, ProcessInfo.processInfo.thermalState.rawValue)]
        main { [self] in web?.evaluateJavaScript("window.__wgThermal = '\(t)';") }
    }
}

/// WKUserContentController 强引用它的 handler：中间垫一层弱引用，视图才能释放
private final class WeakHandler: NSObject, WKScriptMessageHandler {
    weak var target: (NSObject & WKScriptMessageHandler)?
    init(_ t: NSObject & WKScriptMessageHandler) { target = t }
    func userContentController(_ uc: WKUserContentController, didReceive message: WKScriptMessage) {
        target?.userContentController(uc, didReceive: message)
    }
}

/// CADisplayLink 强引用 target：同样垫一层
private final class WeakTarget: NSObject {
    weak var view: MiniWebGameView?
    init(_ v: MiniWebGameView) { view = v }
    @objc func tick() { view?.pump() }
}
