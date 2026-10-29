import QuartzCore
import UIKit
#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

/// 小游戏的宿主视图（与 Android 的 `MiniGameView.kt` 对应，game-doc/02 §12）：引擎线程 + CADisplayLink（[GameSession]），
/// 两块图层：WebGL 主画布经 ANGLE 直接 swap 进 [metalLayer]，2D 主画布的 CPU 帧放 [imageLayer]。
///
/// 用法：
/// ```swift
/// let v = MiniGameView(gameDir: dir, options: ["appId": "wx123", "hostApis": ["login"]])
/// v.onHostEvent = { type, ev in if type == "exit" { … } }
/// view.addSubview(v)                       // 布局之后才建实例（尺寸要真的）
/// …
/// v.close()                                // 页面关掉时
/// ```
/// 方向：游戏要横屏时 [orientationMask] 是横屏，宿主控制器的 `supportedInterfaceOrientations` 返回它并请求转屏
/// （[MiniGameViewController] 已经做好）；转过来之前不建实例（最多等 1.5s），否则游戏启动时读到竖屏尺寸、整屏被拉伸。
public final class MiniGameView: UIView, GameSessionUI {
    /// 引擎交给宿主的事件（主线程）：`exit` / `restart`（wx.exitMiniProgram / restartMiniProgram）、`hostCall`（创建选项
    /// hostApis 点名的 wx.*，用 [hostReply] 回结果）、`report`、`vibrate`、`clipboard`、`shareMenu` …；type 与原始 JSON
    public var onHostEvent: ((_ type: String, _ event: [String: Any]) -> Void)?
    /// 第一帧真正上屏了（主线程，只回调一次）：loading 层盖到这里再撤
    public var onFirstFrame: (() -> Void)?
    /// 游戏要的方向变了（game.json 的 deviceOrientation、wx.setDeviceOrientation）：宿主据此转屏
    public var onOrientationChange: ((UIInterfaceOrientationMask) -> Void)?
    /// 游戏要的方向（宿主控制器的 supportedInterfaceOrientations 返回它）
    public private(set) var orientationMask: UIInterfaceOrientationMask = .portrait

    public let gameDir: String
    public let dataDir: String
    private let options: [String: Any]
    private let session: GameSession
    private let metalLayer = CAMetalLayer()
    private let imageLayer = CALayer()
    private var started = false
    private var closed = false
    private var wantLandscape = false
    private var orientationWaitOver = false
    private var lastSize = CGSize.zero
    /// 引擎要的图层像素尺寸（WebGL 主画布的尺寸）
    private var layerPixels = CGSize.zero
    /// 触摸 → 引擎的小整数 id
    private var touchIds: [ObjectIdentifier: Int32] = [:]
    /// 右上角胶囊（`"menuButton": false` / 引擎内旋转时不要）
    private var capsule: GameCapsule?
    private lazy var keyboard = GameKeyboard(host: self) { [weak self] n, j in self?.dispatchEvent(n, json: j) }
    private lazy var video = GameVideo(host: self, dir: gameDir) { [weak self] n, j in self?.dispatchEvent(n, json: j) }

    /// 小游戏存档的默认目录（Application Support/game-data；与 [MiniGame.precompile] 的默认一致）
    public static var defaultDataDir: String { MiniGame.defaultDataDir }

    /// [gameDir]：解好的小游戏包（根上有 game.json 与 game.js）。[options]：mr_game_create 的选项
    /// （appId、launchOptions、systemInfo、hostApis、adaptiveFps、rotateInEngine …）
    public init(gameDir: String, dataDir: String? = nil, options: [String: Any] = [:]) {
        self.gameDir = gameDir
        self.dataDir = dataDir ?? MiniGame.defaultDataDir
        self.options = options
        session = GameSession(dir: gameDir)
        super.init(frame: .zero)
        backgroundColor = .black
        isMultipleTouchEnabled = true
        for l in [imageLayer, metalLayer] {
            l.anchorPoint = .zero
            l.isHidden = true
            l.actions = ["contents": NSNull(), "bounds": NSNull(), "position": NSNull(), "transform": NSNull(), "hidden": NSNull()]
            layer.addSublayer(l)
        }
        imageLayer.contentsGravity = .resize
        imageLayer.magnificationFilter = .linear
        metalLayer.isOpaque = true
        metalLayer.contentsScale = 1
        metalLayer.pixelFormat = .bgra8Unorm
        session.ui = self
        // 胶囊：「•••」/「⊙」交给宿主（onHostEvent("menuButton", {action})）。引擎内旋转时坐标系是转过的，不画
        if options["menuButton"] as? Bool ?? true, !(options["rotateInEngine"] as? Bool ?? false) {
            let c = GameCapsule(frame: .zero)
            c.isHidden = true
            c.onTap = { [weak self] a in self?.onHostEvent?("menuButton", ["type": "menuButton", "action": a]) }
            addSubview(c)
            capsule = c
        }
        try? FileManager.default.createDirectory(atPath: self.dataDir, withIntermediateDirectories: true)
        // 建实例之前就按 game.json 定方向（rotateInEngine：引擎自己把横屏画面转进竖屏，宿主锁竖屏）
        let o = Self.gameOrientation(gameDir)
        let inEngine = options["rotateInEngine"] as? Bool ?? false
        setOrientation(inEngine ? "portrait" : o)
        wantLandscape = !inEngine && o.hasPrefix("landscape")
        // 兜底：设备转不过来（iPad 锁了方向、分屏）就按现有尺寸开，别一直黑着
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { [weak self] in
            self?.orientationWaitOver = true
            self?.startIfReady()
        }
        let nc = NotificationCenter.default
        nc.addObserver(self, selector: #selector(onForeground), name: UIApplication.didBecomeActiveNotification, object: nil)
        nc.addObserver(self, selector: #selector(onBackground), name: UIApplication.willResignActiveNotification, object: nil)
        nc.addObserver(self, selector: #selector(onMemoryWarning), name: UIApplication.didReceiveMemoryWarningNotification, object: nil)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    deinit { NotificationCenter.default.removeObserver(self) }

    // ───────────────────────── 对外 ─────────────────────────

    /// 回 hostCall（`id` 是事件里的 id；`result` 是 success 的结果对象，或 fail 的 `{errMsg}`）。任意线程调用
    public func hostReply(_ id: Int, ok: Bool, result: [String: Any] = [:]) {
        let json = GameSensors.json(result)
        session.run { _ = mr_game_host_reply($0, UInt32(max(0, id)), ok ? 1 : 0, json) }
    }

    /// 宿主推给游戏的事件（contextEvent、gamepad、pointerLockChange …，见 mr_game_dispatch_event）。任意线程调用
    public func dispatchEvent(_ name: String, json: String = "{}") {
        session.run { _ = mr_game_dispatch_event($0, name, json) }
    }

    /// 诊断：在逻辑层执行 JS（结果回到主线程）
    public func eval(_ js: String, _ done: @escaping (String?) -> Void) { session.eval(js, done) }

    /// 关掉（销毁实例、停引擎线程）。之后这个视图不能再用
    public func close() {
        guard !closed else { return }
        closed = true
        keyboard.hide()
        video.release()
        session.dispose()
    }

    // ───────────────────────── 布局与建实例 ─────────────────────────

    public override func layoutSubviews() {
        super.layoutSubviews()
        layoutLayers()
        keyboard.layout()
        video.relayout()
        let s = bounds.size
        guard s.width > 0, s.height > 0, s != lastSize else { return }
        lastSize = s
        if !started { return startIfReady() }
        let dpr = Float(window?.screen.scale ?? UIScreen.main.scale)
        session.resize(width: UInt32(s.width), height: UInt32(s.height), dpr: dpr, orientation: currentOrientation())
    }

    private func startIfReady() {
        guard !started, !closed, bounds.width > 0, bounds.height > 0 else { return }
        // 方向还没转过来就按竖屏尺寸建的话，游戏启动时读一次 windowWidth/Height 定死画布，转过来后整屏被拉伸
        if !orientationWaitOver && (bounds.width > bounds.height) != wantLandscape { return }
        started = true
        let dpr = Float(window?.screen.scale ?? UIScreen.main.scale)
        session.create(dataDir: dataDir, width: UInt32(bounds.width), height: UInt32(bounds.height), dpr: dpr, options: optionsJson())
        session.attachLayer(metalLayer)
    }

    /// 两块图层都铺满视图：CPU 帧按内容拉伸；EGL 图层的 bounds 就是画布像素（contentsScale 1，ANGLE 按它定 drawable 尺寸），
    /// 用 transform 缩放到视图大小
    private func layoutLayers() {
        imageLayer.frame = bounds
        let px = layerPixels
        guard px.width > 0, px.height > 0, bounds.width > 0 else { return }
        metalLayer.transform = CATransform3DIdentity
        metalLayer.bounds = CGRect(origin: .zero, size: px)
        metalLayer.drawableSize = px
        metalLayer.position = .zero
        metalLayer.transform = CATransform3DMakeScale(bounds.width / px.width, bounds.height / px.height, 1)
    }

    /// 创建选项：宿主给的 + 设备信息（安全区、机型、跑分档）+ 传感器那几个 hostApis
    private func optionsJson() -> String {
        var o = options
        var apis = o["hostApis"] as? [String] ?? []
        apis.append(contentsOf: GameSensors.apis.filter { !apis.contains($0) })
        o["hostApis"] = apis
        var si = o["systemInfo"] as? [String: Any] ?? [:]
        // 屏幕内缩：引擎按方向算 safeArea / statusBarHeight / 胶囊位置（src/game/chrome.rs），转屏后 safeAreaInsetsDidChange 再报
        let i = safeAreaInsets
        reportedInsets = i
        if o["safeArea"] == nil { o["safeArea"] = ["top": i.top, "right": i.right, "bottom": i.bottom, "left": i.left] }
        si["brand"] = si["brand"] ?? "Apple"
        si["model"] = si["model"] ?? MiniGame.model
        si["system"] = si["system"] ?? "iOS \(UIDevice.current.systemVersion)"
        if si["benchmarkLevel"] == nil {
            // 不是微信的跑分口径，只保证「高中低」三档的方向对（与 Android 同一条规则）
            let gb = ProcessInfo.processInfo.physicalMemory >> 30
            let cores = ProcessInfo.processInfo.activeProcessorCount
            let (lv, ml) = cores >= 6 && gb >= 7 ? (40, 1) : cores >= 6 && gb >= 3 ? (20, 2) : (8, 3)
            si["benchmarkLevel"] = lv
            si["modelLevel"] = ml
        }
        o["systemInfo"] = si
        return GameSensors.json(o)
    }

    /// 最近一次报给引擎的内缩（同一份不重复报）
    private var reportedInsets: UIEdgeInsets?

    public override func safeAreaInsetsDidChange() {
        super.safeAreaInsetsDidChange()
        let i = safeAreaInsets
        guard started, reportedInsets != i else { return }
        reportedInsets = i
        session.setSafeArea(i)
    }

    // ───────────────────────── 方向 ─────────────────────────

    static func gameOrientation(_ dir: String) -> String {
        guard let d = FileManager.default.contents(atPath: (dir as NSString).appendingPathComponent("game.json")),
              let j = (try? JSONSerialization.jsonObject(with: d)) as? [String: Any] else { return "portrait" }
        return j["deviceOrientation"] as? String ?? "portrait"
    }

    /// game.json / wx.setDeviceOrientation 的方向 → 宿主控制器要支持的方向。
    /// 官方：landscapeLeft = home 键在左（UIKit 的 .landscapeLeft 也是 home 键在左），landscapeRight = home 键在右
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

    /// 现在的方向（引擎的 "portrait" / "landscape" / "landscapeReverse"）：home 键在右 = landscape（与 Android 的 ROTATION_90 同向）
    private func currentOrientation() -> String {
        var o: UIInterfaceOrientation = .unknown
        if #available(iOS 13.0, *) { o = window?.windowScene?.interfaceOrientation ?? .unknown }
        switch o {
        case .landscapeRight: return "landscape"
        case .landscapeLeft: return "landscapeReverse"
        default: return bounds.width > bounds.height ? "landscape" : "portrait"
        }
    }

    // ───────────────────────── GameSessionUI ─────────────────────────

    func gameLayerWant(width: Int, height: Int) {
        layerPixels = CGSize(width: width, height: height)
        layoutLayers()
        session.layerSized(width: width, height: height)
    }

    func gameShowLayer(metal: Bool) {
        metalLayer.isHidden = !metal
        imageLayer.isHidden = metal
        if metal { imageLayer.contents = nil }
    }

    func gameCpuFrame(_ image: CGImage) {
        imageLayer.contents = image
    }

    func gameFirstFrame() { onFirstFrame?() }

    func gameMenuButton(_ rect: CGRect) {
        guard let c = capsule else { return }
        c.frame = rect
        c.isHidden = rect.width <= 0 || rect.height <= 0
        bringSubviewToFront(c)
    }

    func gameEvents(_ events: [[String: Any]]) {
        for ev in events {
            let type = ev["type"] as? String ?? ""
            switch type {
            case "showKeyboard": keyboard.show(ev)
            case "updateKeyboard": keyboard.update(ev)
            case "hideKeyboard": keyboard.hide()
            case "video": video.handle(ev)
            // wx.setMenuStyle：深色胶囊（白图标）/ 浅色
            case "menuStyle": capsule?.dark = ev["style"] as? String == "dark"
            // initial 的那条建实例之前已经按 game.json 定过了；wx.setDeviceOrientation 的才要转
            case "deviceOrientation":
                if !(ev["initial"] as? Bool ?? false) { setOrientation(ev["value"] as? String ?? "portrait") }
            case "clipboard":
                UIPasteboard.general.string = ev["data"] as? String ?? ""
                onHostEvent?(type, ev)
            case "vibrate":
                (ev["style"] as? String == "heavy" ? UIImpactFeedbackGenerator(style: .heavy) : UIImpactFeedbackGenerator(style: .light)).impactOccurred()
                onHostEvent?(type, ev)
            case "hostCall":
                if let cb = onHostEvent { cb(type, ev) } else {
                    hostReply((ev["id"] as? NSNumber)?.intValue ?? 0, ok: false, result: ["errMsg": "\(ev["api"] ?? ""):fail 宿主没有实现"])
                }
            default:
                NSLog("[mini-game] 事件 %@", type)
                onHostEvent?(type, ev)
            }
        }
    }

    // ───────────────────────── 触摸 ─────────────────────────

    private func send(_ touches: Set<UITouch>, _ event: UIEvent?, phase: Int32) {
        var list: [(phase: Int32, id: Int32, x: Float, y: Float, force: Float, t: TimeInterval)] = []
        for t in touches {
            let key = ObjectIdentifier(t)
            if phase == 0 {
                var id: Int32 = 0
                while touchIds.values.contains(id) { id += 1 }
                touchIds[key] = id
            }
            guard let id = touchIds[key] else { continue }
            // 移动：带上合并掉的中间采样（游戏算手势速度要采样密度），时刻是 UITouch.timestamp（与 DisplayLink 同一个钟）
            let samples = phase == 1 ? (event?.coalescedTouches(for: t) ?? [t]) : [t]
            for s in samples {
                let p = s.location(in: self)
                let f = phase >= 2 || s.maximumPossibleForce <= 0 ? 0 : Float(s.force / s.maximumPossibleForce)
                list.append((phase, id, Float(p.x), Float(p.y), min(1, max(0, f)), s.timestamp))
            }
            if phase >= 2 { touchIds.removeValue(forKey: key) }
        }
        if !list.isEmpty { session.touches(list) }
    }

    public override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, event, phase: 0) }
    public override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, event, phase: 1) }
    public override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, event, phase: 2) }
    public override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, event, phase: 3) }

    // ─────────────────────── 生命周期 ───────────────────────

    @objc private func onForeground() { if !closed { session.foreground() } }

    /// 进后台（含来电、下拉控制中心）：wx.onHide 并**同步**停掉出帧（EGL / Metal 在后台提交会被杀进程）
    @objc private func onBackground() { if !closed { session.background() } }

    /// iOS 的内存告警只有一档，按 Android 的 TRIM_MEMORY_RUNNING_LOW（10）报：派发 wx.onMemoryWarning 并回收
    @objc private func onMemoryWarning() {
        session.run { mr_game_memory_warning($0, 10) }
    }
}

/// 小游戏的静态能力：包种类、库能力、预编译
public enum MiniGame {
    /// 包种类：0 = 认不出，1 = 小程序（app.json），2 = 小游戏（game.json）
    public static func packageKind(_ dir: String) -> Int { Int(mr_package_kind(dir)) }

    /// 本库编进了哪些能力（game / webgl / audio / wasm / gpu / version / sdkVersion）
    public static var capabilities: [String: Any] {
        guard let p = mr_capabilities() else { return [:] }
        return (try? JSONSerialization.jsonObject(with: Data(String(cString: p).utf8))) as? [String: Any] ?? [:]
    }

    /// 存档 / 字节码缓存的默认目录（Application Support/game-data：不进 Caches，系统清缓存不丢存档）
    public static var defaultDataDir: String {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? URL(fileURLWithPath: NSTemporaryDirectory())
        return base.appendingPathComponent("game-data").path
    }

    /// 下载 / 解包完就在后台把包里的大脚本编成字节码（第一次打开也不用解析）。立刻返回；马上打开也没关系，
    /// 实例碰到正在编的脚本会等它编完。[dataDir] 与之后建视图给的同一个
    public static func precompile(dir: String, dataDir: String? = nil) {
        let data = dataDir ?? defaultDataDir
        try? FileManager.default.createDirectory(atPath: data, withIntermediateDirectories: true)
        let t = Thread { _ = mr_game_precompile(dir, data) }
        t.name = "mini-game-precompile"
        t.qualityOfService = .userInitiated
        t.start()
    }

    /// 机型标识（iPhone17,1 这类；模拟器取它模拟的那台）
    static var model: String {
        if let sim = ProcessInfo.processInfo.environment["SIMULATOR_MODEL_IDENTIFIER"] { return sim }
        var u = utsname()
        uname(&u)
        return withUnsafeBytes(of: &u.machine) { String(decoding: $0.prefix { $0 != 0 }, as: UTF8.self) }
    }
}
