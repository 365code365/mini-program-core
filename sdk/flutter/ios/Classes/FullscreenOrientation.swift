import UIKit

/// 全屏视频跟着手机转（与 Android `FullscreenOrientation.kt` 同一套）：只在全屏期间听设备朝向，
/// 换成引擎的 0 / 90 / -90 报上去（`mr_app_set_device_orientation`）。引擎转的是全屏那一层，页面不重排。
/// 倒拿、平放不理；系统锁了竖屏时 UIDevice 不报横屏，也就不转（与系统里别的 App 一致）。
/// 关掉时报一次 0：下一次进全屏从竖屏开始。
final class FullscreenOrientation {
    private let report: (Int32) -> Void
    private var observer: NSObjectProtocol?
    private var current: Int32 = 0

    init(report: @escaping (Int32) -> Void) { self.report = report }

    /// 主线程调
    func enable(_ on: Bool) {
        if on == (observer != nil) { return }
        if on {
            UIDevice.current.beginGeneratingDeviceOrientationNotifications()
            observer = NotificationCenter.default.addObserver(
                forName: UIDevice.orientationDidChangeNotification, object: nil, queue: .main
            ) { [weak self] _ in self?.update() }
            update()
        } else {
            if let o = observer { NotificationCenter.default.removeObserver(o) }
            observer = nil
            UIDevice.current.endGeneratingDeviceOrientationNotifications()
            if current != 0 { report(0) }
            current = 0
        }
    }

    private func update() {
        let next: Int32
        switch UIDevice.current.orientation {
        // landscapeLeft：Home 键在右 = 设备逆时针转了 90 度、左侧朝下 = 引擎的 90
        case .landscapeLeft: next = 90
        case .landscapeRight: next = -90
        case .portrait: next = 0
        default: return
        }
        if next != current {
            current = next
            report(next)
        }
    }
}
