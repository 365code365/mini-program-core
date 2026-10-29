import QuartzCore
import UIKit
#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

/// 小游戏的引擎线程（与小程序的 MiniEngineThread 同一个写法；单独一份是因为 Flutter 插件只收小游戏这几个文件）
final class GameThread: Thread {
    private var loop: CFRunLoop?
    private let ready = DispatchSemaphore(value: 0)

    override func main() {
        loop = CFRunLoopGetCurrent()
        // 没有输入源的 RunLoop 会立刻返回：挂一个空端口让它常驻
        RunLoop.current.add(NSMachPort(), forMode: .default)
        ready.signal()
        while !isCancelled {
            RunLoop.current.run(mode: .default, before: .distantFuture)
        }
    }

    func startAndWait() {
        start()
        ready.wait()
    }

    func perform(_ block: @escaping () -> Void) {
        guard let loop else { return }
        CFRunLoopPerformBlock(loop, CFRunLoopMode.defaultMode.rawValue, block)
        CFRunLoopWakeUp(loop)
    }

    /// 等引擎线程做完（进后台要同步停住 Metal / EGL、销毁要先放手图层）。[timeout] 秒之后不再等
    func performSync(timeout: Double = 2, _ block: @escaping () -> Void) {
        let done = DispatchSemaphore(value: 0)
        perform {
            block()
            done.signal()
        }
        _ = done.wait(timeout: .now() + timeout)
    }

    func stop() {
        cancel()
        if let loop { CFRunLoopStop(loop) }
    }
}

/// 主线程上的回调（[GameSession] 交给 [MiniGameView]）
protocol GameSessionUI: AnyObject {
    /// 引擎要 EGL 图层是这个像素尺寸：改好之后调 [GameSession.layerSized]
    func gameLayerWant(width: Int, height: Int)
    /// 屏上显示哪一块：EGL 图层（WebGL 主画布直接上屏）还是 CPU 帧那一块
    func gameShowLayer(metal: Bool)
    func gameCpuFrame(_ image: CGImage)
    func gameFirstFrame()
    func gameEvents(_ events: [[String: Any]])
}

/// [MiniGameView] 的引擎侧：专用线程 + CADisplayLink + 上屏。规则与 Android 的 `MiniGameView.kt` 一条一条对齐：
///
/// - **上屏**：主画布是 WebGL 时经 ANGLE 的 EGL 窗口 surface 直接 swap 到视图的 CAMetalLayer（`mr_game_present_layer`），
///   不读回；2D 主画布 / 拿不到 EGL 时取 BGRA 像素做成 CGImage 交给另一块图层（EGL 用过的图层不放 CPU 帧）。
///   主画布按自己的分辨率交出去，由 Core Animation 缩放到视图，CPU 不做缩放；
/// - **出帧闸门**：`FRAME_WANTS_MORE` 才续帧；只剩远期定时器（`FRAME_WAKE_AT`）就停 vsync、按 `next_wake_us` 定时唤醒；
/// - **帧率**：`preferredFrameRateRange` 跟着 `mr_game_preferred_fps`（setPreferredFramesPerSecond / 自适应 30 档），
///   系统按它选显示刷新率，不白白每个 120Hz vsync 都醒一次；
/// - **时钟**：pump 传 `CADisplayLink.timestamp`（与 `UITouch.timestamp` 同一个钟），微秒；
/// - **进后台**：同步停住出帧再回（Metal 在后台提交命令会被系统杀进程）。
final class GameSession: NSObject {
    weak var ui: GameSessionUI?
    let dir: String
    private let thread = GameThread()

    // ── 只在引擎线程读写 ──
    private var handle: OpaquePointer?
    private var link: CADisplayLink?
    private var running = true
    private var disposed = false
    private var appliedFps: Int32 = 0
    private var layerAttached = false
    /// 新挂上图层 / 尺寸刚改好 / 回前台：下一帧不管有没有新画面都交一次
    private var forcePresent = false
    /// 已经让主线程去改的图层尺寸（同一个尺寸不重复要）
    private var askedSize = (0, 0)
    private var showingMetal: Bool?
    private var firstFrameSent = false
    private var wakeTimer: Timer?
    private var sensors: GameSensors?
    private var audio: GameAudioSession?

    init(dir: String) {
        self.dir = dir
        super.init()
        thread.name = "mini-game"
        thread.qualityOfService = .userInteractive
        thread.startAndWait()
    }

    /// 建实例并执行 game.js（尺寸是逻辑像素）。建好之后挂 DisplayLink 开始出帧
    func create(dataDir: String, width: UInt32, height: UInt32, dpr: Float, options: String) {
        let dir = self.dir
        thread.perform { [self] in
            guard !disposed else { return }
            let t0 = CACurrentMediaTime()
            guard let h = mr_game_create(dir, dataDir, max(1, width), max(1, height), dpr, options) else {
                NSLog("❌ mini-game：建实例失败 \(dir)")
                return
            }
            handle = h
            NSLog("🎮 建实例 %.0fms（%d×%d@%.1f，%@）", (CACurrentMediaTime() - t0) * 1000, width, height, dpr, dir)
            sensors = GameSensors(post: { [weak self] b in self?.thread.perform(b) }) { [weak self] name, json in
                guard let self, let g = handle else { return }
                _ = mr_game_dispatch_event(g, name, json)
                afterCommand(g)
            }
            audio = GameAudioSession(game: { [weak self] in self?.handle }, post: { [weak self] b in
                self?.thread.perform {
                    b()
                    if let g = self?.handle { self?.afterCommand(g) }
                }
            })
            let l = CADisplayLink(target: self, selector: #selector(tick(_:)))
            l.add(to: RunLoop.current, forMode: .default)
            link = l
            applyFrameRate(h, force: true)
            forcePresent = true
            drainEvents(h)
            requestFrame()
        }
    }

    // ───────────────────────── 命令（主线程进来） ─────────────────────────

    /// 在引擎线程上执行一条命令，之后取事件、挂一帧
    func run(_ block: @escaping (OpaquePointer) -> Void) {
        thread.perform { [self] in
            guard let h = handle else { return }
            block(h)
            afterCommand(h)
        }
    }

    /// 视图建好了 EGL 用的 CAMetalLayer。层由视图持有，**必须活到 [dispose] 之后**
    func attachLayer(_ layer: CAMetalLayer) {
        let ptr = Unmanaged.passUnretained(layer).toOpaque()
        thread.perform { [self] in
            // 实例可能还在建（create 排在前面，同一条线程上按顺序执行）：建失败就算了
            guard let h = handle else { return }
            layerAttached = mr_game_attach_layer(h, ptr) == 1
            askedSize = (0, 0)
            forcePresent = true
            requestFrame()
        }
    }

    /// 主线程把图层改成了引擎要的像素尺寸
    func layerSized(width: Int, height: Int) {
        thread.perform { [self] in
            guard let h = handle else { return }
            mr_game_layer_size(h, UInt32(width), UInt32(height))
            forcePresent = true
            requestFrame()
        }
    }

    /// 触摸（每个指针一条）：`t` 是 UITouch.timestamp（秒）
    func touches(_ list: [(phase: Int32, id: Int32, x: Float, y: Float, force: Float, t: TimeInterval)]) {
        run { h in
            for e in list { mr_game_touch(h, e.phase, e.id, e.x, e.y, e.force, UInt64(max(0, e.t) * 1_000_000)) }
        }
    }

    func foreground() {
        thread.perform { [self] in
            running = true
            guard let h = handle else { return }
            mr_game_set_background(h, 0)
            sensors?.resumeAll()
            audio?.setForeground(true)
            forcePresent = true
            afterCommand(h)
        }
    }

    /// 进后台：**同步**停住出帧（EGL / Metal 在后台提交会被系统杀进程）
    func background() {
        thread.performSync { [self] in
            running = false
            link?.isPaused = true
            wakeTimer?.invalidate()
            guard let h = handle else { return }
            mr_game_set_background(h, 1)
            sensors?.pauseAll()
            audio?.setForeground(false)
        }
    }

    func resize(width: UInt32, height: UInt32, dpr: Float, orientation: String?) {
        run { h in
            if let o = orientation {
                mr_game_resize(h, max(1, width), max(1, height), dpr, o)
            } else {
                mr_game_resize(h, max(1, width), max(1, height), dpr, nil)
            }
            self.forcePresent = true
        }
    }

    /// 诊断：执行一段 JS，结果回到主线程
    func eval(_ js: String, _ done: @escaping (String?) -> Void) {
        thread.perform { [self] in
            let r = handle.flatMap { mr_game_eval($0, js) }.map { String(cString: $0) }
            DispatchQueue.main.async { done(r) }
        }
    }

    /// 主线程调：先让引擎线程放手图层并销毁实例，再停线程
    func dispose() {
        thread.performSync { [self] in
            disposed = true
            link?.invalidate()
            link = nil
            wakeTimer?.invalidate()
            sensors?.release()
            sensors = nil
            audio = nil
            if let h = handle {
                mr_game_detach_layer(h)
                mr_game_destroy(h)
            }
            handle = nil
        }
        thread.stop()
    }

    // ───────────────────────── 引擎线程 ─────────────────────────

    private func afterCommand(_ h: OpaquePointer) {
        drainEvents(h)
        requestFrame()
    }

    private func requestFrame() {
        guard running, !disposed, handle != nil else { return }
        wakeTimer?.invalidate()
        wakeTimer = nil
        link?.isPaused = false
    }

    /// rAF 帧率变了就重新要显示刷新率（60 的约数 / 整数倍由系统挑）
    private func applyFrameRate(_ h: OpaquePointer, force: Bool) {
        let fps = mr_game_preferred_fps(h)
        guard force || fps != appliedFps, let l = link else { return }
        appliedFps = fps
        if #available(iOS 15.0, *) {
            let f = Float(fps)
            l.preferredFrameRateRange = CAFrameRateRange(minimum: min(f, 30), maximum: f, preferred: f)
        } else {
            l.preferredFramesPerSecond = Int(fps)
        }
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
            // -1（图层尺寸还不是画布尺寸）/ 第一次接图层前尺寸就不对（引擎回 CPU）：让主线程改，改好了 layerSized 再来一帧；
            // 这一帧先走 CPU，不黑着。2D 主画布引擎不会要尺寸（want 是 0）
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

    /// 当前帧 → CGImage（BGRA 小端 = iOS 的原生格式）。每帧一块新缓冲：CGImage 交给主线程显示时还在用它
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

    /// 取完事件队列：日志、音频会话、传感器在这里处理，其余交到主线程（键盘、视频、方向、hostCall …）
    private func drainEvents(_ h: OpaquePointer) {
        var list: [[String: Any]] = []
        while let p = mr_game_poll_event(h) {
            let s = String(cString: p)
            guard let ev = (try? JSONSerialization.jsonObject(with: Data(s.utf8))) as? [String: Any] else { continue }
            switch ev["type"] as? String {
            case "log":
                (ev["text"] as? String ?? "").split(separator: "\n", omittingEmptySubsequences: false).forEach { NSLog("[mini-game] %@", String($0)) }
            case "audioSession":
                NSLog("[mini-game] %@", s)
                audio?.handle(event: ev)
            case "error":
                NSLog("⚠️ [mini-game] %@", s)
            case "hostCall":
                if let r = sensors?.handle(api: ev["api"] as? String ?? "", args: ev["args"] as? [String: Any]) {
                    _ = mr_game_host_reply(h, UInt32((ev["id"] as? NSNumber)?.intValue ?? 0), r.ok ? 1 : 0, r.json)
                } else {
                    list.append(ev)
                }
            default:
                list.append(ev)
            }
        }
        if !list.isEmpty {
            DispatchQueue.main.async { [weak self] in self?.ui?.gameEvents(list) }
        }
    }
}
