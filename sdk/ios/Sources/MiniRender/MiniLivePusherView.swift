import AVFoundation
import UIKit

/// SDK 自带的 `<live-pusher>`（与 Android `MiniLivePusherView.kt` 同一组属性、状态码与 LivePusherContext 方法）：
/// 相机预览同层渲染（[CameraCapture]），推流 VideoToolbox H.264 + AAC（[LiveEncoder]）经 RTMP（[RtmpPublisher]）。
/// 美颜 / 美白 / 滤镜走 Core Image（[FrameEffects]，预览与推流同一份）；背景音 playBGM / pauseBGM / resumeBGM / stopBGM / setBGMVolume
/// 本地放 + 混进推流（[BgmMixer]，`bgmstart` / `bgmprogress {progress, duration}`（毫秒）/ `bgmcomplete`，打不开 `error 10003`）；
/// 小窗（`picture-in-picture-mode`）是引擎画的（host/engine_pip.rs），这里照常交帧、推流不断。
/// 变声 / 混响 / 自定义渲染没做；`remote-mirror` 在 iOS 上没接（推出去的画面与预览同一份镜像）。
public final class MiniLivePusherView: MiniSameLayerView {
    private var cam: CameraCapture?
    private var front = true
    private var attached = false
    private var preview = false
    private var paused = false
    private var torch = false
    private var enc: LiveEncoder?
    private var pub: RtmpPublisher?
    private var bgm: BgmMixer?
    private var bgmWasPlaying = false

    public override var contextClass: String { "LivePusherContext" }
    public override var contextMethods: [String] {
        ["start", "stop", "pause", "resume", "startPreview", "stopPreview", "switchCamera", "toggleTorch", "snapshot", "setZoom", "getMaxZoom", "setMICVolume",
         "playBGM", "pauseBGM", "resumeBGM", "stopBGM", "setBGMVolume"]
    }

    private func flag(_ k: String, _ d: Bool) -> Bool {
        guard let v = attrs[k] else { return d }
        let s = "\(v)".trimmingCharacters(in: .whitespaces)
        return !(s == "false" || s == "0" || s.isEmpty)
    }
    private func str(_ k: String) -> String { (attrs[k] as? String ?? "").trimmingCharacters(in: .whitespaces) }

    private func state(_ code: Int, _ msg: String) {
        DispatchQueue.main.async { self.emit("statechange", ["code": code, "message": msg]) }
    }

    public override func draw(in ctx: CGContext, pixelWidth: Int, pixelHeight: Int, scale: CGFloat) {
        ctx.setFillColor(UIColor.black.cgColor)
        ctx.fill(CGRect(x: 0, y: 0, width: CGFloat(pixelWidth) / scale, height: CGFloat(pixelHeight) / scale))
    }

    public override func onBox(width: Int, height: Int) {
        cam?.boxW = width
        cam?.boxH = height
    }

    private var localMirrored: Bool {
        switch str("local-mirror") {
        case "enable": return true
        case "disable": return false
        default: return front
        }
    }

    public override func onAttrs(_ attrs: [String: Any]) {
        invalidate()
        let pos = str("device-position")
        let f = pos.isEmpty || pos == "front"
        if !attached {
            attached = true
            front = f
            if flag("enable-camera", true) { startPreview(nil) }
            if flag("autopush", false) && !str("url").isEmpty { start(nil) }
            return
        }
        if f != front { switchCamera(nil) }
        cam?.setEffects(FrameEffects.parse(attrs))
        enc?.muted = !flag("enable-mic", true) || flag("muted", false)
    }

    private func startPreview(_ reply: MiniHostReply?) {
        AVCaptureDevice.requestAccess(for: .video) { ok in
            DispatchQueue.main.async { [weak self] in
                guard let self else { return }
                if !ok {
                    self.state(-1301, "打开摄像头失败")
                    self.emit("error", ["errCode": 10001, "errMsg": "用户禁止使用摄像头"])
                    reply?.fail("startPreview:fail auth deny")
                    return
                }
                self.preview = true
                self.openCamera()
                reply?.success()
            }
        }
    }

    private func openCamera() {
        if paused { return }
        let c = cam ?? {
            let c = CameraCapture()
            c.onOpened = { [weak self] _ in self?.state(1003, "打开摄像头成功") }
            c.onFirstFrame = { [weak self] in self?.state(1007, "首帧画面采集完成") }
            c.onError = { [weak self] _, _ in self?.state(-1301, "打开摄像头失败") }
            c.onPreview = { [weak self] p, w, h, s in self?.pushFrame(p, width: w, height: h, stride: s) }
            c.onSample = { [weak self] sb, video in
                guard let self, !self.pausedPush, let e = self.enc else { return }
                if video { e.encode(video: sb) } else { e.encode(audio: sb) }
            }
            cam = c
            return c
        }()
        c.boxW = boxWidth
        c.boxH = boxHeight
        c.setEffects(FrameEffects.parse(attrs))
        c.open(front: front, preset: .hd1280x720, mirrored: localMirrored,
               audio: flag("enable-mic", true) && AVCaptureDevice.authorizationStatus(for: .audio) == .authorized)
    }

    private func pushSize() -> (Int, Int) {
        var (w, h): (Int, Int)
        switch str("mode") {
        case "QVGA": (w, h) = (240, 320)
        case "HVGA": (w, h) = (320, 480)
        case "HD": (w, h) = (540, 960)
        case "FHD": (w, h) = (720, 1280)
        default: (w, h) = (360, 640)
        }
        if str("aspect") == "3:4" { h = w * 4 / 3 }
        if let v = Int(str("video-width")), (16...1920).contains(v) { w = v }
        if let v = Int(str("video-height")), (16...1920).contains(v) { h = v }
        if str("orientation") == "horizontal" { swap(&w, &h) }
        return (w & ~1, h & ~1)
    }

    private func start(_ reply: MiniHostReply?) {
        if pub != nil { reply?.success(); return }
        let url = str("url")
        guard !url.isEmpty else { reply?.fail("start:fail 没有推流地址（url）"); return }
        AVCaptureDevice.requestAccess(for: .audio) { mic in
            DispatchQueue.main.async { [weak self] in
                guard let self else { return }
                if !mic { self.emit("error", ["errCode": 10002, "errMsg": "用户禁止使用录音"]) }
                let (w, h) = self.pushSize()
                let fps = min(max(Int(self.str("fps")) ?? 15, 1), 30)
                let kbps = min(max(Int(self.str("max-bitrate")) ?? 1000, 100), 8000)
                let rate: Double = self.str("audio-quality") == "low" ? 16000 : 48000
                let p = RtmpPublisher(url: url)
                p.videoWidth = w
                p.videoHeight = h
                p.fps = fps
                p.onState = { [weak self] c, m in self?.state(c, m) }
                p.onNetStatus = { [weak self] info in DispatchQueue.main.async { self?.emit("netstatus", ["info": info]) } }
                guard let e = LiveEncoder(width: w, height: h, fps: fps, bitrate: kbps * 1000, sampleRate: rate, publisher: p) else {
                    self.state(-1303, "视频编码失败")
                    reply?.fail("start:fail 编码器打不开")
                    return
                }
                p.onCongested = { [weak e] in e?.requestKeyFrame() }
                e.muted = !self.flag("enable-mic", true) || self.flag("muted", false)
                e.bgm = self.bgm
                self.enc = e
                self.pub = p
                // 有了麦克风权限：带上声音输入重开会话
                if mic, let c = self.cam, !c.hasAudio { c.reopenWithAudio(front: self.front, preset: .hd1280x720, mirrored: self.localMirrored) }
                if self.cam == nil { self.preview = true; self.openCamera() }
                self.state(1008, "编码器启动")
                if mic { self.state(2027, "麦克风启动成功") }
                p.start()
                reply?.success()
            }
        }
    }

    private func stop(_ reply: MiniHostReply?) {
        pub?.stop()
        enc?.stop()
        pub = nil
        enc = nil
        reply?.success()
    }

    private func switchCamera(_ reply: MiniHostReply?) {
        front.toggle()
        if preview { openCamera() }
        enc?.requestKeyFrame()
        reply?.success()
    }

    public override func onContextCall(_ method: String, args: [String: Any], reply: MiniHostReply) -> Bool {
        switch method {
        case "start": start(reply)
        case "stop": stop(reply)
        case "pause": enc?.muted = true; pausedPush = true; reply.success()
        case "resume": enc?.muted = !flag("enable-mic", true) || flag("muted", false); pausedPush = false; enc?.requestKeyFrame(); reply.success()
        case "startPreview": startPreview(reply)
        case "stopPreview": preview = false; cam?.close(); reply.success()
        case "switchCamera": switchCamera(reply)
        case "toggleTorch": torch.toggle(); cam?.setTorch(torch); reply.success()
        case "setZoom":
            guard let c = cam else { reply.fail("setZoom:fail 相机还没打开"); return true }
            c.setZoom((args["zoom"] as? NSNumber)?.doubleValue ?? 1) { z in DispatchQueue.main.async { reply.success(["zoom": z]) } }
        case "getMaxZoom": reply.success(["maxZoom": cam?.maxZoom ?? 1])
        case "playBGM": playBgm("\(args["url"] ?? "")", reply)
        case "pauseBGM": bgm?.pause(); reply.success()
        case "resumeBGM": bgm?.resume(); reply.success()
        case "stopBGM": bgm?.stop(); reply.success()
        case "setBGMVolume": mixer().volume = Float((args["volume"] as? NSNumber)?.doubleValue ?? 1); reply.success()
        case "setMICVolume": enc?.volume = Float((args["volume"] as? NSNumber)?.doubleValue ?? 1); reply.success()
        case "snapshot":
            guard let c = cam else { reply.fail("snapshot:fail 相机还没打开"); return true }
            let side = (args["quality"] as? String) == "compressed" ? 640 : 1280
            c.takePhoto(side: side, file: MiniCameraView.newFile("png")) { url, err in
                DispatchQueue.main.async {
                    guard let url, let img = UIImage(contentsOfFile: url.path) else { return reply.fail("snapshot:fail \(err ?? "")") }
                    reply.success(["tempImagePath": url.absoluteString, "width": Int(img.size.width * img.scale), "height": Int(img.size.height * img.scale)])
                }
            }
        default: return false
        }
        return true
    }

    private func mixer() -> BgmMixer {
        if let b = bgm { return b }
        let b = BgmMixer()
        b.onStart = { [weak self] in self?.emit("bgmstart", [:]) }
        b.onProgress = { [weak self] p, d in self?.emit("bgmprogress", ["progress": p, "duration": d]) }
        b.onComplete = { [weak self] in self?.emit("bgmcomplete", [:]) }
        b.onError = { [weak self] m in self?.emit("error", ["errCode": 10003, "errMsg": "背景音资源（BGM）加载失败：\(m)"]) }
        bgm = b
        enc?.bgm = b
        return b
    }

    /// playBGM：网络地址下载后放，小程序里的路径（包内 / wxfile://）换成本地文件
    private func playBgm(_ url: String, _ reply: MiniHostReply) {
        let u = url.trimmingCharacters(in: .whitespaces)
        let src: String?
        if u.hasPrefix("http://") || u.hasPrefix("https://") { src = u }
        else if u.hasPrefix("file://") { src = URL(string: u)?.path }
        else { src = resolvePath(u)?.path }
        guard let s = src else {
            emit("error", ["errCode": 10003, "errMsg": "背景音资源（BGM）加载失败"])
            reply.fail("playBGM:fail 找不到背景音 \(u)")
            return
        }
        mixer().play(s)
        reply.success()
    }

    /// LivePusherContext.pause：画面与声音都不再交给编码器（连接留着）
    private var pausedPush = false

    public override func onPause() {
        guard !paused else { return }
        paused = true
        bgmWasPlaying = bgm?.playing == true
        if bgmWasPlaying { bgm?.pause() }
        if pub != nil { state(5000, "推流：被挂起，小程序或微信被退后台时挂起推流") }
        cam?.close()
    }

    public override func onResume() {
        guard paused else { return }
        paused = false
        if bgmWasPlaying { bgm?.resume() }
        if preview { openCamera() }
        enc?.requestKeyFrame()
    }

    public override func onDetach() {
        bgm?.stop()
        bgm = nil
        enc?.bgm = nil
        stop(nil)
        cam?.close()
        cam = nil
    }
}
