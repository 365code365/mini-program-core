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
/// - 按 `bounds` 与 `contentScaleFactor` 算逻辑尺寸和 dpr；
/// - `CADisplayLink` 驱动，**只在引擎说有新画面时才上屏**，静止时不出帧；
/// - `touchesBegan/Moved/Ended/Cancelled` → 引擎的**指针三段**
///   （含 `coalescedTouches`，120Hz 的中间点不丢）；
/// - 前后台与内存告警转发（`didReceiveMemoryWarning` → 释放解码图与自定义字体）。
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
/// 控制器 pop（在导航栈里）或 dismiss。普通页面的导航栏是宿主画的，胶囊也由宿主放进自己的导航栏。
public final class MiniProgramView: UIView {

    /// 引擎在 custom 导航栏的页面上画右上角胶囊（`open` 之前设）
    public var menuButton = true
    /// 胶囊「•••」：查看关于。nil = 默认弹框
    public var onAbout: ((MiniProgramAbout) -> Void)?
    /// 胶囊「⊙」：返回宿主。nil = 默认 pop / dismiss 所在的控制器
    public var onClose: (() -> Void)?
    /// 引擎发来的每一条事件（JSON 对象；`page` / `navigationBar` / `inputFocus` / `app` / `menuButton` …）
    public var onEvent: (([String: Any]) -> Void)?
    /// 小程序的关于信息（启动后第一帧到）
    public private(set) var about: MiniProgramAbout?

    private var handle: OpaquePointer?
    private var link: CADisplayLink?
    private var pixelBuffer = [UInt8]()
    private let imageLayer = CALayer()
    private var appDir: URL?
    private var startRoute: String?

    public override init(frame: CGRect) {
        super.init(frame: frame)
        setup()
    }

    public required init?(coder: NSCoder) {
        super.init(coder: coder)
        setup()
    }

    private func setup() {
        isMultipleTouchEnabled = false
        backgroundColor = .white
        imageLayer.magnificationFilter = .nearest
        imageLayer.frame = bounds
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

    deinit {
        stop()
        if let h = handle { mr_app_destroy(h) }
    }

    /// 打开一个解包后的小程序目录，可指定起始页（深链）
    public func open(appDir: URL, route: String? = nil) {
        self.appDir = appDir
        self.startRoute = route
        createEngineIfPossible()
    }

    /// 跳到某个页面（`/pages/x/x?id=1`）
    public func navigate(_ url: String) {
        guard let h = handle else { return }
        mr_app_navigate(h, url)
    }

    /// 返回键 / 侧滑返回。**返回 false 表示页面栈只剩一页**，宿主该关掉小程序。
    @discardableResult
    public func goBack() -> Bool {
        guard let h = handle else { return false }
        return mr_app_back(h) != 0
    }

    public override func layoutSubviews() {
        super.layoutSubviews()
        imageLayer.frame = bounds
        if handle == nil { createEngineIfPossible() }
    }

    private func createEngineIfPossible() {
        guard handle == nil, let dir = appDir,
              bounds.width > 0, bounds.height > 0 else { return }
        // 数据目录必须给：storage 与图片磁盘缓存都落这里，不给会静默失效
        let data = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("mini-render")
        try? FileManager.default.createDirectory(at: data, withIntermediateDirectories: true)
        let dpr = Float(window?.screen.scale ?? UIScreen.main.scale)
        // 选项要在 app.js 执行前就位：onLaunch 里读 getSystemInfoSync（状态栏高度、胶囊位置）是常态
        let insets = safeAreaInsets
        let options: [String: Any] = [
            "systemInfo": [
                "platform": "ios",
                "model": UIDevice.current.model,
                "system": "iOS \(UIDevice.current.systemVersion)",
            ],
            "safeArea": ["top": insets.top, "right": insets.right, "bottom": insets.bottom, "left": insets.left],
            "menuButton": menuButton,
        ]
        let json = (try? JSONSerialization.data(withJSONObject: options)).flatMap { String(data: $0, encoding: .utf8) }
        handle = mr_app_create_ex(
            dir.path, data.path,
            UInt32(bounds.width), UInt32(bounds.height), dpr, json)
        guard let h = handle else {
            NSLog("❌ mini-render 创建失败")
            return
        }
        mr_app_launch(h, startRoute)
        start()
    }

    /// 刘海 / 灵动岛 / 底部横条：`statusBarHeight`、胶囊位置与 `env(safe-area-inset-*)` 都按它算
    public override func safeAreaInsetsDidChange() {
        super.safeAreaInsetsDidChange()
        guard let h = handle else { return }
        let i = safeAreaInsets
        mr_app_set_safe_area(h, Float(i.top), Float(i.right), Float(i.bottom), Float(i.left))
    }

    // ───────────────────────── 出帧 ─────────────────────────

    private func start() {
        guard link == nil else { return }
        let l = CADisplayLink(target: self, selector: #selector(tick))
        l.add(to: .main, forMode: .common)
        link = l
    }

    private func stop() {
        link?.invalidate()
        link = nil
    }

    @objc private func tick() {
        guard let h = handle else { return }
        let now = UInt64(CACurrentMediaTime() * 1000)
        let flags = mr_app_pump_ex(h, now)
        if flags & Int32(MR_FRAME_EVENTS) != 0 { drainEvents() }
        guard flags & Int32(MR_FRAME_NEW) != 0 else { return }   // 没变化就不上屏
        var w: UInt32 = 0, hh: UInt32 = 0
        mr_app_pixel_size(h, &w, &hh)
        let need = Int(w) * Int(hh) * 4
        guard need > 0 else { return }
        if pixelBuffer.count != need { pixelBuffer = [UInt8](repeating: 0, count: need) }
        let written = pixelBuffer.withUnsafeMutableBufferPointer {
            mr_app_pixels(h, $0.baseAddress, need)
        }
        guard written == need else { return }
        present(width: Int(w), height: Int(hh))
    }

    private func present(width: Int, height: Int) {
        pixelBuffer.withUnsafeBufferPointer { buf in
            guard let base = buf.baseAddress,
                  let provider = CGDataProvider(
                    dataInfo: nil, data: base, size: buf.count, releaseData: { _, _, _ in })
            else { return }
            let info: CGBitmapInfo = [CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue)]
            if let image = CGImage(
                width: width, height: height,
                bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: width * 4,
                space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: info,
                provider: provider, decode: nil, shouldInterpolate: false,
                intent: .defaultIntent)
            {
                imageLayer.contents = image
            }
        }
    }

    // ───────────────────────── 事件 ─────────────────────────

    /// 取完引擎的事件队列（取到 NULL 为止），胶囊的两个动作交给宿主
    private func drainEvents() {
        guard let h = handle else { return }
        while let p = mr_app_poll_event(h) {
            guard let data = String(cString: p).data(using: .utf8),
                  let ev = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else { continue }
            onEvent?(ev)
            switch ev["type"] as? String {
            case "app":
                if let a = ev["about"] as? [String: Any] { about = MiniProgramAbout(json: a) }
            case "menuButton":
                switch MiniMenuButtonAction(rawValue: ev["action"] as? String ?? "") {
                case .menu: showAbout()
                case .close: requestClose()
                case nil: break
                }
            default: break
            }
        }
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

    private func send(_ phase: Int32, _ p: CGPoint) {
        guard let h = handle else { return }
        let (x, y) = (Float(p.x), Float(p.y))
        switch phase {
        case 0: mr_app_pointer_down(h, x, y)
        case 1: mr_app_pointer_move(h, x, y)
        case 2: mr_app_pointer_up(h, x, y)
        default: mr_app_pointer_cancel(h)
        }
    }

    public override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let t = touches.first else { return }
        send(0, t.location(in: self))
    }

    public override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let t = touches.first else { return }
        // 120Hz 屏上一帧可能积压多个点，全都要送（抽稀会影响滑动手感与方向判定）
        for c in event?.coalescedTouches(for: t) ?? [t] {
            send(1, c.location(in: self))
        }
    }

    public override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let t = touches.first else { return }
        send(2, t.location(in: self))
    }

    public override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        send(3, .zero)
    }

    /// 键盘提交的文字（自己接 `UIKeyInput` 时把字符转过来即可）
    public func sendText(_ text: String) {
        guard let h = handle else { return }
        mr_app_text_input(h, text)
    }

    /// 功能键：enter / newline / backspace / delete / left / right / up / down / home / end、
    /// shift+… 扩展选区、word-left / word-right、doc-start / doc-end、delete-word-back、
    /// selectall / copy / cut / undo / redo / escape。回车也可以 sendText("\n")（textarea 换行）
    public func sendKey(_ name: String) {
        guard let h = handle else { return }
        mr_app_key(h, name)
    }

    // ─────────────────────── 生命周期 ───────────────────────

    @objc private func onForeground() {
        guard let h = handle else { return }
        mr_app_on_show(h)
        start()
    }

    /// 进后台：派发 `App.onHide` 并**停掉 DisplayLink**（继续出帧纯属耗电）
    @objc private func onBackground() {
        guard let h = handle else { return }
        mr_app_on_hide(h)
        stop()
    }

    @objc private func onMemoryWarning() {
        guard let h = handle else { return }
        mr_app_trim_memory(h)
    }
}
