// 小游戏的 iOS 音频会话（game-doc/04 G5.5；引擎侧见 src/game/audio/session.rs，Android 对应 GameAudioSession.kt）。
//
// - `wx.setInnerAudioOption`（引擎发 {"type":"audioSession","mixWithOther","obeyMuteSwitch","speakerOn"}）→ AVAudioSession：
//   obeyMuteSwitch = true → `.ambient`（静音键静音，天然与别的 App 混音）；false → `.playback`（静音键不管），
//   此时 mixWithOther = true 加 `.mixWithOthers`，false 不加（别的 App 的音乐停下）；
//   speakerOn 只在 `.playAndRecord` 类别下有意义（`.defaultToSpeaker`），游戏不录音时不用这个类别，只记下来；
// - 打断：`AVAudioSession.interruptionNotification` 的 `.began` / `.ended` → `mr_game_audio_interruption(1 / 0)`；
//   引擎暂停全部音频后派发 wx.onAudioInterruptionBegin / End，**不自动续播**（不看 `.shouldResume`，由游戏自己 play）；
// - 类别要在会话**非激活**时改（06 §22）：先 setActive(false, .notifyOthersOnDeactivation) 再改、再激活。
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
            try? session.setActive(false, options: .notifyOthersOnDeactivation)
            try session.setCategory(category, mode: .default, options: options)
            try session.setActive(true)
        } catch {
            NSLog("mini-game: AVAudioSession 设置失败 \(error)")
        }
    }
}
