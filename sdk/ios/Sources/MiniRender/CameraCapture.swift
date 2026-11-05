import AVFoundation
import Accelerate
import CoreImage
import UIKit

/// 相机的一路画面（`<camera>` / `<live-pusher>` / `<voip-room mode="camera">` 共用，与 Android `CameraPipe.kt` 对照）：
/// AVCaptureSession → `AVCaptureVideoDataOutput`（BGRA，连接上设成竖屏、前置镜像）→ 每帧在 `mini-camera` 队列上：
/// 1. 按引擎里那个框的设备像素 cover 裁好、缩好、换成 RGBA（vImage）交给 [onPreview]（调用方交给引擎 `mr_native_view_frame_rgba`）；
/// 2. 录像时整帧交给 [onSample]（AVAssetWriter / 推流编码器）；
/// 3. 拍照时取这一帧存图（官方：iOS 是 png）。
///
/// 镜像：输出连接的 `isVideoMirrored` 按 [mirrored] 设（相机组件前置镜像、推流按 local-mirror），预览、拍照、录像都是这一份。
final class CameraCapture: NSObject, AVCaptureVideoDataOutputSampleBufferDelegate, AVCaptureAudioDataOutputSampleBufferDelegate {
    let queue = DispatchQueue(label: "mini-camera")
    private let session = AVCaptureSession()
    private let videoOut = AVCaptureVideoDataOutput()
    private let audioOut = AVCaptureAudioDataOutput()
    private var device: AVCaptureDevice?
    private var running = false
    private(set) var front = false
    /// 会话里带了麦克风输入（录像 / 推流要声音）
    private(set) var hasAudio = false

    /// 引擎框的设备像素（任何线程写，队列上读）
    var boxW = 0
    var boxH = 0
    var previewEnabled = true
    /// 摆正后的画面尺寸
    private(set) var frameW = 0
    private(set) var frameH = 0

    var onOpened: ((Double) -> Void)?
    var onError: ((String, Bool) -> Void)?
    var onFirstFrame: (() -> Void)?
    var onPreview: ((UnsafePointer<UInt8>, Int, Int, Int) -> Void)?
    /// 录像 / 推流：原样的视频（`true`）/ 音频（`false`）样本
    var onSample: ((CMSampleBuffer, Bool) -> Void)?
    private var firstFrame = true
    private var rgba: UnsafeMutablePointer<UInt8>?
    private var rgbaSize = 0
    private var photo: ((CVPixelBuffer) -> Void)?
    private var torch = false
    private var flash = "auto"
    /// live-pusher 的美颜 / 美白 / 滤镜（[FrameEffects]）：预览、拍照、推流都用处理过的那一帧。只在队列上读写（[setEffects]）
    private var effects: FrameEffects.Params?
    private lazy var fx = FrameEffects()

    func setEffects(_ p: FrameEffects.Params?) { queue.async { [self] in effects = p } }

    /// 开相机：[front] 前后置，[preset] 分辨率（low / medium / high），[mirrored] 前置是否镜像，[audio] 要不要麦克风
    func open(front: Bool, preset: AVCaptureSession.Preset, mirrored: Bool, audio: Bool) {
        queue.async { [self] in
            stopNow()
            self.front = front
            session.beginConfiguration()
            session.inputs.forEach { session.removeInput($0) }
            session.outputs.forEach { session.removeOutput($0) }
            if session.canSetSessionPreset(preset) { session.sessionPreset = preset }
            guard let d = AVCaptureDevice.default(.builtInWideAngleCamera, for: .video, position: front ? .front : .back),
                  let input = try? AVCaptureDeviceInput(device: d), session.canAddInput(input)
            else {
                session.commitConfiguration()
                onError?("没有可用的摄像头", false)
                return
            }
            session.addInput(input)
            device = d
            videoOut.videoSettings = [kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA]
            videoOut.alwaysDiscardsLateVideoFrames = true
            videoOut.setSampleBufferDelegate(self, queue: queue)
            if session.canAddOutput(videoOut) { session.addOutput(videoOut) }
            if let c = videoOut.connection(with: .video) {
                if c.isVideoOrientationSupported { c.videoOrientation = .portrait }
                if c.isVideoMirroringSupported {
                    c.automaticallyAdjustsVideoMirroring = false
                    c.isVideoMirrored = mirrored
                }
            }
            hasAudio = false
            if audio, let mic = AVCaptureDevice.default(for: .audio), let ai = try? AVCaptureDeviceInput(device: mic), session.canAddInput(ai) {
                session.addInput(ai)
                hasAudio = true
                audioOut.setSampleBufferDelegate(self, queue: queue)
                if session.canAddOutput(audioOut) { session.addOutput(audioOut) }
            }
            session.commitConfiguration()
            firstFrame = true
            session.startRunning()
            running = session.isRunning
            if !running { onError?("打开摄像头失败", false); return }
            applyTorch()
            onOpened?(Double(min(d.maxAvailableVideoZoomFactor, 10)))
        }
    }

    func close() { queue.async { [self] in stopNow() } }

    /// 第一次录像 / 推流拿到麦克风权限之后：带上声音重开会话
    func reopenWithAudio(front: Bool, preset: AVCaptureSession.Preset, mirrored: Bool) {
        open(front: front, preset: preset, mirrored: mirrored, audio: true)
    }

    private func stopNow() {
        if session.isRunning { session.stopRunning() }
        running = false
    }

    var isOpen: Bool { running }

    func setZoom(_ z: Double, done: @escaping (Double) -> Void) {
        queue.async { [self] in
            guard let d = device, (try? d.lockForConfiguration()) != nil else { return done(1) }
            let v = CGFloat(z).clamped(1, min(d.maxAvailableVideoZoomFactor, 10))
            d.videoZoomFactor = v
            d.unlockForConfiguration()
            done(Double(v))
        }
    }

    var maxZoom: Double { Double(min(device?.maxAvailableVideoZoomFactor ?? 1, 10)) }

    func setFlash(_ mode: String) { queue.async { [self] in flash = mode; applyTorch() } }
    func setTorch(_ on: Bool) { queue.async { [self] in flash = on ? "torch" : "off"; applyTorch() } }

    private func applyTorch(_ forced: Bool? = nil) {
        guard let d = device, d.hasTorch, (try? d.lockForConfiguration()) != nil else { return }
        let on = forced ?? (flash == "torch")
        d.torchMode = on ? .on : .off
        d.unlockForConfiguration()
    }

    /// 拍一张：下一帧存 PNG（[side] 长边上限）。flash="on" 时先开手电 450ms 再取
    func takePhoto(side: Int, file: URL, done: @escaping (URL?, String?) -> Void) {
        queue.async { [self] in
            guard running else { return done(nil, "相机还没开好") }
            let shoot = { [self] in
                photo = { buf in
                    let ok = Self.writePNG(buf, side: side, to: file)
                    done(ok ? file : nil, ok ? nil : "存图失败")
                }
            }
            if flash == "on", device?.hasTorch == true, !front {
                applyTorch(true)
                queue.asyncAfter(deadline: .now() + 0.45) { [self] in
                    shoot()
                    queue.asyncAfter(deadline: .now() + 0.15) { [self] in applyTorch() }
                }
            } else {
                shoot()
            }
        }
    }

    static func writePNG(_ buf: CVPixelBuffer, side: Int, to url: URL) -> Bool {
        let ci = CIImage(cvPixelBuffer: buf)
        let w = CGFloat(CVPixelBufferGetWidth(buf))
        let h = CGFloat(CVPixelBufferGetHeight(buf))
        let s = min(1, CGFloat(side) / max(w, h))
        let img = s < 1 ? ci.transformed(by: CGAffineTransform(scaleX: s, y: s)) : ci
        let ctx = CIContext()
        guard let cg = ctx.createCGImage(img, from: img.extent), let data = UIImage(cgImage: cg).pngData() else { return false }
        return (try? data.write(to: url)) != nil
    }

    // MARK: 帧

    func captureOutput(_ output: AVCaptureOutput, didOutput sb: CMSampleBuffer, from connection: AVCaptureConnection) {
        if output === audioOut {
            onSample?(sb, false)
            return
        }
        guard var pb = CMSampleBufferGetImageBuffer(sb) else { return }
        var sb = sb
        if let p = effects, let out = fx.apply(pb, p), let wrapped = FrameEffects.wrap(out, like: sb) {
            pb = out
            sb = wrapped
        }
        frameW = CVPixelBufferGetWidth(pb)
        frameH = CVPixelBufferGetHeight(pb)
        if firstFrame {
            firstFrame = false
            onFirstFrame?()
        }
        onSample?(sb, true)
        if let p = photo {
            photo = nil
            p(pb)
        }
        if previewEnabled && boxW > 0 && boxH > 0 { preview(pb, boxW, boxH) }
    }

    /// cover 裁 + 缩到 `w×h` + BGRA → RGBA
    private func preview(_ pb: CVPixelBuffer, _ w: Int, _ h: Int) {
        CVPixelBufferLockBaseAddress(pb, .readOnly)
        defer { CVPixelBufferUnlockBaseAddress(pb, .readOnly) }
        guard let base = CVPixelBufferGetBaseAddress(pb) else { return }
        let sw = CVPixelBufferGetWidth(pb), sh = CVPixelBufferGetHeight(pb), row = CVPixelBufferGetBytesPerRow(pb)
        let (cx, cy, cw, ch) = Self.coverRect(sw, sh, w, h)
        var src = vImage_Buffer(data: base.advanced(by: cy * row + cx * 4), height: vImagePixelCount(ch), width: vImagePixelCount(cw), rowBytes: row)
        let need = w * h * 4
        if rgbaSize != need {
            rgba?.deallocate()
            rgba = UnsafeMutablePointer<UInt8>.allocate(capacity: need)
            rgbaSize = need
        }
        guard let out = rgba else { return }
        var dst = vImage_Buffer(data: out, height: vImagePixelCount(h), width: vImagePixelCount(w), rowBytes: w * 4)
        guard vImageScale_ARGB8888(&src, &dst, nil, vImage_Flags(kvImageNoFlags)) == kvImageNoError else { return }
        let map: [UInt8] = [2, 1, 0, 3]
        vImagePermuteChannels_ARGB8888(&dst, &dst, map, vImage_Flags(kvImageNoFlags))
        onPreview?(UnsafePointer(out), w, h, w * 4)
    }

    /// 源 `sw×sh` 铺满 `w×h`（cover）要取的那一块：(x, y, 宽, 高)
    static func coverRect(_ sw: Int, _ sh: Int, _ w: Int, _ h: Int) -> (Int, Int, Int, Int) {
        let src = Double(sw) / Double(sh), dst = Double(w) / Double(h)
        if src > dst {
            let cw = Int(Double(sh) * dst)
            return ((sw - cw) / 2, 0, cw, sh)
        }
        let ch = Int(Double(sw) / dst)
        return (0, (sh - ch) / 2, sw, ch)
    }

    deinit { rgba?.deallocate() }
}

private extension CGFloat {
    func clamped(_ lo: CGFloat, _ hi: CGFloat) -> CGFloat { Swift.min(Swift.max(self, lo), hi) }
}
