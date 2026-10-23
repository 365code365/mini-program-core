import QuartzCore
import UIKit

#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

/// 引擎专用线程：带 RunLoop，CADisplayLink 挂在它上面，vsync 回调就在这条线程上。
///
/// 为什么不再用主线程的 CADisplayLink（这个视图从前就是）：一帧 pump + 合成在 3× 屏上动辄几毫秒，
/// GPU 后端的 Fifo 取 drawable 还可能阻塞 —— 放主线程就是把 UIKit 的触摸、动画一起拖住。
/// 与 Android 的 `EngineSession.kt`、Flutter iOS 插件的 `EngineThread` 同一个结构。
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

    /// 等引擎线程做完（返回键要立刻知道结果、进后台前要停住 Metal）。[timeout] 秒之后不再等
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
    /// `withLock` 要 iOS 16：部署目标是 13
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
    func sessionEvents(_ events: [[String: Any]])
}

/// [MiniProgramView] 的引擎侧：专用引擎线程 + vsync 出帧 + 上屏。规则与 Android 的 `EngineSession.kt` 一条一条对齐：
///
/// - **后端**：偏好默认 auto。建引擎前 `mr_warmup`（字体与 GPU 各一条线程，与建引擎并行）、建好之后
///   `set_device_info` → `set_backend` → `prewarm_gpu` → `launch`；视图建好 CAMetalLayer 后 `attach_surface`。
///   返回不是 GPU（门槛 / 自检没过）、或运行中 `FRAME_BACKEND_CHANGED`（设备丢失）就卸掉 Metal 层改走 CPU，回退有日志。
/// - **上屏**：GPU 时引擎自己画进 CAMetalLayer（`FRAME_PRESENTED`），滚动帧只合成；CPU 时取 BGRA 像素
///   做成 CGImage 交给视图（只在有新画面时）。
/// - **出帧闸门**：引擎回报 `FRAME_WANTS_MORE`（动画 / 惯性 / 定时器 / 请求在飞 / 按住的手指）才续帧，
///   否则暂停 DisplayLink —— 静止页面零耗电；输入 / 命令之后 [requestFrame]。
/// - **进后台**：同步停住 DisplayLink 再回（Metal 在后台提交命令会被系统杀掉）。
final class EngineSession: NSObject {
    struct Device {
        let totalMem: UInt64
        let lowRam: Bool
        let osMajor: UInt32
    }

    weak var ui: EngineSessionUI?
    private let thread = MiniEngineThread()

    // ── 只在引擎线程读写 ──
    private var handle: OpaquePointer?
    private var link: CADisplayLink?
    private var running = true
    /// 走 GPU（偏好不是 cpu、还没回退）。挂上之前的 CPU 帧不显示（Metal 层藏着，露出视图背景色）
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
    var pixelSize: (Int, Int) { lock.locked { (published.pixelW, published.pixelH) } }

    override init() {
        super.init()
        thread.name = "mini-render"
        // 与 Android 的 THREAD_PRIORITY_DISPLAY 同一档：抢不过 UI，但不被后台任务挤
        thread.qualityOfService = .userInteractive
        thread.startAndWait()
    }

    /// 主线程调：引擎线程上建引擎、打开首页。`width` / `height` 是逻辑像素
    func create(appDir: String, dataDir: String, route: String?, width: UInt32, height: UInt32, dpr: Float,
                options: String, backend: Int32, device: Device) {
        thread.perform { [self] in
            // 字体（约 0.35s）与 GPU（约 0.5s）跟建引擎并行。宽高 / dpr 必须与下面的 create_ex 一致，否则认领不上
            _ = mr_warmup(width, height, dpr, backend, device.totalMem, device.lowRam ? 1 : 0, device.osMajor)
            guard let h = mr_app_create_ex(appDir, dataDir, width, height, dpr, options) else {
                NSLog("❌ mini-render：小程序打不开 \(appDir)（建引擎失败）")
                return
            }
            // 机型信息要在挂 surface 之前给（iOS 没有机型门槛，给了只为日志与 Android 一致）
            mr_app_set_device_info(h, device.totalMem, device.lowRam ? 1 : 0, device.osMajor)
            _ = mr_app_set_backend(h, backend)
            gpu = backend != Int32(MR_BACKEND_CPU)
            // 开 GPU 与首页启动并行：CAMetalLayer 建出来时合成器多半已经好了
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
            let l = CADisplayLink(target: self, selector: #selector(tick(_:)))
            if #available(iOS 15.0, *) {
                // ProMotion 机型默认被限在 60Hz，要显式要高刷（iPhone 还要 Info.plist 的 CADisableMinimumFrameDurationOnPhone）
                l.preferredFrameRateRange = CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
            }
            l.add(to: RunLoop.current, forMode: .default)
            link = l
            needsPresent = true
            let metal = gpu
            DispatchQueue.main.async { [weak self] in self?.ui?.sessionPixelSize(width: pw, height: ph, metal: metal) }
        }
    }

    // ───────────────────────── 命令（主线程进来） ─────────────────────────

    /// 在引擎线程上执行一条命令，之后挂一帧（输入之后引擎要出帧回应）
    func run(_ block: @escaping (OpaquePointer) -> Void) {
        thread.perform { [self] in
            guard let h = handle else { return }
            block(h)
            requestFrame()
        }
    }

    /// 同步等引擎线程的结果（返回键要立刻决定 pop 还是不动）。引擎没建好返回 [fallback]
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

    /// 进前台：`App.onShow`，恢复出帧并补画当前帧
    func foreground() {
        thread.perform { [self] in
            running = true
            guard let h = handle else { return }
            mr_app_on_show(h)
            needsPresent = true
            requestFrame()
        }
    }

    /// 进后台：`App.onHide` 并**同步**停住出帧（Metal 在后台提交命令会被系统杀进程）
    func background() {
        thread.performSync { [self] in
            running = false
            link?.isPaused = true
            if let h = handle { mr_app_on_hide(h) }
        }
    }

    /// 视图改了尺寸（逻辑像素）：引擎重排，视图跟着改层的尺寸
    func resize(width: UInt32, height: UInt32, dpr: Float) {
        thread.perform { [self] in
            guard let h = handle, mr_app_resize(h, width, height, dpr) == 1 else { return }
            let (pw, ph) = updatePixelSize()
            let metal = gpu
            DispatchQueue.main.async { [weak self] in self?.ui?.sessionPixelSize(width: pw, height: ph, metal: metal) }
            needsPresent = true
            requestFrame()
        }
    }

    /// 视图建好了 CAMetalLayer（尺寸已按引擎像素设好）。层由视图持有，**必须活到 [detachMetal] / [dispose] 之后**
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

    /// 把引擎当前这一帧取成 CGImage（截图 / 自动化验收；GPU 上屏时引擎补一次同图层的 CPU 合成，与屏上逐字节相同）
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

    /// 离开 GPU：放掉合成器手里的层，之后走 CPU（视图拿掉 Metal 层、显示 CPU 那张图）
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
        // vsync 时刻（与 UITouch.timestamp、CACurrentMediaTime 同一个钟）：动画按它取样，触摸按它重采样。
        // 传「此刻」的话画的时刻相对 vsync 忽早忽晚，动画会抖
        let f = mr_app_pump_ex(h, UInt64(l.timestamp * 1000))
        if f & Int32(MR_FRAME_BACKEND_CHANGED) != 0 && gpu {
            refreshBackend()
            NSLog("⚠️ mini-render GPU 回退 CPU：\(backendReason)")
            leaveGpu(h)
        }
        if gpu {
            // 引擎已经画进 CAMetalLayer 了
            needsPresent = false
            if f & Int32(MR_FRAME_PRESENTED) != 0 && !revealed {
                revealed = true
                DispatchQueue.main.async { [weak self] in self?.ui?.sessionReveal() }
            }
        } else {
            if f & Int32(MR_FRAME_NEW) != 0 { needsPresent = true }
            if needsPresent, let img = cpuImage(h) {
                needsPresent = false
                DispatchQueue.main.async { [weak self] in self?.ui?.sessionCpuFrame(img) }
            }
        }
        if f & Int32(MR_FRAME_EVENTS) != 0 { drainEvents(h) }
        // 没活干就停 vsync：静止页面零耗电，下一条输入再 requestFrame
        if f & Int32(MR_FRAME_WANTS_MORE) == 0 && !needsPresent { l.isPaused = true }
    }

    /// 当前帧 → CGImage（BGRA 小端 = iOS 的原生格式，Core Animation 不用再转）。每帧一块新的缓冲：
    /// CGImage 交给主线程显示时还在用它，复用的话下一帧会写进正在显示的那张
    private func cpuImage(_ h: OpaquePointer) -> CGImage? {
        var w: UInt32 = 0
        var hh: UInt32 = 0
        mr_app_pixel_size(h, &w, &hh)
        let stride = Int(w) * 4
        let len = stride * Int(hh)
        guard len > 0 else { return nil }
        var data = Data(count: len)
        let n = data.withUnsafeMutableBytes { buf -> Int in
            guard let base = buf.baseAddress?.assumingMemoryBound(to: UInt8.self) else { return 0 }
            return Int(mr_app_pixels_into(h, base, len, UInt32(stride), Int32(MR_PIXEL_BGRA8)))
        }
        guard n > 0, let provider = CGDataProvider(data: data as CFData) else { return nil }
        let info = CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue)
        return CGImage(
            width: Int(w), height: Int(hh), bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: stride,
            space: CGColorSpace(name: CGColorSpace.sRGB) ?? CGColorSpaceCreateDeviceRGB(), bitmapInfo: info,
            provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent)
    }

    /// 取完事件队列（取到 NULL 为止），交到主线程分发
    private func drainEvents(_ h: OpaquePointer) {
        var list: [[String: Any]] = []
        while let p = mr_app_poll_event(h) {
            guard let data = String(cString: p).data(using: .utf8),
                  let ev = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else { continue }
            list.append(ev)
        }
        if !list.isEmpty {
            DispatchQueue.main.async { [weak self] in self?.ui?.sessionEvents(list) }
        }
    }
}
