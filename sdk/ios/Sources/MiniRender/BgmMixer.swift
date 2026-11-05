import AVFoundation

/// `<live-pusher>` 的背景音（LivePusherContext.playBGM / pauseBGM / resumeBGM / stopBGM / setBGMVolume；与 Android `BgmPlayer.kt` 对照）：
/// - **本地放**：`AVAudioPlayer`（主播自己听得到），进度 / 放完都以它为准；
/// - **混进推流**：同一个文件另开一个 `AVAudioFile` 按播放器的 `currentTime` 跟着读，换成麦克风那一段的格式加上去（[mix]，
///   编码队列上调）。跟播放器对时（偏了 0.1s 以上就跳过去），暂停时不混。
///
/// 网络地址先下载到临时目录再放（AVAudioPlayer 只认本地文件）。外放时麦克风会把喇叭里的背景音再录进去一遍（没有回声消除），
/// 戴耳机推流没有这个问题。事件：`onStart` / 每 500ms `onProgress(毫秒, 毫秒)` / `onComplete` / `onError`（errCode 10003）。
final class BgmMixer: NSObject, AVAudioPlayerDelegate {
    var onStart: (() -> Void)?
    var onProgress: ((Int, Int) -> Void)?
    var onComplete: (() -> Void)?
    var onError: ((String) -> Void)?

    private var player: AVAudioPlayer?
    private var timer: Timer?
    private let lock = NSLock()
    private var file: AVAudioFile?
    private var conv: AVAudioConverter?
    private var convTo: AVAudioFormat?
    private var inBuf: AVAudioPCMBuffer?
    private var outBuf: AVAudioPCMBuffer?
    private var gen = 0
    var volume: Float = 1 {
        didSet {
            volume = min(max(volume, 0), 1)
            player?.volume = volume
        }
    }

    var playing: Bool { player?.isPlaying == true }

    /// 换一首（本地路径或 http(s) 地址）。主线程上调
    func play(_ source: String) {
        stop()
        gen += 1
        let g = gen
        if source.hasPrefix("http://") || source.hasPrefix("https://"), let url = URL(string: source) {
            URLSession.shared.downloadTask(with: url) { [weak self] tmp, _, err in
                guard let tmp else { DispatchQueue.main.async { self?.onError?(err?.localizedDescription ?? "下载失败") }; return }
                let local = FileManager.default.temporaryDirectory.appendingPathComponent("bgm-\(UUID().uuidString)-\(url.lastPathComponent)")
                try? FileManager.default.moveItem(at: tmp, to: local)
                DispatchQueue.main.async { if self?.gen == g { self?.start(local) } }
            }.resume()
            return
        }
        start(URL(fileURLWithPath: source))
    }

    private func start(_ url: URL) {
        do {
            let s = AVAudioSession.sharedInstance()
            // 推流时会话里有麦克风：要边录边放
            try? s.setCategory(.playAndRecord, options: [.defaultToSpeaker, .mixWithOthers, .allowBluetoothA2DP])
            try? s.setActive(true)
            let p = try AVAudioPlayer(contentsOf: url)
            p.delegate = self
            p.volume = volume
            p.prepareToPlay()
            let f = try AVAudioFile(forReading: url)
            lock.lock()
            file = f
            conv = nil
            lock.unlock()
            player = p
            p.play()
            onStart?()
            timer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
                guard let self, let p = self.player, p.isPlaying else { return }
                self.onProgress?(Int(p.currentTime * 1000), Int(p.duration * 1000))
            }
        } catch {
            onError?("\(error)")
        }
    }

    func pause() { player?.pause() }
    func resume() { player?.play() }

    func stop() {
        gen += 1
        timer?.invalidate()
        timer = nil
        player?.stop()
        player = nil
        lock.lock()
        file = nil
        conv = nil
        lock.unlock()
    }

    func audioPlayerDidFinishPlaying(_ p: AVAudioPlayer, successfully flag: Bool) {
        let d = Int(p.duration * 1000)
        onProgress?(d, d)
        timer?.invalidate()
        timer = nil
        onComplete?()
    }

    /// 麦克风那一段 [pcm] 上加同样多的背景音（编码队列上调）。没在放 / 暂停时什么也不做
    func mix(into pcm: AVAudioPCMBuffer) {
        guard let p = player, p.isPlaying else { return }
        let now = p.currentTime
        lock.lock()
        defer { lock.unlock() }
        guard let f = file else { return }
        let fmt = pcm.format
        let frames = AVAudioFrameCount(pcm.frameLength)
        if conv == nil || convTo != fmt {
            // 换成单声道 Float32、麦克风的采样率：加的时候再按麦克风的样本格式写回
            guard let mono = AVAudioFormat(commonFormat: .pcmFormatFloat32, sampleRate: fmt.sampleRate, channels: 1, interleaved: false),
                  let c = AVAudioConverter(from: f.processingFormat, to: mono) else { return }
            conv = c
            convTo = fmt
        }
        guard let c = conv else { return }
        // 跟播放器对时：偏了 0.1s 以上（暂停过、拖过）就跳到播放器的位置
        let want = AVAudioFramePosition(now * f.processingFormat.sampleRate)
        if abs(f.framePosition - want) > AVAudioFramePosition(f.processingFormat.sampleRate * 0.1) {
            f.framePosition = min(max(0, want), f.length)
            c.reset()
        }
        let need = AVAudioFrameCount(ceil(Double(frames) * f.processingFormat.sampleRate / fmt.sampleRate)) + 8
        if inBuf == nil || inBuf!.frameCapacity < need || inBuf!.format != f.processingFormat {
            inBuf = AVAudioPCMBuffer(pcmFormat: f.processingFormat, frameCapacity: need)
        }
        if outBuf == nil || outBuf!.frameCapacity < frames || outBuf!.format != c.outputFormat {
            outBuf = AVAudioPCMBuffer(pcmFormat: c.outputFormat, frameCapacity: frames)
        }
        guard let ib = inBuf, let ob = outBuf else { return }
        ob.frameLength = 0
        var err: NSError?
        c.convert(to: ob, error: &err) { n, status in
            ib.frameLength = 0
            if (try? f.read(into: ib, frameCount: min(n, ib.frameCapacity))) == nil || ib.frameLength == 0 {
                status.pointee = .endOfStream
                return nil
            }
            status.pointee = .haveData
            return ib
        }
        guard let src = ob.floatChannelData?[0] else { return }
        let n = Int(min(ob.frameLength, frames))
        let g = volume
        let stride = fmt.isInterleaved ? Int(fmt.channelCount) : 1
        let chans = Int(fmt.channelCount)
        if let ch = pcm.int16ChannelData {
            for k in 0..<n {
                let add = src[k] * g * 32767
                for cIdx in 0..<(fmt.isInterleaved ? chans : 1) {
                    let i = k * stride + cIdx
                    ch[0][i] = Int16(max(-32768, min(32767, Float(ch[0][i]) + add)))
                }
                if !fmt.isInterleaved {
                    for cIdx in 1..<max(chans, 1) { ch[cIdx][k] = Int16(max(-32768, min(32767, Float(ch[cIdx][k]) + add))) }
                }
            }
        } else if let ch = pcm.floatChannelData {
            for k in 0..<n {
                let add = src[k] * g
                if fmt.isInterleaved {
                    for cIdx in 0..<chans { ch[0][k * stride + cIdx] = max(-1, min(1, ch[0][k * stride + cIdx] + add)) }
                } else {
                    for cIdx in 0..<chans { ch[cIdx][k] = max(-1, min(1, ch[cIdx][k] + add)) }
                }
            }
        }
    }
}
