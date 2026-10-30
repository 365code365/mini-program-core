import Foundation
import QuartzCore

/// 小游戏帧统计（诊断开关：启动参数 `-gameStats 1` 或环境变量 `MINI_GAME_STATS=1`）：每 2 秒一行
/// 「⏱ [GAME] …」—— 上屏帧数、上屏间隔的中位 / 最大、> 1.5 倍目标间隔的次数（掉帧），
/// pump（逻辑 + rAF + 绘制提交）与 present（交给 CAMetalLayer）的中位 / p95 / 最大。
/// 与 Android 的 `debug.mini.game_log`（MiniGameView.kt 的「⏱ [GAME] 工作」）对应；不开时每帧只多一次布尔判断。
///
/// 为什么看这几项：平均帧率会骗人（steering 10）。iOS 上 `adaptiveFps` 跑不满 60 会降到 30，
/// 手感上就是「一卡一卡」—— 要看的是 pump 的 p95 有没有越过 16.7ms，以及上屏间隔的峰值
final class GameFrameStats {
    static let enabled: Bool = {
        UserDefaults.standard.bool(forKey: "gameStats") || ProcessInfo.processInfo.environment["MINI_GAME_STATS"] == "1"
    }()

    private var pumps: [Double] = []
    private var presents: [Double] = []
    private var gaps: [Double] = []
    private var lastPresentAt: CFTimeInterval = 0
    private var windowStart: CFTimeInterval = 0

    /// 一次 tick 的耗时（毫秒）与这一帧是否上屏；`target` 是 rAF 的目标帧率
    func record(pumpMs: Double, presentMs: Double?, at now: CFTimeInterval, target: Int32) {
        if windowStart == 0 { windowStart = now }
        pumps.append(pumpMs)
        if let p = presentMs {
            presents.append(p)
            if lastPresentAt > 0 { gaps.append((now - lastPresentAt) * 1000) }
            lastPresentAt = now
        }
        guard now - windowStart >= 2 else { return }
        let span = now - windowStart
        let budget = 1000 / Double(max(1, target))
        let slow = gaps.filter { $0 > budget * 1.5 }.count
        // 发热档（nominal / fair / serious / critical）与低电量模式：系统按它们压 CPU 频率，同一帧能慢到 3 倍
        let pi = ProcessInfo.processInfo
        let thermal = ["正常", "温", "热", "过热"][min(3, pi.thermalState.rawValue)]
        NSLog("⏱ [GAME] %.0fs 上屏 %d 帧（%.1f fps，目标 %d）间隔中位 %.1f 最大 %.1fms 掉帧 %d；pump 中位 %.1f p95 %.1f 最大 %.1fms；present 中位 %.1f 最大 %.1fms；发热 %@%@",
              span, presents.count, Double(presents.count) / span, target, Self.q(gaps, 0.5), gaps.max() ?? 0, slow,
              Self.q(pumps, 0.5), Self.q(pumps, 0.95), pumps.max() ?? 0, Self.q(presents, 0.5), presents.max() ?? 0,
              thermal, pi.isLowPowerModeEnabled ? "，低电量模式" : "")
        pumps.removeAll(keepingCapacity: true)
        presents.removeAll(keepingCapacity: true)
        gaps.removeAll(keepingCapacity: true)
        windowStart = now
    }

    /// 停帧（暂停 vsync）期间不算间隔：否则静止画面也被记成掉帧
    func idle() { lastPresentAt = 0 }

    private static func q(_ a: [Double], _ p: Double) -> Double {
        guard !a.isEmpty else { return 0 }
        let s = a.sorted()
        return s[min(s.count - 1, Int(Double(s.count - 1) * p))]
    }
}
