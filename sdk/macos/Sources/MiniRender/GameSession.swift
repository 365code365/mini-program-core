import AppKit
import QuartzCore
#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

/// 主线程上的回调（[GameSession] 交给 [MiniGameView]）
protocol GameSessionUI: AnyObject {
    /// 引擎线程要一个跟着视图所在屏幕刷新的显示同步（macOS 14 起只能从 NSView 取）
    func gameMakeDisplayLink(target: Any, selector: Selector) -> CADisplayLink?
    /// 引擎要 EGL 图层是这个像素尺寸：改好之后调 [GameSession.layerSized]
    func gameLayerWant(width: Int, height: Int)
    /// 屏上显示哪一块：EGL 图层（WebGL 主画布直接上屏）还是 CPU 帧那一块
    func gameShowLayer(metal: Bool)
    func gameCpuFrame(_ image: CGImage)
    func gameFirstFrame()
    /// 胶囊的矩形（逻辑像素，与 wx.getMenuButtonBoundingClientRect 同一个数）
    func gameMenuButton(_ rect: CGRect)
    func gameEvents(_ events: [[String: Any]])
}

/// macOS 版 [MiniGameView] 的引擎侧（与 iOS 的 GameSession.swift 同一套规则，少了传感器与 AVAudioSession）：
/// 专用引擎线程（[MiniEngineThread]）+ 从视图取的 CADisplayLink + 上屏。
///
/// - **上屏**：主画布是 WebGL 时经 ANGLE 的 EGL 窗口 surface 直接 swap 进视图的 CAMetalLayer（`mr_game_present_layer`）；
///   2D 主画布 / 拿不到 EGL 时取 BGRA 像素做成 CGImage 交给另一块图层；
/// - **出帧闸门**：`FRAME_WANTS_MORE` 才续帧；只剩远期定时器就停 vsync、按 `next_wake_us` 定时唤醒；
/// - **帧率**：`preferredFrameRateRange` 跟着 `mr_game_preferred_fps`；时钟是 `CADisplayLink.timestamp`（与 NSEvent.timestamp 同一个钟）。
final class GameSession: NSObject {
    weak var ui: GameSessionUI?
    let dir: String
    private let thread = MiniEngineThread()

    // ── 只在引擎线程读写 ──
    private var handle: OpaquePointer?
    private var link: CADisplayLink?
    private var running = true
    private var disposed = false
    private var appliedFps: Int32 = 0
    private var layerAttached = false
    private var forcePresent = false
    private var askedSize = (0, 0)
    private var showingMetal: Bool?
    private var firstFrameSent = false
    private var wakeTimer: Timer?

    init(dir: String) {
        self.dir = dir
        super.init()
        thread.name = "mini-game"
        thread.qualityOfService = .userInteractive
        thread.startAndWait()
    }

    /// 主线程调：建实例并执行 game.js（尺寸是逻辑像素）。显示同步在主线程从视图取，挂到引擎线程上
    func create(dataDir: String, width: UInt32, height: UInt32, dpr: Float, options: String) {
        let dir = self.dir
        let l = ui?.gameMakeDisplayLink(target: self, selector: #selector(tick(_:)))
        thread.perform { [self] in
            guard !disposed else { return }
            let t0 = CACurrentMediaTime()
            guard let h = mr_game_create(dir, dataDir, max(1, width), max(1, height), dpr, options) else {
                NSLog("❌ mini-game：建实例失败 \(dir)")
                return
            }
            handle = h
            NSLog("🎮 建实例 %.0fms（%d×%d@%.1f，%@）", (CACurrentMediaTime() - t0) * 1000, width, height, dpr, dir)
            if let l {
                l.add(to: RunLoop.current, forMode: .default)
                link = l
            }
            applyFrameRate(h, force: true)
            reportMenuButton(h)
            forcePresent = true
            drainEvents(h)
            requestFrame()
        }
    }

    // ───────────────────────── 命令（主线程进来） ─────────────────────────

    func run(_ block: @escaping (OpaquePointer) -> Void) {
        thread.perform { [self] in
            guard let h = handle else { return }
            block(h)
            drainEvents(h)
            requestFrame()
        }
    }

    /// 视图建好了 EGL 用的 CAMetalLayer。层由视图持有，**必须活到 [dispose] 之后**
    func attachLayer(_ layer: CAMetalLayer) {
        let ptr = Unmanaged.passUnretained(layer).toOpaque()
        thread.perform { [self] in
            guard let h = handle else { return }
            layerAttached = mr_game_attach_layer(h, ptr) == 1
            askedSize = (0, 0)
            forcePresent = true
            requestFrame()
        }
    }

    func layerSized(width: Int, height: Int) {
        thread.perform { [self] in
            guard let h = handle else { return }
            mr_game_layer_size(h, UInt32(width), UInt32(height))
            forcePresent = true
            requestFrame()
        }
    }

    func resize(width: UInt32, height: UInt32, dpr: Float) {
        run { h in
            mr_game_resize(h, max(1, width), max(1, height), dpr, nil)
            self.reportMenuButton(h)
            self.forcePresent = true
        }
    }

    /// 屏幕内缩（标题栏那一行）：引擎重算 safeArea / 胶囊位置（src/game/chrome.rs）
    func setSafeArea(top: CGFloat) {
        run { h in
            mr_game_set_safe_area(h, Float(top), 0, 0, 0)
            self.reportMenuButton(h)
        }
    }

    func foreground() {
        thread.perform { [self] in
            running = true
            guard let h = handle else { return }
            mr_game_set_background(h, 0)
            forcePresent = true
            drainEvents(h)
            requestFrame()
        }
    }

    /// 窗口最小化 / App 隐藏：wx.onHide 并同步停住出帧
    func background() {
        thread.performSync { [self] in
            running = false
            link?.isPaused = true
            wakeTimer?.invalidate()
            if let h = handle { mr_game_set_background(h, 1) }
        }
    }

    func eval(_ js: String, _ done: @escaping (String?) -> Void) {
        thread.perform { [self] in
            let r = handle.flatMap { mr_game_eval($0, js) }.map { String(cString: $0) }
            DispatchQueue.main.async { done(r) }
        }
    }

    /// 当前帧（截图验收：画面在 CAMetalLayer 上，窗口截图截不到）
    func snapshot(_ done: @escaping (CGImage?) -> Void) {
        thread.perform { [self] in
            let img = handle.flatMap { cpuImage($0) }
            DispatchQueue.main.async { done(img) }
        }
    }

    func dispose() {
        thread.performSync { [self] in
            disposed = true
            link?.invalidate()
            link = nil
            wakeTimer?.invalidate()
            if let h = handle {
                mr_game_detach_layer(h)
                mr_game_destroy(h)
                NSLog("🎮 关掉实例 %@", dir)
            }
            handle = nil
        }
        thread.stop()
    }

    // ───────────────────────── 引擎线程 ─────────────────────────

    private func reportMenuButton(_ h: OpaquePointer) {
        var x: Float = 0, y: Float = 0, w: Float = 0, hh: Float = 0
        mr_game_menu_button(h, &x, &y, &w, &hh)
        let r = CGRect(x: CGFloat(x), y: CGFloat(y), width: CGFloat(w), height: CGFloat(hh))
        DispatchQueue.main.async { [weak self] in self?.ui?.gameMenuButton(r) }
    }

    private func requestFrame() {
        guard running, !disposed, handle != nil else { return }
        wakeTimer?.invalidate()
        wakeTimer = nil
        link?.isPaused = false
    }

    private func applyFrameRate(_ h: OpaquePointer, force: Bool) {
        let fps = mr_game_preferred_fps(h)
        guard force || fps != appliedFps, let l = link else { return }
        appliedFps = fps
        let f = Float(fps)
        l.preferredFrameRateRange = CAFrameRateRange(minimum: min(f, 30), maximum: f, preferred: f)
    }

    @objc private func tick(_ l: CADisplayLink) {
        guard let h = handle, running else { return }
        let f = mr_game_pump_ex(h, UInt64(l.timestamp * 1_000_000))
        if f & Int32(MR_FRAME_NEW) != 0 || forcePresent { present(h) }
        if f & Int32(MR_FRAME_EVENTS) != 0 { drainEvents(h) }
        applyFrameRate(h, force: false)
        if f & Int32(MR_FRAME_WANTS_MORE) != 0 || forcePresent { return }
        l.isPaused = true
        if f & Int32(MR_FRAME_WAKE_AT) != 0 {
            let us = mr_game_next_wake_us(h)
            if us >= 0 {
                let t = Timer(timeInterval: max(0.001, Double(us) / 1_000_000), repeats: false) { [weak self] _ in self?.requestFrame() }
                RunLoop.current.add(t, forMode: .default)
                wakeTimer = t
            }
        }
    }

    private func present(_ h: OpaquePointer) {
        guard layerAttached else { return presentCpu(h) }
        let r = mr_game_present_layer(h)
        switch r {
        case 1:
            forcePresent = false
            show(metal: true)
            firstFrame()
        case 0:
            forcePresent = false
        default:
            // 图层尺寸还不对 / 2D 主画布 / 拿不到 EGL：这一帧走 CPU，要尺寸的话让主线程改
            askLayerSize(h)
            if r != -1 && r != Int32(MR_GAME_PRESENT_CPU) { NSLog("⚠️ mini-game 上屏失败（\(r)），这一帧走 CPU") }
            presentCpu(h)
        }
    }

    private func askLayerSize(_ h: OpaquePointer) {
        var w: UInt32 = 0
        var hh: UInt32 = 0
        mr_game_layer_want(h, &w, &hh)
        guard w > 0, hh > 0, (Int(w), Int(hh)) != askedSize else { return }
        askedSize = (Int(w), Int(hh))
        DispatchQueue.main.async { [weak self] in self?.ui?.gameLayerWant(width: Int(w), height: Int(hh)) }
    }

    private func presentCpu(_ h: OpaquePointer) {
        guard let img = cpuImage(h) else { return }
        forcePresent = false
        DispatchQueue.main.async { [weak self] in self?.ui?.gameCpuFrame(img) }
        show(metal: false)
        firstFrame()
    }

    private func show(metal: Bool) {
        guard showingMetal != metal else { return }
        showingMetal = metal
        DispatchQueue.main.async { [weak self] in self?.ui?.gameShowLayer(metal: metal) }
    }

    private func firstFrame() {
        guard !firstFrameSent else { return }
        firstFrameSent = true
        NSLog("🎮 第一帧上屏")
        DispatchQueue.main.async { [weak self] in self?.ui?.gameFirstFrame() }
    }

    private func cpuImage(_ h: OpaquePointer) -> CGImage? {
        var w: UInt32 = 0
        var hh: UInt32 = 0
        mr_game_frame_size(h, &w, &hh)
        let stride = Int(w) * 4
        let len = stride * Int(hh)
        guard len > 0 else { return nil }
        var data = Data(count: len)
        let n = data.withUnsafeMutableBytes { buf -> Int32 in
            guard let base = buf.baseAddress?.assumingMemoryBound(to: UInt8.self) else { return 0 }
            return mr_game_pixels_into(h, base, len, stride, 1)
        }
        guard n > 0, let provider = CGDataProvider(data: data as CFData) else { return nil }
        let info = CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue)
        return CGImage(
            width: Int(w), height: Int(hh), bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: stride,
            space: CGColorSpace(name: CGColorSpace.sRGB) ?? CGColorSpaceCreateDeviceRGB(), bitmapInfo: info,
            provider: provider, decode: nil, shouldInterpolate: true, intent: .defaultIntent)
    }

    /// 取完事件队列：日志、错误在这里打，其余交到主线程（键盘、胶囊样式、方向、hostCall …）
    private func drainEvents(_ h: OpaquePointer) {
        var list: [[String: Any]] = []
        while let p = mr_game_poll_event(h) {
            let s = String(cString: p)
            guard let ev = (try? JSONSerialization.jsonObject(with: Data(s.utf8))) as? [String: Any] else { continue }
            switch ev["type"] as? String {
            case "log":
                (ev["text"] as? String ?? "").split(separator: "\n", omittingEmptySubsequences: false).forEach { NSLog("[mini-game] %@", String($0)) }
            case "error":
                NSLog("⚠️ [mini-game] %@", s)
            // macOS 没有 AVAudioSession：静音键 / 混音选项只记日志（CoreAudio 输出照常）
            case "audioSession":
                NSLog("[mini-game] %@", s)
            default:
                list.append(ev)
            }
        }
        if !list.isEmpty {
            DispatchQueue.main.async { [weak self] in self?.ui?.gameEvents(list) }
        }
    }
}
