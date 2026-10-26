import AppKit
import ImageIO
import QuartzCore

#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

/// 小程序视图（macOS / AppKit）—— 集成方**只需要用这一个类**，API 与 iOS 的 `MiniProgramView` 同一个形状。
///
/// ```swift
/// let mini = MiniProgramView(frame: window.contentView!.bounds)
/// mini.autoresizingMask = [.width, .height]
/// window.contentView!.addSubview(mini)
/// mini.open(appDir: unpackedDir)
/// ```
///
/// ## 它替你做了什么
/// - 一条**专用引擎线程**，显示同步（`NSView.displayLink`，跟着窗口所在屏幕的刷新率）挂在上面；
/// - 后端默认 **auto**：画在自己的 `CAMetalLayer` 上（GPU 合成），没选上 / 运行中丢设备回退 CPU（CGImage）；
/// - 鼠标 → 引擎的**指针三段**（按下 / 拖动 / 抬起，带事件时间）；触控板 / 滚轮 → `mr_app_wheel`；
/// - 输入框聚焦时接键盘与系统输入法（[MiniKeyboard]）；`wx.setClipboardData` 写系统剪贴板；
/// - 窗口最小化 / App 隐藏 → `App.onHide` 并停住出帧；回来 → `App.onShow`；
/// - 按 `bounds` 与 `backingScaleFactor` 算逻辑尺寸和 dpr，拖窗口大小、换屏（Retina ↔ 外接屏）都交给引擎重排。
///
/// ## 顶部
/// PC 微信的小程序窗口：红绿灯压在小程序自己的导航栏那一条上。窗口用 `.fullSizeContentView` + 透明标题栏时，
/// 把标题栏高度给 [topInset]：引擎把它当状态栏（导航栏底色铺到最顶上，标题与胶囊在红绿灯这一行下面）。
public final class MiniProgramView: NSView, EngineSessionUI {
    public enum RenderBackend: Int32 {
        case auto = 0
        case cpu = 1
        case gpu = 2
    }

    /// 引擎在 custom 导航栏的页面上画右上角胶囊（`open` 之前设）
    public var menuButton = true
    /// 导航栏由引擎画（默认；`open` 之前设）
    public var navigationBar = true
    /// 页面转场动画（默认开；`open` 之前设）
    public var routeAnimation = true
    /// 顶部让给窗口标题栏 / 红绿灯的高度（pt，`open` 之前设）：引擎当状态栏高度用
    public var topInset: CGFloat = 0
    /// 启动参数（`open` 之前设）：`App.onLaunch(options)` 与 `wx.getLaunchOptionsSync()` 里读到
    public var launchOptions: [String: Any]?
    /// 渲染后端偏好（`open` 之前设）
    public var backend: RenderBackend = .auto
    /// 分栏（微信 PC / 阔折叠的 frameset，`open` 之前设）：窗口宽到 [framesetMinWidth] 自动分左右两栏，
    /// 栈上两页以上时上一页在左栏、当前页在右栏；窄回去退回单栏。false = 永不分栏
    public var frameset = true
    public var framesetMinWidth: CGFloat = 600
    public var activeBackend: RenderBackend { RenderBackend(rawValue: session.activeBackend) ?? .cpu }
    public var backendReason: String { session.backendReason }
    /// 宿主实现的 `wx.*`（`open` 之前登记；主线程上调）
    public var hostApis: [String: MiniHostApi] = [:]
    /// 胶囊「•••」：查看关于。nil = 弹一个 NSAlert
    public var onAbout: ((MiniProgramAbout) -> Void)?
    /// 胶囊「⊙」：返回宿主。nil = 关掉视图所在的窗口
    public var onClose: (() -> Void)?
    /// 引擎发来的每条事件（page / navigationBar / hostCall …）
    public var onEvent: (([String: Any]) -> Void)?
    public private(set) var about: MiniProgramAbout?

    private let session = EngineSession()
    private let imageLayer = CALayer()
    /// 分栏时的另一栏（引擎给的整栏图）与两栏中间的分隔线
    private let sideLayer = CALayer()
    private let dividerLayer = CALayer()
    /// 分栏状态（`frameset` 事件）：活页面那一栏的左边 x、一栏宽（pt）
    private var split = (on: false, liveX: CGFloat(0), pane: CGFloat(0))
    /// 另一栏显示的是一页（能点），不是占位
    private var sidePage = false
    /// 右栏是占位时画在它上面的窗口胶囊（一栏内坐标）：点它与点引擎里的胶囊一样
    private var sideCapsule: CGRect?
    /// 这一次按下落在占位栏上：整串不派给引擎
    private var pointerInSide = false
    private var metalLayer: CAMetalLayer?
    private var appDir: URL?
    private var startRoute: String?
    private var created = false
    private var closed = false
    private var logicalSize = (UInt32(0), UInt32(0))
    private var keyboard: MiniKeyboard!

    public override init(frame: NSRect) {
        super.init(frame: frame)
        setup()
    }

    public required init?(coder: NSCoder) {
        super.init(coder: coder)
        setup()
    }

    private func setup() {
        wantsLayer = true
        layer?.backgroundColor = NSColor.white.cgColor
        session.ui = self
        imageLayer.magnificationFilter = .nearest
        imageLayer.anchorPoint = .zero
        imageLayer.actions = Self.noAnimations
        layer?.addSublayer(imageLayer)
        for l in [sideLayer, dividerLayer] {
            l.anchorPoint = .zero
            l.actions = Self.noAnimations
            l.isHidden = true
            layer?.addSublayer(l)
        }
        sideLayer.magnificationFilter = .nearest
        dividerLayer.backgroundColor = NSColor(white: 0.88, alpha: 1).cgColor
        keyboard = MiniKeyboard(host: self, sendText: { [weak self] in self?.sendText($0) }, sendKey: { [weak self] in self?.sendKey($0) })
        let nc = NotificationCenter.default
        nc.addObserver(self, selector: #selector(onBackground), name: NSApplication.didHideNotification, object: nil)
        nc.addObserver(self, selector: #selector(onForeground), name: NSApplication.didUnhideNotification, object: nil)
    }

    private static let noAnimations: [String: CAAction] = [
        "contents": NSNull(), "bounds": NSNull(), "position": NSNull(), "hidden": NSNull(),
        "frame": NSNull(), "contentsScale": NSNull(),
    ]

    public override var isFlipped: Bool { true }

    deinit {
        NotificationCenter.default.removeObserver(self)
        close()
    }

    /// 关掉小程序：同步销毁引擎（返回时引擎已经放手 Metal 层、线程已停）。关过之后不能再 `open`，换一个新视图。可重复调。
    public func close() {
        if closed { return }
        closed = true
        keyboard?.onBlur()
        session.dispose()
    }

    /// 打开一个解包后的小程序目录，可指定起始页（深链）。一个视图只开一个小程序
    public func open(appDir: URL, route: String? = nil) {
        self.appDir = appDir
        startRoute = route
        createEngineIfPossible()
    }

    public func navigate(_ url: String) {
        session.run { _ = mr_app_navigate($0, url) }
    }

    /// 返回上一页。**返回 false 表示页面栈只剩一页**
    @discardableResult
    public func goBack() -> Bool {
        if closed { return false }
        return session.call(false) { mr_app_back($0) != 0 }
    }

    /// 引擎当前这一帧（截图 / 自动化验收）
    public func snapshot(_ done: @escaping (NSImage?) -> Void) {
        session.snapshot { img in done(img.map { NSImage(cgImage: $0, size: .zero) }) }
    }

    public func sendText(_ text: String) {
        session.run { mr_app_text_input($0, text) }
    }

    /// 功能键名见 mini_render.h 的 `mr_app_key`
    public func sendKey(_ name: String) {
        session.run { mr_app_key($0, name) }
    }

    // ───────────────────────── 尺寸与窗口 ─────────────────────────

    private var dpr: Float { Float(window?.backingScaleFactor ?? 2) }

    public override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        let nc = NotificationCenter.default
        nc.removeObserver(self, name: NSWindow.didMiniaturizeNotification, object: nil)
        nc.removeObserver(self, name: NSWindow.didDeminiaturizeNotification, object: nil)
        if let w = window {
            nc.addObserver(self, selector: #selector(onBackground), name: NSWindow.didMiniaturizeNotification, object: w)
            nc.addObserver(self, selector: #selector(onForeground), name: NSWindow.didDeminiaturizeNotification, object: w)
        }
        createEngineIfPossible()
    }

    public override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        guard created else { return createEngineIfPossible() }
        relayout()
    }

    /// 换屏（Retina ↔ 外接 1x 屏）：dpr 变了
    public override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        guard created else { return }
        logicalSize = (0, 0)
        relayout()
    }

    private func relayout() {
        let size = (UInt32(bounds.width), UInt32(bounds.height))
        if size != logicalSize, size.0 > 0, size.1 > 0 {
            logicalSize = size
            session.resize(width: size.0, height: size.1, dpr: dpr)
        }
    }

    private func createEngineIfPossible() {
        guard !created, !closed, let dir = appDir, window != nil, bounds.width > 0, bounds.height > 0 else { return }
        created = true
        let data = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent(Bundle.main.bundleIdentifier ?? "mini-render", isDirectory: true)
            .appendingPathComponent("mini-render", isDirectory: true)
        try? FileManager.default.createDirectory(at: data, withIntermediateDirectories: true)
        let os = ProcessInfo.processInfo.operatingSystemVersion
        var options: [String: Any] = [
            "systemInfo": [
                "platform": "mac",
                "model": "Mac",
                "system": "macOS \(os.majorVersion).\(os.minorVersion)",
            ],
            "safeArea": ["top": topInset, "right": 0, "bottom": 0, "left": 0],
            "menuButton": menuButton,
            "navigationBar": navigationBar,
            "routeAnimation": routeAnimation,
            "frameset": frameset ? "auto" : "off",
            "framesetMinWidth": framesetMinWidth,
            "hostApis": Array(hostApis.keys),
        ]
        if let lo = launchOptions, JSONSerialization.isValidJSONObject(lo) { options["launchOptions"] = lo }
        let json = (try? JSONSerialization.data(withJSONObject: options)).flatMap { String(data: $0, encoding: .utf8) } ?? "{}"
        logicalSize = (UInt32(bounds.width), UInt32(bounds.height))
        session.create(
            appDir: dir.path, dataDir: data.path, route: startRoute,
            width: logicalSize.0, height: logicalSize.1, dpr: dpr, options: json, backend: backend.rawValue,
            device: .init(totalMem: ProcessInfo.processInfo.physicalMemory, osMajor: UInt32(os.majorVersion)))
    }

    // ───────────────────────── 图层（EngineSessionUI，主线程） ─────────────────────────

    func sessionMakeDisplayLink(target: Any, selector: Selector) -> CADisplayLink? {
        displayLink(target: target, selector: selector)
    }

    func sessionPixelSize(width: Int, height: Int, metal: Bool) {
        guard width > 0, height > 0 else { return }
        let scale = CGFloat(dpr)
        // 一个引擎像素对一个屏幕像素：层的尺寸 = 引擎像素 / scale，摆在左上角（视图是 flipped 的）
        let frame = CGRect(x: split.liveX, y: 0, width: CGFloat(width) / scale, height: CGFloat(height) / scale)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        imageLayer.frame = frame
        imageLayer.contentsScale = scale
        if let m = metalLayer {
            m.frame = frame
            m.contentsScale = scale
            m.drawableSize = CGSize(width: width, height: height)
        }
        CATransaction.commit()
        guard metal, metalLayer == nil else { return }
        let m = CAMetalLayer()
        m.actions = Self.noAnimations
        m.anchorPoint = .zero
        m.frame = frame
        m.contentsScale = scale
        m.drawableSize = CGSize(width: width, height: height)
        m.isOpaque = true
        m.framebufferOnly = true
        m.isHidden = true
        layer?.addSublayer(m)
        metalLayer = m
        session.attachMetal(m)
    }

    func sessionReveal() {
        metalLayer?.isHidden = false
        imageLayer.contents = nil
        imageLayer.isHidden = true
    }

    /// `MINI_SPLIT_TRACE=<目录>`：每次交帧后把两栏图层此刻的内容按摆放位置拼一张（按 pt 大小）存下来，另外每个 vsync 往 layers.txt 记一行图层的真实状态，
    /// 查「点一下闪两次」这类问题用 —— 屏上每一个中间状态都留得下来，不靠录屏。Metal 层的内容截不到（只记它藏没藏）
    private static let splitTrace = ProcessInfo.processInfo.environment["MINI_SPLIT_TRACE"]
    private var traceN = 0

    /// 每个 vsync 记一行屏上图层的真实状态（presentation layer：Core Animation 此刻显示的那份）
    private var traceLink: CADisplayLink?
    private var traceLines: [String] = []
    @objc private func traceTick(_ l: CADisplayLink) {
        func d(_ x: CALayer?) -> String {
            guard let x else { return "-" }
            let p = x.presentation() ?? x
            let c = p.contents.map { String(UInt(bitPattern: Unmanaged.passUnretained($0 as AnyObject).toOpaque().hashValue) % 100000) } ?? "nil"
            return String(format: "x%.0f w%.0f %@ c%@ o%.2f", p.frame.minX, p.frame.width, p.isHidden ? "H" : "V", c, p.opacity)
        }
        traceLines.append(String(format: "%.3f img[%@] side[%@] metal[%@]", l.timestamp, d(imageLayer), d(sideLayer), d(metalLayer)))
        if traceLines.count % 120 == 0, let dir = Self.splitTrace {
            try? traceLines.joined(separator: "\n").write(toFile: dir + "/layers.txt", atomically: true, encoding: .utf8)
        }
    }

    func sessionCommitted() {
        if Self.splitTrace != nil, traceLink == nil {
            traceLink = displayLink(target: self, selector: #selector(traceTick(_:)))
            traceLink?.add(to: .main, forMode: .common)
        }
        guard let dir = Self.splitTrace, split.on else { return }
        traceN += 1
        let s: CGFloat = 1
        let w = Int(bounds.width * s), h = Int(bounds.height * s)
        guard w > 0, h > 0, let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                                                 space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return }
        ctx.setFillColor(NSColor.magenta.cgColor) // 两层都没盖到的地方：洋红（闪白 / 露底一眼看得出）
        ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
        for l in [sideLayer, imageLayer] where !l.isHidden {
            guard let c = l.contents else { continue }
            let f = l.frame
            // 视图是 flipped 的，CGContext 原点在左下
            ctx.draw(c as! CGImage, in: CGRect(x: f.minX * s, y: CGFloat(h) - f.maxY * s, width: f.width * s, height: f.height * s))
        }
        guard let img = ctx.makeImage() else { return }
        let url = URL(fileURLWithPath: dir).appendingPathComponent(String(format: "%04d-live%.0f.png", traceN, split.liveX))
        try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        if let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil) {
            CGImageDestinationAddImage(dest, img, nil)
            CGImageDestinationFinalize(dest)
        }
    }

    func sessionHoldGpu() {
        guard let m = metalLayer else { return }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        imageLayer.isHidden = false
        m.isHidden = true
        CATransaction.commit()
    }

    func sessionLeftGpu() {
        metalLayer?.removeFromSuperlayer()
        metalLayer = nil
        imageLayer.isHidden = false
    }

    func sessionCpuFrame(_ image: CGImage) {
        imageLayer.contents = image
    }

    /// 分栏状态：是否分栏、活页面那一栏的左边 x、一栏宽（pt）
    public var splitLayout: (on: Bool, liveX: CGFloat, pane: CGFloat) { split }

    /// 分栏时另一栏的图（截图验收用；不分栏为 nil）
    public func snapshotSide() -> NSImage? {
        guard split.on, let c = sideLayer.contents else { return nil }
        return NSImage(cgImage: c as! CGImage, size: .zero)
    }

    func sessionSideFrame(_ image: CGImage?) {
        let scale = CGFloat(dpr)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        sideLayer.contents = image
        sideLayer.contentsScale = scale
        if let image {
            // 拖窗口只改了高时不会再来 frameset 事件：尺寸跟着图走
            sideLayer.frame.size = CGSize(width: CGFloat(image.width) / scale, height: CGFloat(image.height) / scale)
            dividerLayer.frame.size.height = bounds.height
        }
        sideLayer.isHidden = image == nil || !split.on
        CATransaction.commit()
    }

    /// `frameset` 事件：活页面放 livePane 那一栏，另一栏放引擎给的图，中间一条分隔线
    private func applyFrameset(_ ev: [String: Any]) {
        let on = ev["on"] as? Bool ?? false
        let pane = CGFloat((ev["paneWidth"] as? NSNumber)?.doubleValue ?? 0)
        let liveRight = on && (ev["livePane"] as? String) == "right"
        split = (on, liveRight ? pane : 0, pane)
        sidePage = on && (ev["sidePage"] as? Bool ?? false)
        sideCapsule = (ev["sideCapsule"] as? [String: Any]).map { r in
            let n = { (k: String) in CGFloat((r[k] as? NSNumber)?.doubleValue ?? 0) }
            return CGRect(x: n("x"), y: n("y"), width: n("width"), height: n("height"))
        }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for l in [imageLayer, metalLayer].compactMap({ $0 }) { l.frame.origin.x = split.liveX }
        sideLayer.frame = CGRect(x: liveRight ? 0 : pane, y: 0, width: pane, height: bounds.height)
        sideLayer.isHidden = !on || sideLayer.contents == nil
        dividerLayer.frame = CGRect(x: pane, y: 0, width: 1 / max(1, CGFloat(dpr)), height: bounds.height)
        dividerLayer.isHidden = !on
        CATransaction.commit()
    }

    /// 视图里的 x 落在另一栏
    private func inSidePane(_ x: CGFloat) -> Bool {
        split.on && (x < split.liveX || x >= split.liveX + split.pane)
    }

    func sessionEvents(_ events: [[String: Any]]) {
        events.forEach(dispatchEvent)
    }

    // ───────────────────────── 事件 ─────────────────────────

    private func dispatchEvent(_ ev: [String: Any]) {
        onEvent?(ev)
        switch ev["type"] as? String {
        case "frameset": applyFrameset(ev)
        case "app":
            if let a = ev["about"] as? [String: Any] { about = MiniProgramAbout(json: a) }
        case "inputFocus": keyboard.onFocus(ev, originX: split.liveX)
        case "inputBlur": keyboard.onBlur()
        case "inputValue": keyboard.onValue(ev)
        case "menuButton":
            switch MiniMenuButtonAction(rawValue: ev["action"] as? String ?? "") {
            case .menu: showAbout()
            case .close: if let cb = onClose { cb() } else { window?.performClose(nil) }
            case nil: break
            }
        case "hostCall":
            onHostCall(MiniHostCall(json: ev))
        case "clipboard":
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(ev["data"] as? String ?? "", forType: .string)
        default: break
        }
    }

    private func onHostCall(_ call: MiniHostCall) {
        let id = UInt32(call.id)
        let reply = MiniHostReply { [session] ok, res in
            let json = (try? JSONSerialization.data(withJSONObject: res)).flatMap { String(data: $0, encoding: .utf8) } ?? "{}"
            session.run { _ = mr_app_host_reply($0, id, ok ? 1 : 0, json) }
        }
        guard let api = hostApis[call.api] else { return reply.fail("\(call.api):fail 宿主没有实现") }
        api(call, reply)
    }

    private func showAbout() {
        let a = about ?? MiniProgramAbout(json: [:])
        if let cb = onAbout { cb(a); return }
        let alert = NSAlert()
        alert.messageText = a.dialogTitle
        alert.informativeText = a.dialogMessage
        if let w = window { alert.beginSheetModal(for: w) } else { alert.runModal() }
    }

    // ───────────────────────── 输入 ─────────────────────────

    public override var acceptsFirstResponder: Bool { true }
    public override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    /// 引擎坐标：分栏时相对活页面那一栏
    private func point(_ e: NSEvent) -> (Float, Float) {
        let p = convert(e.locationInWindow, from: nil)
        return (Float(p.x - split.liveX), Float(p.y))
    }

    public override func mouseDown(with e: NSEvent) {
        let vx = convert(e.locationInWindow, from: nil).x
        pointerInSide = false
        if inSidePane(vx) {
            // 另一栏是占位：不派。是一页：先让引擎把它换成活页面，这一串的坐标改按那一栏算
            // （引擎线程上按顺序执行，focus 一定在 pointer_down 之前；图层等 frameset 事件来了再挪）
            guard sidePage else {
                pointerInSide = true
                // 占位栏上的胶囊：左半「•••」、右半「⊙」（与引擎的 menuButton 事件同一套处理）
                let origin = vx < split.pane ? 0 : split.pane
                let p = CGPoint(x: vx - origin, y: convert(e.locationInWindow, from: nil).y)
                if let c = sideCapsule, c.contains(p) {
                    dispatchEvent(["type": "menuButton", "action": p.x < c.midX ? "menu" : "close"])
                }
                return
            }
            let left = vx < split.pane
            split.liveX = left ? 0 : split.pane
            let (x, y) = point(e)
            session.run {
                _ = mr_app_frameset_focus($0, left ? 0 : 1)
                mr_app_pointer_down($0, x, y)
            }
            return
        }
        let (x, y) = point(e)
        session.run { mr_app_pointer_down($0, x, y) }
    }

    public override func mouseDragged(with e: NSEvent) {
        if pointerInSide { return }
        let (x, y) = point(e)
        // NSEvent.timestamp 与 CADisplayLink、CACurrentMediaTime 同一个钟：引擎按它重采样拖动
        let ms = e.timestamp * 1000
        session.run { mr_app_pointer_move_at($0, x, y, ms) }
    }

    public override func mouseUp(with e: NSEvent) {
        if pointerInSide { pointerInSide = false; return }
        let (x, y) = point(e)
        session.run { mr_app_pointer_up($0, x, y) }
    }

    /// 触控板双指 / 鼠标滚轮。`scrollingDeltaY > 0` 是内容往下走（滚动位置变小），引擎要的是位置的增量
    public override func scrollWheel(with e: NSEvent) {
        let dy = Float(-e.scrollingDeltaY)
        let precise: Int32 = e.hasPreciseScrollingDeltas ? 1 : 0
        guard dy != 0 else { return }
        if inSidePane(convert(e.locationInWindow, from: nil).x) {
            session.run { mr_app_side_wheel($0, dy, precise) }
        } else {
            session.run { mr_app_wheel($0, dy, precise) }
        }
    }

    /// 没有输入框聚焦时：⌘[ / Esc 返回上一页（PC 微信小程序窗口没有返回键，导航栏左上角的返回箭头之外再给一个快捷键）
    public override func keyDown(with e: NSEvent) {
        if e.keyCode == 53 || (e.modifierFlags.contains(.command) && e.charactersIgnoringModifiers == "[") {
            goBack()
            return
        }
        super.keyDown(with: e)
    }

    // ─────────────────────── 生命周期 ───────────────────────

    @objc private func onForeground() { session.foreground() }

    @objc private func onBackground() { session.background() }
}
