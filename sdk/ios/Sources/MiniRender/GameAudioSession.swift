// 小游戏的 iOS 音频会话（game-doc/04 G5.5；引擎侧见 src/game/audio/session.rs，Android 对应 GameAudioSession.kt）。
//
// - `wx.setInnerAudioOption`（引擎发 {"type":"audioSession","mixWithOther","obeyMuteSwitch","speakerOn"}）→ AVAudioSession：
//   obeyMuteSwitch = true → `.ambient`（静音键静音，天然与别的 App 混音）；false → `.playback`（静音键不管），
//   此时 mixWithOther = true 加 `.mixWithOthers`，false 不加（别的 App 的音乐停下）；
//   speakerOn 只在 `.playAndRecord` 类别下有意义（`.defaultToSpeaker`），游戏不录音时不用这个类别，只记下来；
// - 打断：`AVAudioSession.interruptionNotification` 的 `.began` / `.ended` → `mr_game_audio_interruption(1 / 0)`；
//   引擎暂停全部音频后派发 wx.onAudioInterruptionBegin / End，**不自动续播**（不看 `.shouldResume`，由游戏自己 play）；
// - 改类别**不先停用会话**：停用会把引擎已经起来的 RemoteIO 停掉，cpal 不知道，之后再也不出声（从前就是这样：
//   建实例之后第一次 apply 先 setActive(false)，真机上设备回调 0.68 秒后就不来了）。激活状态下直接 setCategory，
//   系统做一次路由切换、I/O 照跑；只有直接改失败时才走「停用 → 改 → 激活」，停掉的输出由引擎的看门狗重启（output.rs 的 Watch）。
//   建实例之前先建本对象（GameSession.create），第一次配类别时还没有 I/O。
//
// 所有调用都在小游戏的引擎线程上（与 mr_game_* 同一条线程）：通知在系统的线程上来，经 `post` 投递回去。
// 由 GameSession（MiniGameView 的引擎侧）持有。
import AVFoundation
#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

final class GameAudioSession {
    private let session = AVAudioSession.sharedInstance()
    private let post: (@escaping () -> Void) -> Void
    private let game: () -> OpaquePointer?
    private var observer: NSObjectProtocol?
    private var current: (mix: Bool, obey: Bool)?

    /// `game`：当前的 MRGame 句柄（实例销毁后返回 nil）；`post`：投递到引擎线程
    init(game: @escaping () -> OpaquePointer?, post: @escaping (@escaping () -> Void) -> Void) {
        self.game = game
        self.post = post
        apply(mixWithOther: true, obeyMuteSwitch: true)
        observer = NotificationCenter.default.addObserver(forName: AVAudioSession.interruptionNotification, object: session, queue: nil) { [weak self] n in
            guard let self, let raw = n.userInfo?[AVAudioSessionInterruptionTypeKey] as? UInt,
                  let type = AVAudioSession.InterruptionType(rawValue: raw) else { return }
            self.post {
                guard let g = self.game() else { return }
                // 打断结束要重新激活会话，系统才把 I/O 还回来（引擎恢复混音、看门狗把停掉的输出重启）
                if type == .ended { try? self.session.setActive(true) }
                mr_game_audio_interruption(g, type == .began ? 1 : 0)
            }
        }
    }

    deinit {
        if let o = observer { NotificationCenter.default.removeObserver(o) }
    }

    /// 前后台（与 Android 的 GameAudioSession.setForeground 对应）：回前台重新激活会话（别的 App 可能在后台期间拿走了它），
    /// 进后台不主动放手（引擎已经暂停全部音频；放手会让别的 App 的音乐被打断后又恢复一次）
    func setForeground(_ fg: Bool) {
        guard fg else { return }
        try? session.setActive(true)
    }

    /// `MINI_AUDIO_LOG=1`（环境变量）：每 2 秒一行「🔊 [GAME] 音频」—— 引擎的输出统计（设备回调数、样本峰值、在播几个）
    /// 加 AVAudioSession 的类别、音量、输出路由。排查「没声音」：callbacks 不涨 = 设备没在拉；peak 0 = 混音器出的是静音；
    /// 都有还听不见 = 会话类别 / 静音键 / 路由（与 Android 的 debug.mini.audio_log 对应）
    private static let logOn = ProcessInfo.processInfo.environment["MINI_AUDIO_LOG"] == "1"
    private var lastLog: CFTimeInterval = 0

    func logIfAsked(_ h: OpaquePointer, now: CFTimeInterval) {
        guard Self.logOn, now - lastLog >= 2 else { return }
        lastLog = now
        let stats = mr_game_audio_stats(h).map { String(cString: $0) } ?? "{}"
        let outs = session.currentRoute.outputs.map { $0.portType.rawValue }.joined(separator: ",")
        NSLog("🔊 [GAME] 音频 %@ 会话 %@%@ 音量 %.2f 输出 %@ 别的 App 在放 %@", stats, session.category.rawValue,
              session.categoryOptions.contains(.mixWithOthers) ? "+mix" : "", session.outputVolume, outs,
              session.isOtherAudioPlaying ? "是" : "否")
    }

    /// 引擎的 audioSession 事件（JSON 已解析）
    func handle(event: [String: Any]) {
        apply(mixWithOther: event["mixWithOther"] as? Bool ?? true, obeyMuteSwitch: event["obeyMuteSwitch"] as? Bool ?? true)
    }

    private func apply(mixWithOther: Bool, obeyMuteSwitch: Bool) {
        if let c = current, c.mix == mixWithOther, c.obey == obeyMuteSwitch { return }
        current = (mixWithOther, obeyMuteSwitch)
        let category: AVAudioSession.Category = obeyMuteSwitch ? .ambient : .playback
        let options: AVAudioSession.CategoryOptions = (!obeyMuteSwitch && mixWithOther) ? [.mixWithOthers] : []
        do {
            try session.setCategory(category, mode: .default, options: options)
            try session.setActive(true)
        } catch {
            // 少数类别切换要求非激活：停用 → 改 → 激活（引擎的输出被停掉，看门狗随后重启）
            do {
                try? session.setActive(false, options: .notifyOthersOnDeactivation)
                try session.setCategory(category, mode: .default, options: options)
                try session.setActive(true)
            } catch {
                NSLog("mini-game: AVAudioSession 设置失败 \(error)")
            }
        }
    }
}
