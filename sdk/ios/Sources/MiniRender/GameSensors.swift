import CoreLocation
import CoreMotion
import Foundation
import QuartzCore

/// 小游戏的传感器（与 Android 的 GameSensors.kt 对应）：SDK 自己采集，作为宿主 API 接住
/// `wx.startAccelerometer / stopAccelerometer`、`startGyroscope / stopGyroscope`、`startCompass / stopCompass`、
/// `startDeviceMotionListening / stopDeviceMotionListening`（[MiniGameView] 建实例时把这几个名字并进 hostApis），
/// 采到的数据经 `mr_game_dispatch_event` 推成 `accelerometerChange` 等事件。
///
/// - interval：`game` 20ms、`ui` 60ms、`normal`（缺省）200ms（官方），直接设成 CoreMotion 的 updateInterval；
/// - 加速度 `{x, y, z}` 以 g 为单位，CoreMotion 原值（竖屏平放时 z ≈ -1）；陀螺仪 `{x, y, z}` rad/s；
/// - DeviceMotion `{alpha, beta, gamma}` 度：按 W3C DeviceOrientation 的 Z-X'-Y'' 欧拉角从姿态矩阵换算
///   （alpha 0~360、beta −180~180、gamma −90~90），参考系优先磁北；
/// - 罗盘 `{direction, accuracy}`：CLLocationManager 的磁北航向（不要定位权限），accuracy 按 headingAccuracy 分
///   high（≤10°）/ medium（≤25°）/ low / unreliable（<0）；
/// - 切后台全部停（[pauseAll]），回前台按原来的间隔恢复；设备没有对应传感器时 start fail。
/// [handle] / [pauseAll] / [resumeAll] / [release] 在引擎线程上调；采样回调经 [post] 投递回引擎线程再 [emit]。
final class GameSensors: NSObject, CLLocationManagerDelegate {
    static let apis = [
        "startAccelerometer", "stopAccelerometer", "startGyroscope", "stopGyroscope",
        "startCompass", "stopCompass", "startDeviceMotionListening", "stopDeviceMotionListening",
    ]

    private let post: (@escaping () -> Void) -> Void
    private let emit: (_ name: String, _ json: String) -> Void
    private let motion = CMMotionManager()
    private let queue = OperationQueue()
    /// 在跑的种类 → 间隔（秒）
    private var running: [String: Double] = [:]
    private var paused = false
    /// 罗盘（主线程上建、用）
    private var location: CLLocationManager?
    private var compassInterval = 0.2
    private var lastCompass = 0.0

    init(post: @escaping (@escaping () -> Void) -> Void, emit: @escaping (_ name: String, _ json: String) -> Void) {
        self.post = post
        self.emit = emit
        queue.maxConcurrentOperationCount = 1
        super.init()
    }

    /// 宿主调用：处理了返回 (ok, 结果 JSON)，不归这里管返回 nil
    func handle(api: String, args: [String: Any]?) -> (ok: Bool, json: String)? {
        guard Self.apis.contains(api) else { return nil }
        let kind = api.hasPrefix("start") ? String(api.dropFirst(5)) : String(api.dropFirst(4))
        let ok = (true, Self.json(["errMsg": "\(api):ok"]))
        if api.hasPrefix("stop") {
            stop(kind)
            return ok
        }
        let available: Bool
        switch kind {
        case "Accelerometer": available = motion.isAccelerometerAvailable
        case "Gyroscope": available = motion.isGyroAvailable
        case "Compass": available = CLLocationManager.headingAvailable()
        default: available = motion.isDeviceMotionAvailable
        }
        guard available else { return (false, Self.json(["errMsg": "\(api):fail 设备没有这个传感器"])) }
        let interval: Double
        switch args?["interval"] as? String {
        case "game": interval = 0.02
        case "ui": interval = 0.06
        default: interval = 0.2
        }
        stop(kind)
        running[kind] = interval
        if !paused { start(kind, interval) }
        return ok
    }

    func pauseAll() {
        paused = true
        running.keys.forEach(halt)
    }

    func resumeAll() {
        guard paused else { return }
        paused = false
        running.forEach { start($0.key, $0.value) }
    }

    func release() {
        running.keys.forEach(halt)
        running.removeAll()
    }

    // ───────────────────────── 采集 ─────────────────────────

    private func stop(_ kind: String) {
        guard running.removeValue(forKey: kind) != nil else { return }
        halt(kind)
    }

    private func send(_ name: String, _ data: [String: Any]) {
        let j = Self.json(data)
        post { [weak self] in self?.emit(name, j) }
    }

    private func start(_ kind: String, _ interval: Double) {
        switch kind {
        case "Accelerometer":
            motion.accelerometerUpdateInterval = interval
            motion.startAccelerometerUpdates(to: queue) { [weak self] d, _ in
                guard let a = d?.acceleration else { return }
                self?.send("accelerometerChange", ["x": a.x, "y": a.y, "z": a.z])
            }
        case "Gyroscope":
            motion.gyroUpdateInterval = interval
            motion.startGyroUpdates(to: queue) { [weak self] d, _ in
                guard let r = d?.rotationRate else { return }
                self?.send("gyroscopeChange", ["x": r.x, "y": r.y, "z": r.z])
            }
        case "Compass":
            compassInterval = interval
            DispatchQueue.main.async { [self] in
                let m = location ?? CLLocationManager()
                m.delegate = self
                m.headingFilter = kCLHeadingFilterNone
                location = m
                m.startUpdatingHeading()
            }
        default:
            motion.deviceMotionUpdateInterval = interval
            let frames = CMMotionManager.availableAttitudeReferenceFrames()
            let frame: CMAttitudeReferenceFrame = frames.contains(.xMagneticNorthZVertical) ? .xMagneticNorthZVertical : .xArbitraryZVertical
            motion.startDeviceMotionUpdates(using: frame, to: queue) { [weak self] d, _ in
                guard let m = d?.attitude.rotationMatrix else { return }
                self?.send("deviceMotionChange", Self.euler(m))
            }
        }
    }

    private func halt(_ kind: String) {
        switch kind {
        case "Accelerometer": motion.stopAccelerometerUpdates()
        case "Gyroscope": motion.stopGyroUpdates()
        case "Compass": DispatchQueue.main.async { [self] in location?.stopUpdatingHeading() }
        default: motion.stopDeviceMotionUpdates()
        }
    }

    func locationManager(_ manager: CLLocationManager, didUpdateHeading h: CLHeading) {
        let now = CACurrentMediaTime()
        // 按要的间隔抽样（留 2ms 余量，系统给的时刻有抖动）
        guard now - lastCompass >= compassInterval - 0.002 else { return }
        lastCompass = now
        let acc = h.headingAccuracy
        let level = acc < 0 ? "unreliable" : acc <= 10 ? "high" : acc <= 25 ? "medium" : "low"
        send("compassChange", ["direction": h.magneticHeading, "accuracy": level])
    }

    /// 姿态矩阵 → W3C 的 alpha / beta / gamma（度）。CoreMotion 的矩阵是「参考系 → 设备」，W3C 的是「设备 → 地球」：取转置
    static func euler(_ c: CMRotationMatrix) -> [String: Any] {
        let (m12, m22, m31, m32, m33) = (c.m21, c.m22, c.m13, c.m23, c.m33)
        var a: Double, b: Double, g: Double
        if m33 > 0 {
            a = atan2(-m12, m22); b = asin(max(-1, min(1, m32))); g = atan2(-m31, m33)
        } else if m33 < 0 {
            a = atan2(m12, -m22); b = -asin(max(-1, min(1, m32))); b += b >= 0 ? -Double.pi : Double.pi; g = atan2(m31, -m33)
        } else {
            a = atan2(-m12, m22); b = m32 > 0 ? Double.pi / 2 : -Double.pi / 2; g = 0
        }
        if a < 0 { a += 2 * Double.pi }
        let deg = 180 / Double.pi
        return ["alpha": a * deg, "beta": b * deg, "gamma": max(-90, min(90, g * deg))]
    }

    static func json(_ o: [String: Any]) -> String {
        (try? JSONSerialization.data(withJSONObject: o)).map { String(decoding: $0, as: UTF8.self) } ?? "{}"
    }
}
