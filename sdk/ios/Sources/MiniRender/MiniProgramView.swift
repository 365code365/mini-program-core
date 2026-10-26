import QuartzCore
import UIKit

// C ABI（XCFramework 里的 MiniRenderFFI）。用 CocoaPods + 桥接头时这个模块不存在，
// 此时符号已经通过桥接头可见，所以要条件导入。
#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

/// 小程序视图 —— 集成方**只需要用这一个类**。
///
/// ```swift
/// let view = MiniProgramView()
/// view.frame = self.view.bounds
/// self.view.addSubview(view)
/// view.open(appDir: unpackedDir)          // 解包后的小程序目录
///
/// // 侧滑/返回按钮交给小程序：
/// if !view.goBack() { navigationController?.popViewController(animated: true) }
/// ```
///
/// ## 它替你做了什么
/// - 起一条**专用引擎线程**（引擎不是线程安全的，所有调用都排到这条线程上），`CADisplayLink` 挂在上面，
///   vsync 时刻交给引擎（动画按它取样）；ProMotion 上要 120Hz（iPhone 还要 Info.plist 的
///   `CADisableMinimumFrameDurationOnPhone = YES`）；
/// - 后端默认 **auto**：画面画在自己的一块 `CAMetalLayer` 上，GPU（wgpu 的 Metal）合成、滚动帧只合成不重画；
///   GPU 没选上 / 运行中丢设备一律回退 CPU（取像素做成 CGImage），回退有日志，见 [backend] / [activeBackend]；
/// - 只在引擎还有活时出帧，静止时暂停 DisplayLink（不耗电）；进后台同步停住（Metal 在后台提交会被系统杀进程）；
/// - `touchesBegan/Moved/Ended/Cancelled` → 引擎的**指针三段**（含 `coalescedTouches` 与各自的时间戳：
///   引擎按事件时间重采样页面拖动，慢拖不一顿一顿）；
/// - 按 `bounds` 与屏幕 scale 算逻辑尺寸和 dpr，转屏 / 分屏时交给引擎重排；
/// - 前后台与内存告警转发；`wx.setClipboardData` 写系统剪贴板；[hostApis] 接宿主实现的 `wx.*`。
///
/// ## 为什么触摸要送三段而不是"点击"
/// 引擎内部有触摸状态机，`touchstart/touchmove/touchend/longpress/tap` 都由它产出。
/// 宿主自己合成"点击"会绕过状态机 —— 那类问题在自动化测试里全绿、真机上却点不动。
///
/// ## 右上角胶囊
/// `navigationStyle: custom` 的页面由引擎画胶囊（位置 = `wx.getMenuButtonBoundingClientRect()`，
/// 所以视图要铺到状态栏下面，安全区会自动交给引擎）。两个口子留给宿主：
/// ```swift
/// view.onAbout = { about in /* 「•••」查看关于：推自己的关于页 */ }
/// view.onClose = { /* 「⊙」返回宿主：pop / dismiss 自己的页面 */ }
/// ```
/// 不设时的默认：`onAbout` 弹一个展示小程序信息的 `UIAlertController`，`onClose` 把视图所在的
/// 控制器 pop（在导航栈里）或 dismiss。普通页面的导航栏默认也由引擎画（[navigationBar]）。
///
/// ## 图层
/// 画面是一个子图层（GPU：CAMetalLayer；CPU：内容是 CGImage 的 CALayer），按引擎像素 1:1 摆在左上角
/// （视图宽 390.5pt 时引擎是 390pt，不拉伸）。第一帧交出来之前 Metal 层是藏着的，露出视图的背景色。
public final class MiniProgramView: UIView, EngineSessionUI {

    /// 渲染后端偏好
    public enum RenderBackend: Int32 {
        /// 能用 GPU 就用（启动自检逐字节一致才算能用），否则 CPU
        case auto = 0
        /// 强制软件光栅化
        case cpu = 1
        /// 要 GPU（照样过启动自检；不行时回退 CPU）
        case gpu = 2
    }

    /// 引擎在 custom 导航栏的页面上画右上角胶囊（`open` 之前设）
    public var menuButton = true
    /// 导航栏由引擎画（默认，与 Android 视图、Flutter 插件一致；`open` 之前设）：普通页面顶上是引擎画的导航栏
    /// （标题、返回箭头、胶囊、`wx.setNavigationBar*`）。关掉的话导航栏要宿主自己画
    public var navigationBar = true
    /// 页面转场动画（默认开；`open` 之前设）：navigateTo 从右边滑入、返回向右滑出，300ms
    public var routeAnimation = true
    /// 启动参数（`open` 之前设）：外部 App / H5 / 别的小程序打开时给，小程序在 `App.onLaunch(options)` 与
    /// `wx.getLaunchOptionsSync()` 里读到。形状与微信一致：
    /// `["path": "pages/x/x", "query": ["id": "1"], "scene": 1065, "referrerInfo": ["appId": "wx…"]]`。不给就是默认（scene 1001）
    public var launchOptions: [String: Any]?
    /// 渲染后端偏好（`open` 之前设）。调试时 Xcode 的 scheme 里设环境变量 `MINI_BACKEND=cpu` 可以强制
    public var backend: RenderBackend = .auto
    /// 实际后端（挂上 Metal 层之后才定）
    public var activeBackend: RenderBackend { RenderBackend(rawValue: session.activeBackend) ?? .cpu }
    /// 为什么是这个后端（自检结果 / 回退原因）
    public var backendReason: String { session.backendReason }
    /// 宿主实现的 `wx.*`（`open` 之前登记；登录、支付、定位、扫码……）。登记了名字的 API 调用时交给它（主线程），
    /// 没登记的照旧立刻 `fail`（errMsg 写明「宿主可接管」）
    public var hostApis: [String: MiniHostApi] = [:]
    /// 胶囊「•••」：查看关于。nil = 默认弹框
    public var onAbout: ((MiniProgramAbout) -> Void)?
    /// 胶囊「⊙」：返回宿主。nil = 默认 pop / dismiss 所在的控制器
    public var onClose: (() -> Void)?
    /// 引擎发来的每一条事件（JSON 对象；`page` / `navigationBar` / `inputFocus` / `app` / `menuButton` …）
    public var onEvent: (([String: Any]) -> Void)?
    /// 小程序的关于信息（启动后第一帧到）
    public private(set) var about: MiniProgramAbout?

    private let session = EngineSession()
    private let imageLayer = CALayer()
    private var metalLayer: CAMetalLayer?
    private var pixel = (0, 0)
    private var appDir: URL?
    private var startRoute: String?
    private var created = false
    /// [close] 过：引擎已销毁、线程已停，不再建引擎
    private var closed = false
    /// 软键盘：输入框聚焦弹键盘、字交给引擎、键盘挡住时顶起画面（MiniKeyboard.swift）
    private var keyboard: MiniKeyboard?
    private var logicalSize = (UInt32(0), UInt32(0))

    public override init(frame: CGRect) {
        super.init(frame: frame)
        setup()
    }

    public required init?(coder: NSCoder) {
        super.init(coder: coder)
        setup()
    }

    private func setup() {
        // 第二根手指只给图片预览的双指缩放（mr_app_pointer2），页面仍是单指（见 touchesBegan）
        isMultipleTouchEnabled = true
        backgroundColor = .white
        session.ui = self
        keyboard = MiniKeyboard(
            host: self,
            sendText: { [weak self] t in self?.sendText(t) },
            sendKey: { [weak self] k in self?.sendKey(k) },
            onShift: { [weak self] v in self?.session.run { mr_app_set_keyboard_shift($0, Float(v)) } },
            onHeight: { [weak self] h in
                // wx.onKeyboardHeightChange / bindkeyboardheightchange：键盘挡住视图的那一截
                self?.session.run { _ = mr_app_dispatch_event($0, "keyboardHeightChange", "{\"height\":\(Int(h))}") }
            })
        imageLayer.magnificationFilter = .nearest
        imageLayer.anchorPoint = .zero
        // 子图层改 contents / 尺寸默认带 0.25s 的隐式动画：每一帧都是一次淡入淡出
        imageLayer.actions = Self.noAnimations
        layer.addSublayer(imageLayer)
        NotificationCenter.default.addObserver(
            self, selector: #selector(onMemoryWarning),
            name: UIApplication.didReceiveMemoryWarningNotification, object: nil)
        NotificationCenter.default.addObserver(
            self, selector: #selector(onForeground),
            name: UIApplication.didBecomeActiveNotification, object: nil)
        NotificationCenter.default.addObserver(
            self, selector: #selector(onBackground),
            name: UIApplication.didEnterBackgroundNotification, object: nil)
    }

    private static let noAnimations: [String: CAAction] = [
        "contents": NSNull(), "bounds": NSNull(), "position": NSNull(), "hidden": NSNull(),
        "frame": NSNull(), "contentsScale": NSNull(),
    ]

    deinit {
        NotificationCenter.default.removeObserver(self)
        close()
    }

    /// 关掉小程序：同步销毁引擎（返回时引擎已经放手 Metal 层、线程已停）。与 Android 的 `MiniProgramView.close()` 同义。
    ///
    /// 换小程序时要先关旧的再开新的（引擎的小程序根目录是进程级的，同一时刻只能有一个实例）：
    /// 只靠 `removeFromSuperview` 等 deinit 不行 —— 闭包 / 手势还拿着视图时 deinit 会晚到，新引擎就撞上旧的。
    /// 关过之后这个视图不能再 `open`，换一个新视图。可重复调。
    public func close() {
        if closed { return }
        closed = true
        keyboard?.onBlur()
        // 先卸 Metal 层、销毁引擎，层这时才能放（引擎不持有 CAMetalLayer）
        session.dispose()
    }

    /// 打开一个解包后的小程序目录，可指定起始页（深链）。一个视图只开一个小程序
    public func open(appDir: URL, route: String? = nil) {
        self.appDir = appDir
        self.startRoute = route
        createEngineIfPossible()
    }

    /// 跳到某个页面（`/pages/x/x?id=1`）
    public func navigate(_ url: String) {
        session.run { _ = mr_app_navigate($0, url) }
    }

    /// 返回键 / 侧滑返回。**返回 false 表示页面栈只剩一页**，宿主该关掉小程序。
    /// 同步等引擎线程的结果（返回是低频操作，代价可以忽略）
    @discardableResult
    public func goBack() -> Bool {
        // 关过的视图引擎线程已停：不等那 2 秒的超时
        if closed { return false }
        return session.call(false) { mr_app_back($0) != 0 }
    }

    /// 引擎当前这一帧（截图 / 自动化验收用；画面在 CAMetalLayer 上时 `layer.render(in:)` 截不到）
    public func snapshot(_ done: @escaping (UIImage?) -> Void) {
        let scale = contentScaleFactor
        session.snapshot { img in done(img.map { UIImage(cgImage: $0, scale: scale, orientation: .up) }) }
    }

    public override func layoutSubviews() {
        super.layoutSubviews()
        guard created else { return createEngineIfPossible() }
        // 转屏 / 分屏 / 键盘顶起之外的尺寸变化：交给引擎重排（层的尺寸等引擎回了新的像素尺寸再改）
        let size = (UInt32(bounds.width), UInt32(bounds.height))
        if size != logicalSize, size.0 > 0, size.1 > 0 {
            logicalSize = size
            session.resize(width: size.0, height: size.1, dpr: dpr)
        }
    }

    private var dpr: Float { Float(window?.screen.scale ?? UIScreen.main.scale) }

    private func createEngineIfPossible() {
        guard !created, !closed, let dir = appDir, bounds.width > 0, bounds.height > 0 else { return }
        created = true
        // 数据目录必须给：storage 与图片磁盘缓存都落这里，不给会静默失效
        let data = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("mini-render")
        try? FileManager.default.createDirectory(at: data, withIntermediateDirectories: true)
        // 选项要在 app.js 执行前就位：onLaunch 里读 getSystemInfoSync（状态栏高度、胶囊位置）是常态
        let insets = safeAreaInsets
        var options: [String: Any] = [
            "systemInfo": [
                "platform": "ios",
                "model": UIDevice.current.model,
                "system": "iOS \(UIDevice.current.systemVersion)",
            ],
            "safeArea": ["top": insets.top, "right": insets.right, "bottom": insets.bottom, "left": insets.left],
            "menuButton": menuButton,
            "navigationBar": navigationBar,
            "routeAnimation": routeAnimation,
            // 点了名的 wx.* 才交给宿主（hostCall 事件）；没点名的照旧立刻 fail
            "hostApis": Array(hostApis.keys),
        ]
        if let lo = launchOptions, JSONSerialization.isValidJSONObject(lo) { options["launchOptions"] = lo }
        let json = (try? JSONSerialization.data(withJSONObject: options)).flatMap { String(data: $0, encoding: .utf8) } ?? "{}"
        logicalSize = (UInt32(bounds.width), UInt32(bounds.height))
        let os = ProcessInfo.processInfo.operatingSystemVersion
        session.create(
            appDir: dir.path, dataDir: data.path, route: startRoute,
            width: logicalSize.0, height: logicalSize.1, dpr: dpr, options: json, backend: backend.rawValue,
            device: .init(totalMem: ProcessInfo.processInfo.physicalMemory, lowRam: false, osMajor: UInt32(os.majorVersion)))
    }

    /// 刘海 / 灵动岛 / 底部横条：`statusBarHeight`、胶囊位置与 `env(safe-area-inset-*)` 都按它算
    public override func safeAreaInsetsDidChange() {
        super.safeAreaInsetsDidChange()
        let i = safeAreaInsets
        session.run { _ = mr_app_set_safe_area($0, Float(i.top), Float(i.right), Float(i.bottom), Float(i.left)) }
    }

    // ───────────────────────── 图层（EngineSessionUI，主线程） ─────────────────────────

    func sessionPixelSize(width: Int, height: Int, metal: Bool) {
        guard width > 0, height > 0 else { return }
        pixel = (width, height)
        let scale = CGFloat(dpr)
        // 一个引擎像素对一个屏幕像素：层的尺寸 = 引擎像素 / scale，摆在左上角
        let frame = CGRect(x: 0, y: 0, width: CGFloat(width) / scale, height: CGFloat(height) / scale)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        imageLayer.frame = frame
        imageLayer.contentsScale = scale
        if let m = metalLayer {
            m.frame = frame
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
        // 第一帧交出来之前藏着：没有内容的 Metal 层是黑的（开 GPU 要几百毫秒）
        m.isHidden = true
        layer.addSublayer(m)
        metalLayer = m
        session.attachMetal(m)
    }

    func sessionReveal() {
        metalLayer?.isHidden = false
        // GPU 在画了：CPU 那张图不用了，放掉它占的内存
        imageLayer.contents = nil
        imageLayer.isHidden = true
    }

    func sessionLeftGpu() {
        // 引擎已经放手（leaveGpu 先 detach 再回到这里）
        metalLayer?.removeFromSuperlayer()
        metalLayer = nil
        imageLayer.isHidden = false
    }

    func sessionCpuFrame(_ image: CGImage) {
        imageLayer.contents = image
    }

    func sessionEvents(_ events: [[String: Any]]) {
        events.forEach(dispatchEvent)
    }

    // ───────────────────────── 事件 ─────────────────────────

    private func dispatchEvent(_ ev: [String: Any]) {
        onEvent?(ev)
        switch ev["type"] as? String {
        case "app":
            if let a = ev["about"] as? [String: Any] { about = MiniProgramAbout(json: a) }
        case "inputFocus": keyboard?.onFocus(ev)
        case "inputBlur": keyboard?.onBlur()
        case "inputValue": keyboard?.onValue(ev)
        case "menuButton":
            switch MiniMenuButtonAction(rawValue: ev["action"] as? String ?? "") {
            case .menu: showAbout()
            case .close: requestClose()
            case nil: break
            }
        case "hostCall":
            onHostCall(MiniHostCall(json: ev))
        case "clipboard":
            // wx.setClipboardData、输入框里的 copy / cut：引擎自己存一份并弹 Toast，系统剪贴板由宿主写
            UIPasteboard.general.string = ev["data"] as? String ?? ""
        case "videoFullscreen":
            // 全屏期间听设备朝向：手机横过来画面跟着转（竖着进全屏，见引擎 video/fullscreen.rs）
            orientation.enable(ev["fullScreen"] as? Bool ?? false)
        default: break
        }
    }

    private func onHostCall(_ call: MiniHostCall) {
        let id = UInt32(call.id)
        let reply = MiniHostReply { [session] ok, res in
            let json = (try? JSONSerialization.data(withJSONObject: res)).flatMap { String(data: $0, encoding: .utf8) } ?? "{}"
            session.run { _ = mr_app_host_reply($0, id, ok ? 1 : 0, json) }
        }
        guard let api = hostApis[call.api] else {
            return reply.fail("\(call.api):fail 宿主没有实现")
        }
        api(call, reply)
    }

    /// 「•••」：宿主给了 onAbout 就交给它，否则弹框展示（演示用）
    private func showAbout() {
        let a = about ?? MiniProgramAbout(json: [:])
        if let cb = onAbout { cb(a); return }
        let alert = UIAlertController(title: a.dialogTitle, message: a.dialogMessage, preferredStyle: .alert)
        alert.addAction(UIAlertAction(title: "确定", style: .default))
        hostController?.present(alert, animated: true)
    }

    /// 「⊙」：宿主给了 onClose 就交给它，否则 pop（在导航栈里）或 dismiss 所在的控制器
    private func requestClose() {
        if let cb = onClose { cb(); return }
        guard let vc = hostController else { return }
        if let nav = vc.navigationController, nav.viewControllers.count > 1, nav.topViewController === vc {
            nav.popViewController(animated: true)
        } else {
            vc.dismiss(animated: true)
        }
    }

    /// 视图所在的控制器（顺着响应链找）
    private var hostController: UIViewController? {
        var r: UIResponder? = self
        while let next = r?.next {
            if let vc = next as? UIViewController { return vc }
            r = next
        }
        return nil
    }

    // ───────────────────────── 输入 ─────────────────────────

    /// 全屏视频跟着手机转
    private lazy var orientation = FullscreenOrientation { [session] deg in
        session.run { mr_app_set_device_orientation($0, deg) }
    }

    /// 第一根手指（页面的触摸状态机是单指的）与第二根（只给图片预览的双指缩放）
    private weak var primaryTouch: UITouch?
    private weak var secondaryTouch: UITouch?

    public override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        for t in touches {
            let p = t.location(in: self)
            if primaryTouch == nil {
                primaryTouch = t
                session.run { mr_app_pointer_down($0, Float(p.x), Float(p.y)) }
            } else if secondaryTouch == nil {
                secondaryTouch = t
                session.run { mr_app_pointer2($0, 0, Float(p.x), Float(p.y)) }
            }
        }
    }

    public override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        if let s = secondaryTouch, touches.contains(s) {
            let p = s.location(in: self)
            session.run { mr_app_pointer2($0, 1, Float(p.x), Float(p.y)) }
        }
        guard let t = primaryTouch, touches.contains(t) else { return }
        // 120Hz 屏上一帧可能积压多个点，全都要送（抽稀会影响滑动手感与方向判定）；带上各自的时间
        // （UITouch.timestamp 与 CADisplayLink、CACurrentMediaTime 同一个钟）：引擎按它重采样页面拖动
        let points = (event?.coalescedTouches(for: t) ?? [t]).map { ($0.location(in: self), $0.timestamp * 1000) }
        session.run { h in
            for (p, ms) in points { mr_app_pointer_move_at(h, Float(p.x), Float(p.y), ms) }
        }
    }

    public override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        if let s = secondaryTouch, touches.contains(s) {
            let p = s.location(in: self)
            secondaryTouch = nil
            session.run { mr_app_pointer2($0, 2, Float(p.x), Float(p.y)) }
        }
        guard let t = primaryTouch, touches.contains(t) else { return }
        if let s = secondaryTouch {
            // 第一根先抬、第二根还按着：报第二根抬起，之后把它当第一根接着送（不报第一根抬起：
            // 否则页面 / 预览当成一次点击）。与 Android MiniTouch.kt 同一条规则
            let p = s.location(in: self)
            session.run { mr_app_pointer2($0, 2, Float(p.x), Float(p.y)) }
            primaryTouch = s
            secondaryTouch = nil
            return
        }
        let p = t.location(in: self)
        primaryTouch = nil
        session.run { mr_app_pointer_up($0, Float(p.x), Float(p.y)) }
    }

    public override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        let two = secondaryTouch != nil
        primaryTouch = nil
        secondaryTouch = nil
        session.run { h in
            if two { mr_app_pointer2(h, 3, 0, 0) }
            mr_app_pointer_cancel(h)
        }
    }

    /// 键盘提交的文字（自己接 `UIKeyInput` 时把字符转过来即可）
    public func sendText(_ text: String) {
        session.run { mr_app_text_input($0, text) }
    }

    /// 功能键：enter / newline / backspace / delete / left / right / up / down / home / end、
    /// shift+… 扩展选区、word-left / word-right、doc-start / doc-end、delete-word-back、
    /// selectall / copy / cut / undo / redo / escape。回车也可以 sendText("\n")（textarea 换行）
    public func sendKey(_ name: String) {
        session.run { mr_app_key($0, name) }
    }

    // ─────────────────────── 生命周期 ───────────────────────

    @objc private func onForeground() {
        session.foreground()
    }

    /// 进后台：派发 `App.onHide` 并**停掉 DisplayLink**（继续出帧纯属耗电，Metal 在后台提交还会被杀进程）
    @objc private func onBackground() {
        session.background()
    }

    @objc private func onMemoryWarning() {
        session.run { mr_app_trim_memory($0) }
    }
}
