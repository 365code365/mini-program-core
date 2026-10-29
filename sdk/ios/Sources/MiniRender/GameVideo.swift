import AVFoundation
import UIKit

/// 小游戏的 `wx.createVideo`（与 Android 的 GameVideo.kt 对应；引擎侧见 `src/js/prelude/game-proxies.js`）。
///
/// 引擎发宿主事件 `{type:'video', op, id, props?, time?, direction?}`，这里每个 id 一个 AVPlayerLayer 视图，
/// 按 props 的 x / y / width / height（逻辑像素 = UIKit 的点）摆在游戏画面上面。事件经 `contextEvent` 推回去：
/// play / pause / ended / waiting / error / progress {buffered}，播放中每 250ms 一次 timeUpdate {position, duration}（秒）。
///
/// 取舍（与 Android 一致）：`underGameView` 一律盖在游戏上面（游戏图层不透明）；`requestFullScreen` 铺满宿主视图、不转屏；
/// `controls` 不画系统控件（AVPlayerLayer 没有，游戏的视频多半自己画按钮）。`objectFit` 三种都支持（contain / cover / fill）。
/// 所有方法在主线程调；[emit] 由调用方投递到引擎线程。
final class GameVideo {
    private weak var host: UIView?
    private let dir: String
    private let emit: (_ name: String, _ json: String) -> Void
    private var videos: [String: Item] = [:]

    private final class Item {
        let id: String
        let view = PlayerView()
        let player = AVPlayer()
        var props: [String: Any] = [:]
        var src = ""
        var full = false
        var want = false
        var observers: [NSObjectProtocol] = []
        var timeObserver: Any?
        var kvo: [NSKeyValueObservation] = []
        init(id: String) { self.id = id }
    }

    final class PlayerView: UIView {
        override class var layerClass: AnyClass { AVPlayerLayer.self }
        var playerLayer: AVPlayerLayer { layer as! AVPlayerLayer }
    }

    init(host: UIView, dir: String, emit: @escaping (_ name: String, _ json: String) -> Void) {
        self.host = host
        self.dir = dir
        self.emit = emit
    }

    func handle(_ o: [String: Any]) {
        let id = "\(o["id"] ?? "")"
        let op = o["op"] as? String ?? ""
        if op == "create" { return create(id, o["props"] as? [String: Any] ?? [:]) }
        guard let v = videos[id] else { return }
        switch op {
        case "update": apply(v, o["props"] as? [String: Any] ?? [:])
        case "play": play(v)
        case "pause":
            v.want = false
            v.player.pause()
            send(id, "pause")
        case "stop":
            v.want = false
            v.player.pause()
            v.player.seek(to: .zero)
            send(id, "pause")
        case "seek": v.player.seek(to: CMTime(seconds: (o["time"] as? NSNumber)?.doubleValue ?? 0, preferredTimescale: 600))
        case "requestFullScreen":
            v.full = true
            layout(v)
        case "exitFullScreen":
            v.full = false
            layout(v)
        case "destroy": destroy(v)
        default: break
        }
    }

    func release() { videos.values.forEach(destroy) }

    /// 宿主视图改了尺寸（转屏）：全屏的视频跟着
    func relayout() { videos.values.forEach(layout) }

    private func send(_ id: String, _ event: String, _ data: [String: Any] = [:]) {
        emit("contextEvent", GameSensors.json(["context": "Video", "contextId": id, "event": event, "data": data]))
    }

    private func create(_ id: String, _ p: [String: Any]) {
        let v = Item(id: id)
        videos[id] = v
        v.view.playerLayer.player = v.player
        v.view.backgroundColor = .clear
        host?.addSubview(v.view)
        let nc = NotificationCenter.default
        v.observers.append(nc.addObserver(forName: .AVPlayerItemDidPlayToEndTime, object: nil, queue: .main) { [weak self, weak v] n in
            guard let self, let v, (n.object as? AVPlayerItem) === v.player.currentItem else { return }
            if v.props["loop"] as? Bool ?? false {
                v.player.seek(to: .zero)
                v.player.play()
            } else {
                v.want = false
                self.send(v.id, "ended")
            }
        })
        v.timeObserver = v.player.addPeriodicTimeObserver(forInterval: CMTime(value: 1, timescale: 4), queue: .main) { [weak self, weak v] t in
            guard let self, let v, v.want, v.player.rate > 0 else { return }
            let d = v.player.currentItem?.duration.seconds ?? 0
            self.send(v.id, "timeUpdate", ["position": t.seconds, "duration": d.isFinite ? d : 0])
        }
        v.kvo.append(v.player.observe(\.timeControlStatus, options: [.new]) { [weak self, weak v] p, _ in
            DispatchQueue.main.async {
                guard let self, let v, v.want else { return }
                if p.timeControlStatus == .waitingToPlayAtSpecifiedRate { self.send(v.id, "waiting") }
            }
        })
        apply(v, p)
    }

    private func apply(_ v: Item, _ p: [String: Any]) {
        v.props = p
        layout(v)
        switch p["objectFit"] as? String {
        case "cover": v.view.playerLayer.videoGravity = .resizeAspectFill
        case "fill": v.view.playerLayer.videoGravity = .resize
        default: v.view.playerLayer.videoGravity = .resizeAspect
        }
        let src = p["src"] as? String ?? ""
        if !src.isEmpty, src != v.src, let url = url(src) {
            v.src = src
            let item = AVPlayerItem(url: url)
            v.kvo.append(item.observe(\.status, options: [.new]) { [weak self, weak v] it, _ in
                DispatchQueue.main.async {
                    guard let self, let v else { return }
                    if it.status == .failed { self.send(v.id, "error", ["errMsg": "video error \(it.error?.localizedDescription ?? "")"]) }
                    if it.status == .readyToPlay {
                        let t = (v.props["initialTime"] as? NSNumber)?.doubleValue ?? 0
                        if t > 0 { v.player.seek(to: CMTime(seconds: t, preferredTimescale: 600)) }
                    }
                }
            })
            v.kvo.append(item.observe(\.loadedTimeRanges, options: [.new]) { [weak self, weak v] it, _ in
                guard let r = it.loadedTimeRanges.last?.timeRangeValue, it.duration.seconds > 0 else { return }
                let pct = Int((r.end.seconds / it.duration.seconds * 100).rounded())
                DispatchQueue.main.async { if let self, let v { self.send(v.id, "progress", ["buffered": pct]) } }
            })
            v.player.replaceCurrentItem(with: item)
            if p["autoplay"] as? Bool ?? false { play(v) }
        }
        v.player.isMuted = p["muted"] as? Bool ?? false
        if v.want, let rate = (p["playbackRate"] as? NSNumber)?.floatValue, rate > 0 { v.player.rate = rate }
    }

    private func layout(_ v: Item) {
        guard let host else { return }
        func n(_ k: String, _ d: Double) -> CGFloat { CGFloat((v.props[k] as? NSNumber)?.doubleValue ?? d) }
        v.view.frame = v.full ? host.bounds : CGRect(x: n("x", 0), y: n("y", 0), width: n("width", 300), height: n("height", 150))
        host.bringSubviewToFront(v.view)
    }

    private func play(_ v: Item) {
        v.want = true
        let rate = (v.props["playbackRate"] as? NSNumber)?.floatValue ?? 1
        v.player.playImmediately(atRate: rate > 0 ? rate : 1)
        send(v.id, "play")
    }

    private func destroy(_ v: Item) {
        v.want = false
        v.player.pause()
        if let t = v.timeObserver { v.player.removeTimeObserver(t) }
        v.observers.forEach { NotificationCenter.default.removeObserver($0) }
        v.kvo.forEach { $0.invalidate() }
        v.player.replaceCurrentItem(with: nil)
        v.view.removeFromSuperview()
        videos.removeValue(forKey: v.id)
    }

    /// 网络地址原样；其余按游戏包内的相对路径
    private func url(_ src: String) -> URL? {
        if src.hasPrefix("http://") || src.hasPrefix("https://") || src.hasPrefix("file://") { return URL(string: src) }
        let rel = src.hasPrefix("/") ? String(src.dropFirst()) : src
        return URL(fileURLWithPath: dir).appendingPathComponent(rel)
    }
}
