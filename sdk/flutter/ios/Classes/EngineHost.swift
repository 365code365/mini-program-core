import CoreVideo
import Flutter
import QuartzCore
import UIKit
#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

/// Flutter 外部纹理：始终交出**最新**的一帧。
///
/// `copyPixelBuffer` 在 raster 线程上被调；连续多次 `textureFrameAvailable` 只会触发一次取帧
/// （flutter#159162），所以不排队，只留最新一张。交出去之后 Flutter 持有它直到不用，
/// 我们从池子里拿下一张写，不会写到它正在采样的那张上。
final class PixelTexture: NSObject, FlutterTexture {
    private let lock = NSLock()
    private var latest: CVPixelBuffer?

    func update(_ buffer: CVPixelBuffer) {
        lock.lock()
        latest = buffer
        lock.unlock()
    }

    func copyPixelBuffer() -> Unmanaged<CVPixelBuffer>? {
        lock.lock()
        defer { lock.unlock() }
        guard let b = latest else { return nil }
        return Unmanaged.passRetained(b)
    }
}

/// 引擎专用线程：带 RunLoop，CADisplayLink 挂在它上面，vsync 回调就在这条线程上。
///
/// 为什么不用主线程的 CADisplayLink（原生 SDK 的 MiniProgramView 就是这么做的）：
/// Flutter 3.29 起 Dart 也跑在主线程上，引擎的一帧（3× 屏上动辄几毫秒到十几毫秒）
/// 放主线程会把整个 App 一起拖住。
final class EngineThread: Thread {
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

    var runLoop: CFRunLoop? { loop }

    func perform(_ block: @escaping () -> Void) {
        guard let loop else { return }
        CFRunLoopPerformBlock(loop, CFRunLoopMode.defaultMode.rawValue, block)
        CFRunLoopWakeUp(loop)
    }

    func performSync(_ block: @escaping () -> Void) {
        let done = DispatchSemaphore(value: 0)
        perform {
            block()
            done.signal()
        }
        _ = done.wait(timeout: .now() + 1)
    }

    func stop() {
        cancel()
        if let loop { CFRunLoopStop(loop) }
    }
}

/// 一个小程序实例在 iOS 上的宿主：引擎线程 + vsync 出帧 + CVPixelBuffer 纹理。
/// 结构与 Android 的 EngineHost.kt 一一对应（出帧闸门、事件、命令）。
final class EngineHost: NSObject {
    private let textures: FlutterTextureRegistry
    private let onEvents: ([String]) -> Void
    private let thread = EngineThread()
    private let texture = PixelTexture()
    private var textureId: Int64 = 0

    // ── 只在引擎线程读写 ──
    private var handle: OpaquePointer?
    private var link: CADisplayLink?
    private var running = true
    private var needsPresent = false
    private var pool: CVPixelBufferPool?
    private var poolSize = (0, 0)
    private(set) var pixelW = 0
    private(set) var pixelH = 0

    init(textures: FlutterTextureRegistry, onEvents: @escaping ([String]) -> Void) {
        self.textures = textures
        self.onEvents = onEvents
        super.init()
        thread.name = "mini-render"
        thread.qualityOfService = .userInteractive
        thread.startAndWait()
    }

    func create(appDir: String, dataDir: String, route: String?, width: Int, height: Int, dpr: Float,
                options: String, result: @escaping FlutterResult, onFailed: @escaping () -> Void) {
        textureId = textures.register(texture)
        thread.perform { [self] in
            let created = mr_app_create_ex(appDir, dataDir, UInt32(max(1, width)), UInt32(max(1, height)), dpr, options)
            guard let h = created, mr_app_launch(h, route) == 1 else {
                if let c = created { mr_app_destroy(c) } // 建出来了但首页打不开：别留着
                DispatchQueue.main.async { [self] in
                    textures.unregisterTexture(textureId)
                    onFailed()
                    result(FlutterError(code: "CREATE_FAILED", message: "小程序打不开：\(appDir)", details: nil))
                }
                return
            }
            handle = h
            updatePixelSize()
            let l = CADisplayLink(target: self, selector: #selector(tick(_:)))
            if #available(iOS 15.0, *) {
                // ProMotion 机型默认被限在 60Hz，要显式要高刷
                l.preferredFrameRateRange = CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
            }
            l.add(to: RunLoop.current, forMode: .default)
            link = l
            needsPresent = true
            let info = textureInfo()
            DispatchQueue.main.async { result(info) }
        }
    }

    /// `backend` 恒为 1（CPU）：Flutter 在 iOS 的外部纹理是 CVPixelBuffer，不是 CAMetalLayer，
    /// 引擎的 GPU 合成器没有可上屏的目标（Dart 传来的 `backend` 偏好在这里不起作用）
    private func textureInfo() -> [String: Any] {
        [
            "textureId": textureId, "pixelWidth": pixelW, "pixelHeight": pixelH,
            "backend": 1, "backendReason": "iOS 外部纹理（CVPixelBuffer）只走 CPU",
        ]
    }

    // ───────────────────────── 引擎线程 ─────────────────────────

    private func updatePixelSize() {
        var w: UInt32 = 0
        var h: UInt32 = 0
        mr_app_pixel_size(handle, &w, &h)
        pixelW = Int(w)
        pixelH = Int(h)
    }

    /// 挂下一个 vsync（输入 / 命令之后，或引擎回报 WANTS_MORE）
    private func requestFrame() {
        guard running, handle != nil else { return }
        link?.isPaused = false
    }

    @objc private func tick(_ l: CADisplayLink) {
        guard let h = handle, running else { return }
        // vsync 时刻（与 UITouch.timestamp 同一个钟，Dart 的 PointerEvent.timeStamp 就是它）：
        // 动画按它取样、拖动按它重采样。从前传 CACurrentMediaTime()（回调「此刻」），
        // 画的时刻相对 vsync 忽早忽晚，与原生 iOS 宿主（EngineSession.swift）不一致
        let f = mr_app_pump_ex(h, UInt64(l.timestamp * 1000))
        if f & Int32(MR_FRAME_NEW) != 0 { needsPresent = true }
        if needsPresent { present() }
        if f & Int32(MR_FRAME_EVENTS) != 0 { drainEvents() }
        // 没活干就停 vsync：静止页面零耗电，下一条输入再 requestFrame
        if f & Int32(MR_FRAME_WANTS_MORE) == 0 && !needsPresent { l.isPaused = true }
    }

    private func bufferPool() -> CVPixelBufferPool? {
        if let p = pool, poolSize == (pixelW, pixelH) { return p }
        let attrs: [String: Any] = [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
            kCVPixelBufferWidthKey as String: pixelW,
            kCVPixelBufferHeightKey as String: pixelH,
            // Flutter 用 CVMetalTextureCache 取纹理：必须是 IOSurface 支撑的
            kCVPixelBufferIOSurfacePropertiesKey as String: [:] as [String: Any],
            kCVPixelBufferMetalCompatibilityKey as String: true,
        ]
        let poolAttrs: [String: Any] = [kCVPixelBufferPoolMinimumBufferCountKey as String: 3]
        var p: CVPixelBufferPool?
        CVPixelBufferPoolCreate(nil, poolAttrs as CFDictionary, attrs as CFDictionary, &p)
        pool = p
        poolSize = (pixelW, pixelH)
        return p
    }

    /// 写一张 BGRA 的 CVPixelBuffer（Flutter 的外部纹理只认 32BGRA / NV12）交给纹理。
    /// `notify: false`：只换上新帧，由调用方决定什么时候 `textureFrameAvailable`（改尺寸时用，见 resize）
    @discardableResult
    private func present(notify: Bool = true) -> Bool {
        guard let h = handle, pixelW > 0, pixelH > 0, let pool = bufferPool() else { return false }
        var pb: CVPixelBuffer?
        guard CVPixelBufferPoolCreatePixelBuffer(nil, pool, &pb) == kCVReturnSuccess, let buffer = pb else { return false }
        CVPixelBufferLockBaseAddress(buffer, [])
        let stride = CVPixelBufferGetBytesPerRow(buffer)
        var written = 0
        if let base = CVPixelBufferGetBaseAddress(buffer) {
            written = mr_app_pixels_into(h, base.assumingMemoryBound(to: UInt8.self), stride * pixelH,
                                         UInt32(stride), Int32(MR_PIXEL_BGRA8))
        }
        CVPixelBufferUnlockBaseAddress(buffer, [])
        guard written > 0 else { return false }
        texture.update(buffer)
        needsPresent = false
        if notify {
            let id = textureId
            DispatchQueue.main.async { [weak self] in self?.textures.textureFrameAvailable(id) }
        }
        return true
    }

    private func drainEvents() {
        var list: [String] = []
        while let p = mr_app_poll_event(handle) {
            list.append(String(cString: p))
        }
        if !list.isEmpty {
            let cb = onEvents
            DispatchQueue.main.async { cb(list) }
        }
    }

    /// 在引擎线程上执行一条命令，之后挂一帧
    private func run(_ block: @escaping (OpaquePointer) -> Void) {
        thread.perform { [self] in
            guard let h = handle else { return }
            block(h)
            requestFrame()
        }
    }

    // ───────────────────────── 命令（主线程进来） ─────────────────────────

    func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
        let a = call.arguments as? [String: Any] ?? [:]
        func f(_ k: String) -> Float { (a[k] as? NSNumber)?.floatValue ?? 0 }
        func i(_ k: String) -> Int { (a[k] as? NSNumber)?.intValue ?? 0 }
        func s(_ k: String) -> String { a[k] as? String ?? "" }
        switch call.method {
        case "pointer":
            let (phase, x, y) = (i("phase"), f("x"), f("y"))
            // 移动带事件时间（Dart 的 PointerEvent.timeStamp，iOS 上就是 UITouch.timestamp）：
            // 引擎按它重采样页面拖动。从前这里丢了 `t`、调不带时间的 move —— 重采样被清空，
            // 经 MethodChannel 一帧到几个移动是碰运气，慢拖一顿一顿（清单第 9 / 15 条）
            let t = (a["t"] as? NSNumber)?.doubleValue
            run { h in
                switch phase {
                case 0: mr_app_pointer_down(h, x, y)
                case 1:
                    if let t { mr_app_pointer_move_at(h, x, y, t) } else { mr_app_pointer_move(h, x, y) }
                case 2: mr_app_pointer_up(h, x, y)
                default: mr_app_pointer_cancel(h)
                }
            }
            result(nil)
        case "wheel":
            let (dy, precise) = (f("dy"), (a["precise"] as? Bool ?? true) ? Int32(1) : 0)
            run { mr_app_wheel($0, dy, precise) }
            result(nil)
        case "text":
            let t = s("text")
            run { mr_app_text_input($0, t) }
            result(nil)
        case "key":
            let n = s("name")
            run { mr_app_key($0, n) }
            result(nil)
        case "navigate":
            let u = s("url")
            run { _ = mr_app_navigate($0, u) }
            result(nil)
        case "back":
            thread.perform { [self] in
                let ok = handle.map { mr_app_back($0) != 0 } ?? false
                requestFrame()
                DispatchQueue.main.async { result(ok) }
            }
        case "resize":
            let (w, h, dpr) = (i("width"), i("height"), f("dpr"))
            thread.perform { [self] in
                guard let e = handle, mr_app_resize(e, UInt32(max(1, w)), UInt32(max(1, h)), dpr) != 0 else {
                    DispatchQueue.main.async { result(nil) }
                    return
                }
                updatePixelSize()
                // 新尺寸的那一帧先写好、先回尺寸给 Dart，再告诉 Flutter 有新帧：Dart 按新尺寸
                // 重建 Texture 的那一帧取到的就是新画面。从前是回尺寸、下一个 vsync 才画 ——
                // 中间那一帧旧画面被拉伸到新尺寸（切换导航栏样式时整页先放大一下再弹回来，
                // 2026-10-22 Android 模拟器上录屏看到的，iOS 同一个时序）
                needsPresent = true
                let presented = present(notify: false)
                if !presented { requestFrame() }
                let info = textureInfo()
                let id = textureId
                DispatchQueue.main.async { [weak self] in
                    result(info)
                    if presented { self?.textures.textureFrameAvailable(id) }
                }
            }
        case "safeArea":
            let (t, r, b, l) = (f("top"), f("right"), f("bottom"), f("left"))
            run { _ = mr_app_set_safe_area($0, t, r, b, l) }
            result(nil)
        case "lifecycle":
            let what = i("what")
            thread.perform { [self] in
                guard let h = handle else { return }
                switch what {
                case 0:
                    mr_app_on_show(h)
                    running = true
                    needsPresent = true
                    requestFrame()
                case 1:
                    mr_app_on_hide(h)
                    running = false // 进后台：停止出帧（继续出帧纯属耗电）
                    link?.isPaused = true
                default:
                    mr_app_trim_memory(h)
                }
            }
            result(nil)
        case "hostReply":
            let (id, ok, res) = (i("id"), a["ok"] as? Bool ?? false, s("result"))
            run { _ = mr_app_host_reply($0, UInt32(max(0, id)), ok ? 1 : 0, res) }
            result(nil)
        case "nativeViewEvent":
            let (key, name, detail) = (s("key"), s("name"), s("detail"))
            run { _ = mr_app_native_view_event($0, key, name, detail) }
            result(nil)
        case "dispatchEvent":
            // 宿主推事件给 wx.onXxx：引擎线程上派发，回监听者个数
            let (name, payload) = (s("name"), s("payload"))
            thread.perform { [self] in
                let n = handle.map { Int(mr_app_dispatch_event($0, name, payload)) } ?? 0
                // 回调里多半 setData 了：在引擎线程上挂一帧（与 run 一样）。
                // 从前在主线程上调 requestFrame —— running / handle / link 只归引擎线程，那是数据竞争
                requestFrame()
                DispatchQueue.main.async { result(n) }
            }
        case "eval":
            let code = s("code")
            thread.perform { [self] in
                let r = handle.flatMap { mr_app_eval($0, code) }.map { String(cString: $0) }
                DispatchQueue.main.async { result(r) }
            }
        case "dumpFrame":
            let path = s("path")
            thread.perform { [self] in
                let ok = dumpFrame(to: path)
                DispatchQueue.main.async { result(ok) }
            }
        default:
            result(FlutterMethodNotImplemented)
        }
    }

    /// 调试：把当前帧存成 PNG（与屏幕截图对比纹理有没有被拉伸）
    private func dumpFrame(to path: String) -> Bool {
        guard let h = handle, pixelW > 0, pixelH > 0 else { return false }
        let stride = pixelW * 4
        var bytes = [UInt8](repeating: 0, count: stride * pixelH)
        let n = bytes.withUnsafeMutableBufferPointer {
            mr_app_pixels_into(h, $0.baseAddress, $0.count, UInt32(stride), Int32(MR_PIXEL_RGBA8))
        }
        guard n > 0,
              let provider = CGDataProvider(data: Data(bytes) as CFData),
              let image = CGImage(width: pixelW, height: pixelH, bitsPerComponent: 8, bitsPerPixel: 32,
                                  bytesPerRow: stride, space: CGColorSpaceCreateDeviceRGB(),
                                  bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipLast.rawValue),
                                  provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent),
              let png = UIImage(cgImage: image).pngData()
        else { return false }
        return (try? png.write(to: URL(fileURLWithPath: path))) != nil
    }

    /// 主线程调：引擎线程上销毁引擎，再注销纹理
    func dispose() {
        thread.performSync { [self] in
            link?.invalidate()
            link = nil
            if let h = handle { mr_app_destroy(h) }
            handle = nil
        }
        thread.stop()
        textures.unregisterTexture(textureId)
    }
}
