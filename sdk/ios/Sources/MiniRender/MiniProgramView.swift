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
public final class MiniProgramView: UIView {

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
        handle = mr_app_create(
            dir.path, data.path,
            UInt32(bounds.width), UInt32(bounds.height), dpr)
        guard let h = handle else {
            NSLog("❌ mini-render 创建失败")
            return
        }
        mr_app_launch(h, startRoute)
        start()
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
        guard mr_app_pump(h, now) != 0 else { return }   // 没变化就不上屏
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

    /// 功能键：enter / backspace / delete / left / right / home / end / selectall
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
