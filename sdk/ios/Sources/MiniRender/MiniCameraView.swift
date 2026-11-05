import AVFoundation
import UIKit

/// SDK 自带的 `<camera>` 与 `<voip-room mode="camera">`（与 Android `MiniCameraView.kt` 同一组行为）：
/// AVCaptureSession 的画面按框裁好交给引擎（[CameraCapture]），同层渲染进页面。
/// - 事件：`initdone {maxZoom}`、`error {errMsg}`、`stop`（进后台）；
/// - CameraContext：`takePhoto`（PNG，官方：iOS 是 png）、`startRecord` / `stopRecord`（AVAssetWriter，H.264 + AAC 的 mp4，timeout 到点推 `recordTimeout`）、`setZoom`；
/// - `onCameraFrame` 不经宿主：引擎从交上去的画面里取。
/// - `<voip-room mode="video" openid>`（别人的画面）：向宿主的 [MiniVoip.video] 要这个人的画面；没接的宿主只画黑底。
/// 镜像：前置预览、照片、录像都是镜像的（`selfieMirror: false` 在 iOS 上没接，按镜像出）。
public final class MiniCameraView: MiniSameLayerView {
    private let tag: String
    private var cam: CameraCapture?
    private var front: Bool
    private var preset: AVCaptureSession.Preset = .hd1280x720
    private var flash = "auto"
    private var wantOpen = false
    private var paused = false

    public init(tag: String = "camera") {
        self.tag = tag
        front = tag == "voip-room"
        super.init()
    }

    public override var contextClass: String { tag == "camera" ? "CameraContext" : "" }
    public override var contextMethods: [String] { tag == "camera" ? ["takePhoto", "setZoom", "startRecord", "stopRecord"] : [] }
    private var selfOnly: Bool { tag == "voip-room" && (attrs["mode"] as? String ?? "").trimmingCharacters(in: .whitespaces) == "video" }

    public override func draw(in ctx: CGContext, pixelWidth: Int, pixelHeight: Int, scale: CGFloat) {
        ctx.setFillColor(UIColor.black.cgColor)
        ctx.fill(CGRect(x: 0, y: 0, width: CGFloat(pixelWidth) / scale, height: CGFloat(pixelHeight) / scale))
    }

    public override func onBox(width: Int, height: Int) {
        cam?.boxW = width
        cam?.boxH = height
        if selfOnly { attachRemote() }
    }

    /// voip-room 别人那一格：正在接的那一路（openid, 宽, 高）与停止用的闭包
    private var remote: (String, Int, Int)?
    private var remoteStop: (() -> Void)?

    private func attachRemote() {
        guard let src = MiniVoip.video else { return }
        let id = "\(attrs["openid"] ?? "")".trimmingCharacters(in: .whitespaces)
        if let r = remote, r == (id, boxWidth, boxHeight) { return }
        detachRemote()
        guard !id.isEmpty, boxWidth > 0, boxHeight > 0 else { return }
        remote = (id, boxWidth, boxHeight)
        remoteStop = src(id, boxWidth, boxHeight) { [weak self] p, w, h, stride in self?.pushFrame(p, width: w, height: h, stride: stride) }
    }

    private func detachRemote() {
        remoteStop?()
        remoteStop = nil
        remote = nil
    }

    public override func onAttrs(_ attrs: [String: Any]) {
        invalidate()
        if selfOnly { attachRemote(); return }
        let pos = (attrs["device-position"] as? String ?? "").trimmingCharacters(in: .whitespaces)
        let f = pos.isEmpty ? tag == "voip-room" : pos == "front"
        let fl = (attrs["flash"] as? String ?? "").trimmingCharacters(in: .whitespaces)
        if !wantOpen {
            wantOpen = true
            front = f
            flash = fl.isEmpty ? "auto" : fl
            switch (attrs["resolution"] as? String ?? "").trimmingCharacters(in: .whitespaces) {
            case "low": preset = .vga640x480
            case "high": preset = .hd1920x1080
            default: preset = .hd1280x720
            }
            AVCaptureDevice.requestAccess(for: .video) { ok in
                DispatchQueue.main.async { [weak self] in
                    guard let self else { return }
                    if ok { self.open() } else { self.emit("error", ["errMsg": "\(self.tag):fail auth deny"]) }
                }
            }
            return
        }
        if f != front { front = f; open() }
        if !fl.isEmpty && fl != flash { flash = fl; cam?.setFlash(fl) }
    }

    private func open() {
        if paused { return }
        let c = cam ?? makeCam()
        c.boxW = boxWidth
        c.boxH = boxHeight
        c.setFlash(flash)
        c.open(front: front, preset: preset, mirrored: front, audio: AVCaptureDevice.authorizationStatus(for: .audio) == .authorized)
    }

    private func makeCam() -> CameraCapture {
        let c = CameraCapture()
        c.onOpened = { [weak self] z in DispatchQueue.main.async { if self?.tag == "camera" { self?.emit("initdone", ["maxZoom": z]) } } }
        c.onError = { [weak self] msg, _ in DispatchQueue.main.async { self?.emit("error", ["errMsg": "\(self?.tag ?? "camera"):fail \(msg)"]) } }
        c.onPreview = { [weak self] p, w, h, s in self?.pushFrame(p, width: w, height: h, stride: s) }
        c.onSample = { [weak self] sb, video in self?.recorder?.append(sb, video: video) }
        cam = c
        return c
    }

    public override func onPause() {
        guard !paused else { return }
        paused = true
        guard let c = cam, c.isOpen else { return }
        if recorder != nil { finishRecord(nil) }
        c.close()
        emit("stop")
    }

    public override func onResume() {
        guard paused else { return }
        paused = false
        if wantOpen && !selfOnly { open() }
    }

    public override func onDetach() {
        detachRemote()
        if recorder != nil { finishRecord(nil) }
        cam?.close()
        cam = nil
    }

    // MARK: CameraContext

    private var recorder: AssetRecorder?
    private var timeout: DispatchWorkItem?
    private var finished: [String: Any]?

    public override func onContextCall(_ method: String, args: [String: Any], reply: MiniHostReply) -> Bool {
        switch method {
        case "takePhoto":
            guard let c = cam else { reply.fail("takePhoto:fail 相机还没打开"); return true }
            // 微信：扫码模式下不让拍照（真机回的就是这句）
            if (attrs["mode"] as? String ?? "").trimmingCharacters(in: .whitespaces) == "scanCode" {
                reply.fail("takePhoto:fail Not allow to invoke takePhoto in 'scanCode' mode.")
                return true
            }
            let side: Int
            switch args["quality"] as? String ?? "normal" {
            case "low": side = 640
            case "high": side = 1920
            case "original": side = 4096
            default: side = 1280
            }
            c.takePhoto(side: side, file: Self.newFile("png")) { url, err in
                DispatchQueue.main.async {
                    if let url { reply.success(["tempImagePath": url.absoluteString]) } else { reply.fail("takePhoto:fail \(err ?? "")") }
                }
            }
        case "setZoom":
            guard let c = cam else { reply.fail("setZoom:fail 相机还没打开"); return true }
            c.setZoom((args["zoom"] as? NSNumber)?.doubleValue ?? 1) { z in DispatchQueue.main.async { reply.success(["zoom": z]) } }
        case "startRecord": startRecord(args, reply)
        case "stopRecord":
            if let f = finished { finished = nil; reply.success(f); return true }
            guard recorder != nil else { reply.fail("stopRecord:fail 没有在录像"); return true }
            finishRecord { res in if let res { reply.success(res) } else { reply.fail("stopRecord:fail 录像没有写出来") } }
        default: return false
        }
        return true
    }

    private func startRecord(_ args: [String: Any], _ reply: MiniHostReply) {
        guard let c = cam, c.isOpen else { return reply.fail("startRecord:fail 相机还没打开") }
        guard recorder == nil else { return reply.fail("startRecord:fail 已经在录像") }
        AVCaptureDevice.requestAccess(for: .audio) { mic in
            DispatchQueue.main.async { [weak self] in
                guard let self, let c = self.cam else { return }
                // 有了麦克风权限要重开会话才带上声音输入（第一次录像时）
                if mic && !c.hasAudio { c.reopenWithAudio(front: self.front, preset: self.preset, mirrored: self.front) }
                let video = Self.newFile("mp4")
                let r = AssetRecorder(url: video, audio: mic)
                self.recorder = r
                let limit = min(max((args["timeout"] as? NSNumber)?.doubleValue ?? 30, 1), 300)
                let item = DispatchWorkItem { [weak self] in
                    self?.finishRecord { res in
                        self?.finished = res
                        self?.dispatchContextEvent(["context": "CameraContext", "contextId": "", "event": "recordTimeout", "data": [:]])
                    }
                }
                self.timeout = item
                DispatchQueue.main.asyncAfter(deadline: .now() + limit, execute: item)
                reply.success()
            }
        }
    }

    private func finishRecord(_ done: (([String: Any]?) -> Void)?) {
        guard let r = recorder else { return }
        recorder = nil
        timeout?.cancel()
        timeout = nil
        let thumb = Self.newFile("png")
        let finish = {
            r.finish { ok in
                DispatchQueue.main.async {
                    let has = FileManager.default.fileExists(atPath: thumb.path)
                    done?(ok ? ["tempVideoPath": r.url.absoluteString, "tempThumbPath": has ? thumb.absoluteString : ""] : nil)
                }
            }
        }
        guard let c = cam, c.isOpen else { return finish() }
        c.takePhoto(side: 480, file: thumb) { _, _ in finish() }
    }

    static func newFile(_ ext: String) -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("minirender-camera", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("cam_\(Int(Date().timeIntervalSince1970 * 1000))_\(Int.random(in: 0..<1_000_000)).\(ext)")
    }
}

/// 录像：采集到的样本原样交给 AVAssetWriter（H.264 + AAC），第一帧画面到了才开写
final class AssetRecorder {
    let url: URL
    private var writer: AVAssetWriter?
    private var vIn: AVAssetWriterInput?
    private var aIn: AVAssetWriterInput?
    private let audio: Bool
    private var started = false
    private var done = false
    private let lock = NSLock()

    init(url: URL, audio: Bool) {
        self.url = url
        self.audio = audio
    }

    func append(_ sb: CMSampleBuffer, video: Bool) {
        lock.lock()
        defer { lock.unlock() }
        if done { return }
        if writer == nil {
            guard video, let fmt = CMSampleBufferGetFormatDescription(sb) else { return }
            let d = CMVideoFormatDescriptionGetDimensions(fmt)
            guard let w = try? AVAssetWriter(outputURL: url, fileType: .mp4) else { done = true; return }
            let vi = AVAssetWriterInput(mediaType: .video, outputSettings: [
                AVVideoCodecKey: AVVideoCodecType.h264, AVVideoWidthKey: Int(d.width), AVVideoHeightKey: Int(d.height),
            ])
            vi.expectsMediaDataInRealTime = true
            if w.canAdd(vi) { w.add(vi) }
            if audio {
                let ai = AVAssetWriterInput(mediaType: .audio, outputSettings: [
                    AVFormatIDKey: kAudioFormatMPEG4AAC, AVNumberOfChannelsKey: 1, AVSampleRateKey: 44100, AVEncoderBitRateKey: 96000,
                ])
                ai.expectsMediaDataInRealTime = true
                if w.canAdd(ai) { w.add(ai); aIn = ai }
            }
            writer = w
            vIn = vi
        }
        guard let w = writer else { return }
        if !started {
            guard video else { return }
            w.startWriting()
            w.startSession(atSourceTime: CMSampleBufferGetPresentationTimeStamp(sb))
            started = true
        }
        if video, let vi = vIn, vi.isReadyForMoreMediaData { vi.append(sb) }
        if !video, let ai = aIn, ai.isReadyForMoreMediaData { ai.append(sb) }
    }

    func finish(_ cb: @escaping (Bool) -> Void) {
        lock.lock()
        done = true
        let w = writer
        let ok = started
        lock.unlock()
        guard let w, ok else { return cb(false) }
        vIn?.markAsFinished()
        aIn?.markAsFinished()
        w.finishWriting { cb(w.status == .completed) }
    }
}
