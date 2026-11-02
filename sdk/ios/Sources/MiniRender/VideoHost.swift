import AVFoundation
import CoreVideo
import Foundation
import MiniRenderFFI
import QuartzCore

/// 同层渲染的 `<video>` / `<live-player>`：AVPlayer 解码，帧（NV12）交给引擎画进页面（`mr_video_frame_yuv`），
/// 控制条、全屏由引擎画。与 Android `VideoHost.kt` 同一套约定（`videos` / `videoCommand` 事件，回传 play / pause /
/// waiting / timeupdate / loadedmetadata / seekcomplete / ended / error）。
///
/// 取帧在后台队列上按 60Hz 问 `AVPlayerItemVideoOutput` 有没有新帧，有就交（YUV → RGBA 在引擎里、交帧的这条线程上做），
/// 交完让引擎线程出一帧（只重画那个视频框）。
final class VideoHost {
    private let run: (@escaping (OpaquePointer) -> Void) -> Void
    private let queue = DispatchQueue(label: "mini-video", qos: .userInitiated)
    private var players: [String: Player] = [:]
    private var timer: DispatchSourceTimer?

    init(run: @escaping (@escaping (OpaquePointer) -> Void) -> Void) {
        self.run = run
    }

    fileprivate func emit(_ key: String, _ name: String, _ detail: [String: Any] = [:]) {
        let json = (try? JSONSerialization.data(withJSONObject: detail)).flatMap { String(data: $0, encoding: .utf8) } ?? "{}"
        run { _ = mr_app_native_view_event($0, key, name, json) }
    }

    fileprivate func frameReady() { run { _ in } }

    /// 主线程：`videos` 事件
    func sync(_ ev: [String: Any]) {
        var seen = Set<String>()
        for v in ev["views"] as? [[String: Any]] ?? [] {
            let key = v["key"] as? String ?? ""
            guard !key.isEmpty else { continue }
            seen.insert(key)
            let url = v["url"] as? String ?? ""
            let attrs = v["attrs"] as? [String: Any] ?? [:]
            if let p = players[key], p.url == url {
                p.update(attrs)
            } else {
                players[key]?.release()
                players[key] = Player(host: self, key: key, url: url, attrs: attrs)
            }
        }
        for k in Array(players.keys) where !seen.contains(k) { players.removeValue(forKey: k)?.release() }
        updateTimer()
    }

    /// 主线程：`videoCommand` 事件
    func command(_ ev: [String: Any]) {
        players[ev["key"] as? String ?? ""]?.command(ev)
    }

    /// 进后台：正在播的都停下（与微信一致）
    func pauseAll() { players.values.forEach { $0.pause(emitEvent: true) } }

    func releaseAll() {
        players.values.forEach { $0.release() }
        players.removeAll()
        updateTimer()
    }

    private func updateTimer() {
        if players.isEmpty {
            timer?.cancel()
            timer = nil
            return
        }
        guard timer == nil else { return }
        let t = DispatchSource.makeTimerSource(queue: queue)
        t.schedule(deadline: .now(), repeating: .milliseconds(16), leeway: .milliseconds(2))
        let list = { [weak self] () -> [Player] in
            var out: [Player] = []
            DispatchQueue.main.sync { out = self.map { Array($0.players.values) } ?? [] }
            return out
        }
        var cached: [Player] = []
        var n = 0
        t.setEventHandler { [weak self] in
            // 播放器表在主线程上改：每 30 次（约半秒）同步一次名单，其余时候用手里那份
            if n % 30 == 0 { cached = list() }
            n += 1
            var any = false
            for p in cached where p.pull() { any = true }
            if any { self?.frameReady() }
        }
        t.resume()
        timer = t
    }

    final class Player {
        let key: String
        let url: String
        private weak var host: VideoHost?
        private let player = AVPlayer()
        private let output = AVPlayerItemVideoOutput(pixelBufferAttributes: [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange,
        ])
        private var item: AVPlayerItem?
        private var attrs: [String: Any]
        private var obs: [NSKeyValueObservation] = []
        private var timeObs: Any?
        private var endObs: NSObjectProtocol?
        private var ready = false
        private var wantPlay = false
        private var playing = false
        private var released = false
        private var quietSeek = false

        init(host: VideoHost, key: String, url: String, attrs: [String: Any]) {
            self.host = host
            self.key = key
            self.url = url
            self.attrs = attrs
            let u = url.hasPrefix("/") ? URL(fileURLWithPath: url) : URL(string: url)
            guard let u else {
                host.emit(key, "error", ["errMsg": "地址不对：\(url)"])
                return
            }
            // 有声音：静音键打开时也出声（与微信的视频一致）；别的会话（小游戏）已经设过类别的不动
            let s = AVAudioSession.sharedInstance()
            if s.category == .soloAmbient { try? s.setCategory(.playback, options: [.mixWithOthers]) }
            let item = AVPlayerItem(url: u)
            item.add(output)
            self.item = item
            player.replaceCurrentItem(with: item)
            player.isMuted = flag("muted", false)
            wantPlay = flag("autoplay", false)
            obs.append(item.observe(\.status, options: [.new]) { [weak self] it, _ in
                DispatchQueue.main.async { self?.onStatus(it) }
            })
            obs.append(player.observe(\.timeControlStatus, options: [.new]) { [weak self] p, _ in
                DispatchQueue.main.async { self?.onControl(p.timeControlStatus) }
            })
            timeObs = player.addPeriodicTimeObserver(forInterval: CMTime(value: 1, timescale: 4), queue: .main) { [weak self] _ in
                guard let self, self.playing else { return }
                self.host?.emit(self.key, "timeupdate", self.time())
            }
            endObs = NotificationCenter.default.addObserver(forName: .AVPlayerItemDidPlayToEndTime, object: item, queue: .main) { [weak self] _ in
                self?.onEnded()
            }
        }

        private func flag(_ k: String, _ def: Bool) -> Bool {
            guard let v = attrs[k] else { return def }
            return !["false", "{{false}}", "0"].contains("\(v)".trimmingCharacters(in: .whitespaces))
        }

        private func time() -> [String: Any] {
            let d = item?.duration.seconds ?? 0
            return ["currentTime": player.currentTime().seconds, "duration": d.isFinite ? d : 0]
        }

        func update(_ a: [String: Any]) {
            attrs = a
            player.isMuted = flag("muted", false)
        }

        private func onStatus(_ it: AVPlayerItem) {
            guard !released else { return }
            switch it.status {
            case .readyToPlay where !ready:
                ready = true
                let size = it.presentationSize
                let d = it.duration.seconds
                host?.emit(key, "loadedmetadata", ["width": Int(size.width), "height": Int(size.height), "duration": d.isFinite ? d : 0])
                if let t = attrs["initial-time"].flatMap({ Double("\($0)") }), t > 0 {
                    quietSeek = true
                    player.seek(to: CMTime(seconds: t, preferredTimescale: 600))
                }
                if wantPlay { play() }
            case .failed:
                host?.emit(key, "error", ["errMsg": it.error?.localizedDescription ?? "播放失败"])
            default: break
            }
        }

        private func onControl(_ s: AVPlayer.TimeControlStatus) {
            guard !released else { return }
            switch s {
            case .playing:
                if !playing { playing = true }
                host?.emit(key, "play")
            case .waitingToPlayAtSpecifiedRate:
                if player.reasonForWaitingToPlay == .toMinimizeStalls || player.reasonForWaitingToPlay == .evaluatingBufferingRate { host?.emit(key, "waiting") }
            default: break
            }
        }

        private func onEnded() {
            if flag("loop", false) {
                player.seek(to: .zero)
                player.play()
                return
            }
            playing = false
            host?.emit(key, "ended")
        }

        func play() {
            guard ready else { wantPlay = true; return }
            player.play()
        }

        func pause(emitEvent: Bool) {
            wantPlay = false
            guard playing else { return }
            player.pause()
            playing = false
            if emitEvent { host?.emit(key, "pause") }
        }

        func command(_ ev: [String: Any]) {
            switch ev["command"] as? String ?? "" {
            case "play": play()
            case "pause": pause(emitEvent: true)
            case "stop":
                pause(emitEvent: true)
                quietSeek = true
                player.seek(to: .zero)
            case "seek":
                let pos = (ev["position"] as? NSNumber)?.doubleValue ?? 0
                player.seek(to: CMTime(seconds: pos, preferredTimescale: 600)) { [weak self] _ in
                    DispatchQueue.main.async {
                        guard let self else { return }
                        self.host?.emit(self.key, "seekcomplete", ["position": pos])
                    }
                }
            case "playbackRate":
                let r = (ev["rate"] as? NSNumber)?.floatValue ?? 1
                if playing { player.rate = r }
            default: break
            }
        }

        /// 取帧队列：有新帧就交给引擎。返回交了没有
        func pull() -> Bool {
            if released { return false }
            let t = output.itemTime(forHostTime: CACurrentMediaTime())
            guard output.hasNewPixelBuffer(forItemTime: t),
                  let pb = output.copyPixelBuffer(forItemTime: t, itemTimeForDisplay: nil) else { return false }
            CVPixelBufferLockBaseAddress(pb, .readOnly)
            defer { CVPixelBufferUnlockBaseAddress(pb, .readOnly) }
            guard CVPixelBufferGetPlaneCount(pb) == 2,
                  let y = CVPixelBufferGetBaseAddressOfPlane(pb, 0)?.assumingMemoryBound(to: UInt8.self),
                  let uv = CVPixelBufferGetBaseAddressOfPlane(pb, 1)?.assumingMemoryBound(to: UInt8.self) else { return false }
            let w = CVPixelBufferGetWidth(pb), h = CVPixelBufferGetHeight(pb)
            // NV12：U / V 交错在一个平面里（U 在前），pixel stride 2
            return mr_video_frame_yuv(key, UInt32(w), UInt32(h), y, UInt32(CVPixelBufferGetBytesPerRowOfPlane(pb, 0)),
                                      uv, uv + 1, UInt32(CVPixelBufferGetBytesPerRowOfPlane(pb, 1)), 2) == 1
        }

        func release() {
            released = true
            obs.forEach { $0.invalidate() }
            if let t = timeObs { player.removeTimeObserver(t) }
            if let e = endObs { NotificationCenter.default.removeObserver(e) }
            player.pause()
            player.replaceCurrentItem(with: nil)
        }
    }
}
