import AppKit
import QuartzCore
#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

/// macOS 的小游戏视图（PC 微信式窗口里跑 game.json 包；iOS / Android 各有一个同名视图）。
///
/// - **上屏**：两块图层 —— WebGL 主画布经 ANGLE（`tools/fetch-angle.sh` 的 dylib，示例 App 拷进 Contents/Frameworks）
///   直接 swap 进 [metalLayer]；2D 主画布的 CPU 帧放 [imageLayer]；
/// - **输入**：PC 微信里游戏照样收触摸：鼠标左键同时报成触摸（id 0）与 `wx.onMouseDown/Move/Up`，右键 / 中键只报鼠标，
///   悬停移动报 `onMouseMove`，滚轮报 `wx.onWheel`，键盘报 `wx.onKeyDown / onKeyUp`（`key` / `code` 同 DOM）；
/// - **安全区与胶囊**：标题栏那一行（[topInset]，红绿灯）报成内缩，引擎按它算 safeArea 与胶囊位置；胶囊是原生 [GameCapsule]，
///   点了发 `onHostEvent("menuButton", {action: "menu" | "close"})`；
/// - **方向**：桌面不转屏，game.json 要横屏时宿主把窗口开成横的（[gameOrientation]，wx.setDeviceOrientation 走 [onOrientationChange]）。
public final class MiniGameView: NSView, GameSessionUI {
    public var onHostEvent: ((_ type: String, _ event: [String: Any]) -> Void)?
    /// 第一帧真正上屏了（只回调一次）：loading 层盖到这里再撤
    public var onFirstFrame: (() -> Void)?
    /// wx.setDeviceOrientation：宿主据此把窗口改成横的 / 竖的（"portrait" / "landscape" / "landscapeLeft" / "landscapeRight"）
    public var onOrientationChange: ((String) -> Void)?
    /// 顶上被标题栏（红绿灯）盖住的高度（pt）：报成安全区内缩，胶囊摆在它下面
    public var topInset: CGFloat = 0 {
        didSet { if started, topInset != oldValue { session.setSafeArea(top: topInset) } }
    }
    /// game.json 的 deviceOrientation（建视图时读好；宿主按它开窗口）
    public let gameOrientation: String

    public let gameDir: String
    public let dataDir: String
    private let options: [String: Any]
    private let session: GameSession
    private let metalLayer = CAMetalLayer()
    private let imageLayer = CALayer()
    private var started = false
    private var closed = false
    private var lastSize = CGSize.zero
    private var layerPixels = CGSize.zero
    private var capsule: GameCapsule?
    private lazy var keyboard = GameKeyboard(host: self) { [weak self] n, j in self?.dispatchEvent(n, json: j) }
    private var tracking: NSTrackingArea?
    /// 左键按着（拖动时报触摸移动）
    private var leftDown = false

    public init(gameDir: String, dataDir: String? = nil, options: [String: Any] = [:]) {
        self.gameDir = gameDir
        self.dataDir = dataDir ?? MiniGame.defaultDataDir
        self.options = options
        gameOrientation = MiniGame.orientation(gameDir)
        session = GameSession(dir: gameDir)
        super.init(frame: .zero)
        wantsLayer = true
        layer?.backgroundColor = NSColor.black.cgColor
        let none: [String: CAAction] = ["contents": NSNull(), "bounds": NSNull(), "position": NSNull(), "transform": NSNull(), "hidden": NSNull()]
        for l in [imageLayer, metalLayer] {
            l.anchorPoint = .zero
            l.isHidden = true
            l.actions = none
            layer?.addSublayer(l)
        }
        imageLayer.contentsGravity = .resize
        imageLayer.magnificationFilter = .linear
        metalLayer.isOpaque = true
        metalLayer.contentsScale = 1
        metalLayer.pixelFormat = .bgra8Unorm
        session.ui = self
        try? FileManager.default.createDirectory(atPath: self.dataDir, withIntermediateDirectories: true)
        if options["menuButton"] as? Bool ?? true {
            let c = GameCapsule(frame: .zero)
            c.isHidden = true
            c.onTap = { [weak self] a in self?.onHostEvent?("menuButton", ["type": "menuButton", "action": a]) }
            addSubview(c)
            capsule = c
        }
        let nc = NotificationCenter.default
        nc.addObserver(self, selector: #selector(onBackground), name: NSApplication.didHideNotification, object: nil)
        nc.addObserver(self, selector: #selector(onForeground), name: NSApplication.didUnhideNotification, object: nil)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    deinit {
        NotificationCenter.default.removeObserver(self)
        close()
    }

    public override var isFlipped: Bool { true }

    // ───────────────────────── 对外 ─────────────────────────

    public func hostReply(_ id: Int, ok: Bool, result: [String: Any] = [:]) {
        let json = Self.json(result)
        session.run { _ = mr_game_host_reply($0, UInt32(max(0, id)), ok ? 1 : 0, json) }
    }

    public func dispatchEvent(_ name: String, json: String = "{}") {
        session.run { _ = mr_game_dispatch_event($0, name, json) }
    }

    public func eval(_ js: String, _ done: @escaping (String?) -> Void) { session.eval(js, done) }

    /// 引擎当前这一帧（截图验收：Metal 层上的画面窗口截图截不到）
    public func snapshot(_ done: @escaping (NSImage?) -> Void) {
        session.snapshot { img in done(img.map { NSImage(cgImage: $0, size: .zero) }) }
    }

    /// 关掉（同步销毁实例、停引擎线程）。之后这个视图不能再用
    public func close() {
        if closed { return }
        closed = true
        keyboard.hide()
        session.dispose()
    }

    // ───────────────────────── 布局与建实例 ─────────────────────────

    private var dpr: Float { Float(window?.backingScaleFactor ?? 2) }

    public override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        guard let w = window else { return }
        NotificationCenter.default.addObserver(self, selector: #selector(onBackground), name: NSWindow.didMiniaturizeNotification, object: w)
        NotificationCenter.default.addObserver(self, selector: #selector(onForeground), name: NSWindow.didDeminiaturizeNotification, object: w)
        startIfReady()
    }

    public override func layout() {
        super.layout()
        layoutLayers()
        keyboard.layout()
        let s = bounds.size
        guard s.width > 0, s.height > 0, s != lastSize else { return }
        lastSize = s
        if !started { return startIfReady() }
        session.resize(width: UInt32(s.width), height: UInt32(s.height), dpr: dpr)
    }

    public override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let t = tracking { removeTrackingArea(t) }
        let t = NSTrackingArea(rect: bounds, options: [.mouseMoved, .activeInKeyWindow, .inVisibleRect], owner: self)
        addTrackingArea(t)
        tracking = t
    }

    private func startIfReady() {
        guard !started, !closed, window != nil, bounds.width > 0, bounds.height > 0 else { return }
        started = true
        lastSize = bounds.size
        session.create(dataDir: dataDir, width: UInt32(bounds.width), height: UInt32(bounds.height), dpr: dpr, options: optionsJson())
        session.attachLayer(metalLayer)
    }

    /// CPU 帧按内容拉伸铺满；EGL 图层的 bounds 就是画布像素（contentsScale 1，ANGLE 按它定 drawable 尺寸），用 transform 缩放到视图
    private func layoutLayers() {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        defer { CATransaction.commit() }
        imageLayer.frame = bounds
        let px = layerPixels
        guard px.width > 0, px.height > 0, bounds.width > 0 else { return }
        metalLayer.transform = CATransform3DIdentity
        metalLayer.bounds = CGRect(origin: .zero, size: px)
        metalLayer.drawableSize = px
        metalLayer.position = .zero
        metalLayer.transform = CATransform3DMakeScale(bounds.width / px.width, bounds.height / px.height, 1)
    }

    private func optionsJson() -> String {
        var o = options
        if o["safeArea"] == nil { o["safeArea"] = ["top": topInset, "right": 0, "bottom": 0, "left": 0] }
        var si = o["systemInfo"] as? [String: Any] ?? [:]
        let os = ProcessInfo.processInfo.operatingSystemVersion
        si["brand"] = si["brand"] ?? "Apple"
        si["model"] = si["model"] ?? "Mac"
        si["system"] = si["system"] ?? "macOS \(os.majorVersion).\(os.minorVersion)"
        // 桌面机按高档报（与手机同一条规则：核数与内存）
        if si["benchmarkLevel"] == nil {
            let gb = ProcessInfo.processInfo.physicalMemory >> 30
            let cores = ProcessInfo.processInfo.activeProcessorCount
            let (lv, ml) = cores >= 6 && gb >= 7 ? (40, 1) : cores >= 6 && gb >= 3 ? (20, 2) : (8, 3)
            si["benchmarkLevel"] = lv
            si["modelLevel"] = ml
        }
        o["systemInfo"] = si
        return Self.json(o)
    }

    static func json(_ o: [String: Any]) -> String {
        (try? JSONSerialization.data(withJSONObject: o)).map { String(decoding: $0, as: UTF8.self) } ?? "{}"
    }

    // ───────────────────────── GameSessionUI ─────────────────────────

    func gameMakeDisplayLink(target: Any, selector: Selector) -> CADisplayLink? { displayLink(target: target, selector: selector) }

    func gameLayerWant(width: Int, height: Int) {
        layerPixels = CGSize(width: width, height: height)
        layoutLayers()
        session.layerSized(width: width, height: height)
    }

    func gameShowLayer(metal: Bool) {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        metalLayer.isHidden = !metal
        imageLayer.isHidden = metal
        if metal { imageLayer.contents = nil }
        CATransaction.commit()
    }

    func gameCpuFrame(_ image: CGImage) {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        imageLayer.contents = image
        CATransaction.commit()
    }

    func gameFirstFrame() { onFirstFrame?() }

    func gameMenuButton(_ rect: CGRect) {
        guard let c = capsule else { return }
        c.frame = rect
        c.isHidden = rect.width <= 0 || rect.height <= 0
        addSubview(c, positioned: .above, relativeTo: nil)
    }

    func gameEvents(_ events: [[String: Any]]) {
        for ev in events {
            let type = ev["type"] as? String ?? ""
            switch type {
            case "showKeyboard": keyboard.show(ev)
            case "updateKeyboard": keyboard.update(ev)
            case "hideKeyboard": keyboard.hide()
            case "menuStyle": capsule?.dark = ev["style"] as? String == "dark"
            case "deviceOrientation":
                if !(ev["initial"] as? Bool ?? false) { onOrientationChange?(ev["value"] as? String ?? "portrait") }
            case "clipboard":
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(ev["data"] as? String ?? "", forType: .string)
                onHostEvent?(type, ev)
            case "hostCall":
                if let cb = onHostEvent { cb(type, ev) } else {
                    hostReply((ev["id"] as? NSNumber)?.intValue ?? 0, ok: false, result: ["errMsg": "\(ev["api"] ?? ""):fail 宿主没有实现"])
                }
            default:
                onHostEvent?(type, ev)
            }
        }
    }

    // ───────────────────────── 输入 ─────────────────────────

    public override var acceptsFirstResponder: Bool { true }
    public override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    private func point(_ e: NSEvent) -> (Float, Float) {
        let p = convert(e.locationInWindow, from: nil)
        return (Float(p.x), Float(p.y))
    }

    /// NSEvent.timestamp 与 CADisplayLink 同一个钟（开机以来的秒）：引擎要微秒
    private func us(_ e: NSEvent) -> UInt64 { UInt64(max(0, e.timestamp) * 1_000_000) }

    private func mouse(_ e: NSEvent, phase: Int32, button: Int32, touch: Bool) {
        let (x, y) = point(e)
        let t = us(e)
        session.run { h in
            if touch { mr_game_touch(h, phase, 0, x, y, 0, t) }
            mr_game_mouse(h, phase, button, x, y, t)
        }
    }

    public override func mouseDown(with e: NSEvent) {
        window?.makeFirstResponder(self)
        leftDown = true
        mouse(e, phase: 0, button: 0, touch: true)
    }
    public override func mouseDragged(with e: NSEvent) { mouse(e, phase: 1, button: 0, touch: leftDown) }
    public override func mouseUp(with e: NSEvent) {
        mouse(e, phase: 2, button: 0, touch: leftDown)
        leftDown = false
    }
    public override func mouseMoved(with e: NSEvent) { mouse(e, phase: 1, button: 0, touch: false) }
    public override func rightMouseDown(with e: NSEvent) { mouse(e, phase: 0, button: 2, touch: false) }
    public override func rightMouseDragged(with e: NSEvent) { mouse(e, phase: 1, button: 2, touch: false) }
    public override func rightMouseUp(with e: NSEvent) { mouse(e, phase: 2, button: 2, touch: false) }
    public override func otherMouseDown(with e: NSEvent) { mouse(e, phase: 0, button: 1, touch: false) }
    public override func otherMouseDragged(with e: NSEvent) { mouse(e, phase: 1, button: 1, touch: false) }
    public override func otherMouseUp(with e: NSEvent) { mouse(e, phase: 2, button: 1, touch: false) }

    /// 滚轮 / 触控板：DOM 的 deltaY 向下为正（内容往上走），与 NSEvent 的 scrollingDeltaY 相反
    public override func scrollWheel(with e: NSEvent) {
        let (x, y) = point(e)
        let (dx, dy) = (Float(-e.scrollingDeltaX), Float(-e.scrollingDeltaY))
        guard dx != 0 || dy != 0 else { return }
        let t = us(e)
        session.run { mr_game_wheel($0, dx, dy, 0, x, y, t) }
    }

    public override func keyDown(with e: NSEvent) { key(e, down: true) }
    public override func keyUp(with e: NSEvent) { key(e, down: false) }

    /// 修饰键单独按下 / 松开（Shift、Control、Option、Command）
    public override func flagsChanged(with e: NSEvent) {
        guard let (k, code, flag) = GameKeys.modifier(e.keyCode) else { return }
        let down = e.modifierFlags.contains(flag)
        let t = us(e)
        session.run { mr_game_key($0, down ? 1 : 0, k, code, t) }
    }

    private func key(_ e: NSEvent, down: Bool) {
        // ⌘ 组合键（⌘Q / ⌘W …）留给菜单
        if e.modifierFlags.contains(.command) { return down ? super.keyDown(with: e) : super.keyUp(with: e) }
        let (k, code) = GameKeys.name(e)
        let t = us(e)
        session.run { mr_game_key($0, down ? 1 : 0, k, code, t) }
    }

    // ─────────────────────── 生命周期 ───────────────────────

    @objc private func onForeground() { if !closed { session.foreground() } }

    @objc private func onBackground() { if !closed { session.background() } }
}

/// 小游戏的静态能力：包种类、方向、预编译
public enum MiniGame {
    /// 包种类：0 = 认不出，1 = 小程序（app.json），2 = 小游戏（game.json）
    public static func packageKind(_ dir: String) -> Int { Int(mr_package_kind(dir)) }

    /// game.json 的 deviceOrientation（缺省 portrait）
    public static func orientation(_ dir: String) -> String {
        guard let d = FileManager.default.contents(atPath: (dir as NSString).appendingPathComponent("game.json")),
              let j = (try? JSONSerialization.jsonObject(with: d)) as? [String: Any] else { return "portrait" }
        return j["deviceOrientation"] as? String ?? "portrait"
    }

    /// 存档 / 字节码缓存的默认目录（Application Support/<bundle id>/game-data）
    public static var defaultDataDir: String {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? URL(fileURLWithPath: NSTemporaryDirectory())
        return base.appendingPathComponent(Bundle.main.bundleIdentifier ?? "mini-render").appendingPathComponent("game-data").path
    }

    /// 下载 / 解包完就在后台把大脚本编成字节码（立刻返回；马上打开也没关系，实例会等正在编的那段）
    public static func precompile(dir: String, dataDir: String? = nil) {
        let data = dataDir ?? defaultDataDir
        try? FileManager.default.createDirectory(atPath: data, withIntermediateDirectories: true)
        let t = Thread { _ = mr_game_precompile(dir, data) }
        t.name = "mini-game-precompile"
        t.qualityOfService = .userInitiated
        t.start()
    }
}
