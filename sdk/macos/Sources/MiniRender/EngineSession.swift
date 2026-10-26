import AppKit
import QuartzCore

#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

/// 引擎专用线程：带 RunLoop，显示同步（CADisplayLink）挂在它上面，vsync 回调就在这条线程上。
/// 与 iOS SDK 的 `EngineSession.swift`、Android 的 `EngineSession.kt` 同一个结构：一帧 pump + 合成不放主线程，
/// 窗口拖动、菜单、AppKit 动画不被它拖住。
final class MiniEngineThread: Thread {
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

    /// 等引擎线程做完（返回 / 关窗要立刻知道结果）。[timeout] 秒之后不再等
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

extension NSLock {
    func locked<T>(_ body: () -> T) -> T {
        lock()
        defer { unlock() }
        return body()
    }
}

/// 主线程上的回调（[EngineSession] 交给视图）
protocol EngineSessionUI: AnyObject {
    /// 引擎像素尺寸定了 / 变了。`metal`：要不要一块 CAMetalLayer（偏好不是 cpu、还没回退）
    func sessionPixelSize(width: Int, height: Int, metal: Bool)
    /// CAMetalLayer 上交出了第一帧：显示出来
    func sessionReveal()
    /// GPU 没选上 / 运行中回退：拿掉 CAMetalLayer，之后走 CPU 的那张图
    func sessionLeftGpu()
    /// CPU 合成好的一帧
    func sessionCpuFrame(_ image: CGImage)
    /// 分栏时另一栏的图换了（nil = 不分栏了）
    func sessionSideFrame(_ image: CGImage?)
    func sessionEvents(_ events: [[String: Any]])
    /// 引擎线程要一个跟着视图所在屏幕刷新的显示同步（macOS 14 起只能从 NSView / NSScreen 取）
    func sessionMakeDisplayLink(target: Any, selector: Selector) -> CADisplayLink?
}

/// [MiniProgramView] 的引擎侧：专用引擎线程 + vsync 出帧 + 上屏。规则与 iOS 的同名类一条一条对齐：
///
/// - **后端**：偏好默认 auto。建引擎前 `mr_warmup`、建好之后 `set_device_info` → `set_backend` → `prewarm_gpu` → `launch`；
///   视图建好 CAMetalLayer 后 `attach_surface`。没选上 GPU、或运行中 `FRAME_BACKEND_CHANGED` 就卸掉 Metal 层改走 CPU。
/// - **上屏**：GPU 时引擎自己画进 CAMetalLayer（`FRAME_PRESENTED`）；CPU 时取 BGRA 像素做成 CGImage 交给视图。
/// - **出帧闸门**：引擎回报 `FRAME_WANTS_MORE` 才续帧，否则暂停显示同步 —— 静止页面零耗电；输入之后 [requestFrame]。
final class EngineSession: NSObject {
    struct Device {
        let totalMem: UInt64
        let osMajor: UInt32
    }

    weak var ui: EngineSessionUI?
    private let thread = MiniEngineThread()

    // ── 只在引擎线程读写 ──
    private var handle: OpaquePointer?
    private var link: CADisplayLink?
    private var running = true
    private var gpu = false
    private var gpuAttached = false
    private var revealed = false
    private var needsPresent = false

    // ── 引擎线程写、主线程读 ──
    private let lock = NSLock()
    private var published = (ready: false, pixelW: 0, pixelH: 0, backend: Int32(1), reason: "")

    var ready: Bool { lock.locked { published.ready } }
    var activeBackend: Int32 { lock.locked { published.backend } }
    var backendReason: String { lock.locked { published.reason } }

    override init() {
        super.init()
        thread.name = "mini-render"
        thread.qualityOfService = .userInteractive
        thread.startAndWait()
    }

    /// 主线程调：引擎线程上建引擎、打开首页。`width` / `height` 是逻辑像素
    func create(appDir: String, dataDir: String, route: String?, width: UInt32, height: UInt32, dpr: Float,
                options: String, backend: Int32, device: Device) {
        // 显示同步只能在主线程从视图取，取到之后挂到引擎线程的 RunLoop 上
        let l = ui?.sessionMakeDisplayLink(target: self, selector: #selector(tick(_:)))
        thread.perform { [self] in
            _ = mr_warmup(width, height, dpr, backend, device.totalMem, 0, device.osMajor)
            guard let h = mr_app_create_ex(appDir, dataDir, width, height, dpr, options) else {
                NSLog("❌ mini-render：小程序打不开 \(appDir)（建引擎失败）")
                return
            }
            mr_app_set_device_info(h, device.totalMem, 0, device.osMajor)
            _ = mr_app_set_backend(h, backend)
            gpu = backend != Int32(MR_BACKEND_CPU)
            if gpu { _ = mr_app_prewarm_gpu(h) }
            guard mr_app_launch(h, route) == 1 else {
                mr_app_destroy(h)
                NSLog("❌ mini-render：小程序打不开 \(appDir)（首页没打开）")
                return
            }
            handle = h
            let (pw, ph) = updatePixelSize()
            refreshBackend()
            lock.locked { published.ready = true }
            if let l {
                l.preferredFrameRateRange = CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
                l.add(to: RunLoop.current, forMode: .default)
                link = l
            }
            needsPresent = true
            let metal = gpu
            DispatchQueue.main.async { [weak self] in self?.ui?.sessionPixelSize(width: pw, height: ph, metal: metal) }
        }
    }

    // ───────────────────────── 命令（主线程进来） ─────────────────────────

    func run(_ block: @escaping (OpaquePointer) -> Void) {
        thread.perform { [self] in
            guard let h = handle else { return }
            block(h)
            requestFrame()
        }
    }

    func call<T>(_ fallback: T, _ block: @escaping (OpaquePointer) -> T) -> T {
        guard ready else { return fallback }
        var r = fallback
        thread.performSync { [self] in
            guard let h = handle else { return }
            r = block(h)
            requestFrame()
        }
        return r
    }

    /// 窗口回到前台 / 不再最小化：`App.onShow`，恢复出帧并补画当前帧
    func foreground() {
        thread.perform { [self] in
            running = true
            guard let h = handle else { return }
            mr_app_on_show(h)
            needsPresent = true
            requestFrame()
        }
    }

    /// 窗口最小化 / App 隐藏：`App.onHide`，停住出帧
    func background() {
        thread.performSync { [self] in
            running = false
            link?.isPaused = true
            if let h = handle { mr_app_on_hide(h) }
        }
    }

    /// 拖窗口边时 AppKit 每个鼠标事件就改一次尺寸（120Hz 触控板上一秒上百次）。从前每次都排一条
    /// `mr_app_resize` 到引擎线程：改宽度一次要重排整页，排队的比处理的快，画面越拖越落后、松手后还在追。
    /// 现在只记**最新的**尺寸，下一个 vsync 统一改一次（每帧至多一次重排），中间的尺寸直接丢掉
    func resize(width: UInt32, height: UInt32, dpr: Float) {
        let first = lock.locked { () -> Bool in
            let had = pendingSize != nil
            pendingSize = (width, height, dpr)
            return !had
        }
        if first { thread.perform { [self] in requestFrame() } }
    }

    private var pendingSize: (UInt32, UInt32, Float)?

    /// 引擎线程、tick 开头：把攒下的尺寸改掉
    private func applyPendingSize(_ h: OpaquePointer) {
        guard let (w, hh, d) = lock.locked({ () -> (UInt32, UInt32, Float)? in
            defer { pendingSize = nil }
            return pendingSize
        }) else { return }
        guard mr_app_resize(h, w, hh, d) == 1 else { return }
        let (pw, ph) = updatePixelSize()
        let metal = gpu
        DispatchQueue.main.async { [weak self] in self?.ui?.sessionPixelSize(width: pw, height: ph, metal: metal) }
        needsPresent = true
    }

    /// 视图建好了 CAMetalLayer。层由视图持有，**必须活到 [dispose] 之后**
    func attachMetal(_ layer: CAMetalLayer) {
        let ptr = Unmanaged.passUnretained(layer).toOpaque()
        thread.perform { [self] in
            guard let h = handle, gpu else { return }
            if gpuAttached { mr_app_detach_surface(h) }
            let r = mr_app_attach_surface(h, Int32(MR_SURFACE_METAL_LAYER), ptr)
            refreshBackend()
            if r == Int32(MR_BACKEND_GPU) {
                gpuAttached = true
                NSLog("🎨 mini-render GPU 上屏：\(backendReason)")
                needsPresent = true
                requestFrame()
            } else {
                NSLog("mini-render GPU 没选上（\(backendReason)），改走 CPU")
                leaveGpu(h)
            }
        }
    }

    func snapshot(_ done: @escaping (CGImage?) -> Void) {
        thread.perform { [self] in
            let img = handle.flatMap { cpuImage($0) }
            DispatchQueue.main.async { done(img) }
        }
    }

    /// 主线程调：先让引擎线程放手 Metal 层并销毁引擎，再停线程
    func dispose() {
        thread.performSync { [self] in
            link?.invalidate()
            link = nil
            if let h = handle {
                if gpuAttached { mr_app_detach_surface(h) }
                mr_app_destroy(h)
            }
            handle = nil
            gpuAttached = false
        }
        thread.stop()
    }

    // ───────────────────────── 引擎线程 ─────────────────────────

    @discardableResult
    private func updatePixelSize() -> (Int, Int) {
        var w: UInt32 = 0
        var h: UInt32 = 0
        mr_app_pixel_size(handle, &w, &h)
        lock.locked {
            published.pixelW = Int(w)
            published.pixelH = Int(h)
        }
        return (Int(w), Int(h))
    }

    private func refreshBackend() {
        guard let h = handle else { return }
        let b = mr_app_backend(h)
        let reason = mr_app_backend_reason(h).map { String(cString: $0) } ?? ""
        lock.locked {
            published.backend = b
            published.reason = reason
        }
    }

    private func leaveGpu(_ h: OpaquePointer) {
        if gpuAttached { mr_app_detach_surface(h) }
        gpuAttached = false
        gpu = false
        needsPresent = true
        DispatchQueue.main.async { [weak self] in self?.ui?.sessionLeftGpu() }
        requestFrame()
    }

    private func requestFrame() {
        guard running, handle != nil else { return }
        link?.isPaused = false
    }

    @objc private func tick(_ l: CADisplayLink) {
        guard let h = handle, running else { return }
        applyPendingSize(h)
        // vsync 时刻（与 NSEvent.timestamp、CACurrentMediaTime 同一个钟）：动画按它取样
        let f = mr_app_pump_ex(h, UInt64(l.timestamp * 1000))
        if f & Int32(MR_FRAME_BACKEND_CHANGED) != 0 && gpu {
            refreshBackend()
            NSLog("⚠️ mini-render GPU 回退 CPU：\(backendReason)")
            leaveGpu(h)
        }
        // 事件、另一栏、主画面放进同一个主线程块：分栏开关时「换摆法」与两栏的新图在同一次提交里上屏，
        // 不会有一帧新图摆在旧位置上
        let events = f & Int32(MR_FRAME_EVENTS) != 0 ? pollEvents(h) : []
        let sideChanged = f & Int32(MR_FRAME_SIDE) != 0
        let side = sideChanged ? sideImage(h) : nil
        var frame: CGImage?
        if gpu {
            needsPresent = false
            if f & Int32(MR_FRAME_PRESENTED) != 0 && !revealed {
                revealed = true
                DispatchQueue.main.async { [weak self] in self?.ui?.sessionReveal() }
            }
        } else {
            if f & Int32(MR_FRAME_NEW) != 0 { needsPresent = true }
            if needsPresent, let img = cpuImage(h) {
                needsPresent = false
                frame = img
            }
        }
        if !events.isEmpty || sideChanged || frame != nil {
            DispatchQueue.main.async { [weak self] in
                guard let ui = self?.ui else { return }
                if !events.isEmpty { ui.sessionEvents(events) }
                if sideChanged { ui.sessionSideFrame(side) }
                if let frame { ui.sessionCpuFrame(frame) }
            }
        }
        if f & Int32(MR_FRAME_WANTS_MORE) == 0 && !needsPresent { l.isPaused = true }
    }

    /// 当前帧 → CGImage（BGRA 小端）。每帧一块新的缓冲：主线程显示时还在用它
    private func cpuImage(_ h: OpaquePointer) -> CGImage? {
        image(h, size: mr_app_pixel_size, write: mr_app_pixels_into)
    }

    /// 分栏时另一栏的图（不分栏为 nil）
    private func sideImage(_ h: OpaquePointer) -> CGImage? {
        image(h, size: mr_app_side_pixel_size, write: mr_app_side_pixels_into)
    }

    private func image(
        _ h: OpaquePointer,
        size: (OpaquePointer?, UnsafeMutablePointer<UInt32>?, UnsafeMutablePointer<UInt32>?) -> Void,
        write: (OpaquePointer?, UnsafeMutablePointer<UInt8>?, Int, UInt32, Int32) -> Int
    ) -> CGImage? {
        var w: UInt32 = 0
        var hh: UInt32 = 0
        size(h, &w, &hh)
        let stride = Int(w) * 4
        let len = stride * Int(hh)
        guard len > 0 else { return nil }
        var data = Data(count: len)
        let n = data.withUnsafeMutableBytes { buf -> Int in
            guard let base = buf.baseAddress?.assumingMemoryBound(to: UInt8.self) else { return 0 }
            return write(h, base, len, UInt32(stride), Int32(MR_PIXEL_BGRA8))
        }
        guard n > 0, let provider = CGDataProvider(data: data as CFData) else { return nil }
        let info = CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue)
        return CGImage(
            width: Int(w), height: Int(hh), bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: stride,
            space: CGColorSpace(name: CGColorSpace.sRGB) ?? CGColorSpaceCreateDeviceRGB(), bitmapInfo: info,
            provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent)
    }

    private func pollEvents(_ h: OpaquePointer) -> [[String: Any]] {
        var list: [[String: Any]] = []
        while let p = mr_app_poll_event(h) {
            guard let data = String(cString: p).data(using: .utf8),
                  let ev = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else { continue }
            list.append(ev)
        }
        return list
    }
}
