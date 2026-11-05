import AVFoundation
import CoreImage
import VideoToolbox

/// `<live-pusher>` 的编码（与 Android `AvEncoder.kt` 对照）：画面 VTCompressionSession 硬编 H.264（按推流尺寸 cover 裁缩），
/// 声音 AVAudioConverter 编 AAC-LC（单声道）。编出来的直接交给 [RtmpPublisher]（AVCC 原样、ASC 自己拼）。
final class LiveEncoder {
    let width: Int, height: Int, fps: Int
    private var vt: VTCompressionSession?
    private let pub: RtmpPublisher
    private var base: CMTime?
    private var headerSent = false
    private var lastPts = CMTime.invalid
    private var pool: CVPixelBufferPool?
    private var audioConv: AVAudioConverter?
    private var audioOutFmt: AVAudioFormat?
    private let sampleRate: Double
    var muted = false
    var volume: Float = 1
    /// 背景音（LivePusherContext.playBGM）：每段麦克风 PCM 混上同样多的背景音（静音的是麦克风，背景音照推）
    var bgm: BgmMixer?

    init?(width: Int, height: Int, fps: Int, bitrate: Int, sampleRate: Double, publisher: RtmpPublisher) {
        self.width = width
        self.height = height
        self.fps = fps
        self.sampleRate = sampleRate
        pub = publisher
        var s: VTCompressionSession?
        let st = VTCompressionSessionCreate(allocator: nil, width: Int32(width), height: Int32(height), codecType: kCMVideoCodecType_H264,
                                            encoderSpecification: nil, imageBufferAttributes: nil, compressedDataAllocator: nil,
                                            outputCallback: nil, refcon: nil, compressionSessionOut: &s)
        guard st == noErr, let session = s else { return nil }
        VTSessionSetProperty(session, key: kVTCompressionPropertyKey_RealTime, value: kCFBooleanTrue)
        VTSessionSetProperty(session, key: kVTCompressionPropertyKey_ProfileLevel, value: kVTProfileLevel_H264_Main_AutoLevel)
        VTSessionSetProperty(session, key: kVTCompressionPropertyKey_AverageBitRate, value: bitrate as CFNumber)
        VTSessionSetProperty(session, key: kVTCompressionPropertyKey_ExpectedFrameRate, value: fps as CFNumber)
        VTSessionSetProperty(session, key: kVTCompressionPropertyKey_MaxKeyFrameIntervalDuration, value: 2 as CFNumber)
        VTSessionSetProperty(session, key: kVTCompressionPropertyKey_AllowFrameReordering, value: kCFBooleanFalse)
        VTCompressionSessionPrepareToEncodeFrames(session)
        vt = session
        let attrs: [String: Any] = [kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
                                    kCVPixelBufferWidthKey as String: width, kCVPixelBufferHeightKey as String: height,
                                    kCVPixelBufferIOSurfacePropertiesKey as String: [:]]
        CVPixelBufferPoolCreate(nil, nil, attrs as CFDictionary, &pool)
    }

    func requestKeyFrame() { forceKey = true }
    private var forceKey = true

    /// 一帧画面（采集来的整帧）：按帧率取、cover 裁缩到推流尺寸再编
    func encode(video sb: CMSampleBuffer) {
        guard let vt, let src = CMSampleBufferGetImageBuffer(sb) else { return }
        let pts = CMSampleBufferGetPresentationTimeStamp(sb)
        if lastPts.isValid, CMTimeGetSeconds(pts - lastPts) < 0.9 / Double(fps) { return }
        lastPts = pts
        if base == nil { base = pts }
        guard let pool, let dst = scaled(src, pool) else { return }
        let props: CFDictionary? = forceKey ? [kVTEncodeFrameOptionKey_ForceKeyFrame: true] as CFDictionary : nil
        forceKey = false
        VTCompressionSessionEncodeFrame(vt, imageBuffer: dst, presentationTimeStamp: pts, duration: .invalid, frameProperties: props, infoFlagsOut: nil) { [weak self] st, _, out in
            guard st == noErr, let out, let self else { return }
            self.deliver(out)
        }
    }

    private func scaled(_ src: CVPixelBuffer, _ pool: CVPixelBufferPool) -> CVPixelBuffer? {
        var dst: CVPixelBuffer?
        CVPixelBufferPoolCreatePixelBuffer(nil, pool, &dst)
        guard let d = dst else { return nil }
        let ci = CIImage(cvPixelBuffer: src)
        let (x, y, w, h) = CameraCapture.coverRect(CVPixelBufferGetWidth(src), CVPixelBufferGetHeight(src), width, height)
        let crop = ci.cropped(to: CGRect(x: x, y: CVPixelBufferGetHeight(src) - y - h, width: w, height: h))
            .transformed(by: CGAffineTransform(translationX: -CGFloat(x), y: -CGFloat(CVPixelBufferGetHeight(src) - y - h)))
            .transformed(by: CGAffineTransform(scaleX: CGFloat(width) / CGFloat(w), y: CGFloat(height) / CGFloat(h)))
        Self.ci.render(crop, to: d)
        return d
    }

    private static let ci = CIContext()

    private func deliver(_ sb: CMSampleBuffer) {
        guard let fmt = CMSampleBufferGetFormatDescription(sb), let data = CMSampleBufferGetDataBuffer(sb) else { return }
        let attach = CMSampleBufferGetSampleAttachmentsArray(sb, createIfNecessary: false) as? [[CFString: Any]]
        let key = !(attach?.first?[kCMSampleAttachmentKey_NotSync] as? Bool ?? false)
        if key {
            var sps: UnsafePointer<UInt8>?, spsLen = 0, pps: UnsafePointer<UInt8>?, ppsLen = 0, count = 0
            CMVideoFormatDescriptionGetH264ParameterSetAtIndex(fmt, parameterSetIndex: 0, parameterSetPointerOut: &sps, parameterSetSizeOut: &spsLen, parameterSetCountOut: &count, nalUnitHeaderLengthOut: nil)
            CMVideoFormatDescriptionGetH264ParameterSetAtIndex(fmt, parameterSetIndex: 1, parameterSetPointerOut: &pps, parameterSetSizeOut: &ppsLen, parameterSetCountOut: nil, nalUnitHeaderLengthOut: nil)
            if !headerSent, let sps, let pps {
                pub.setVideoHeader(sps: Array(UnsafeBufferPointer(start: sps, count: spsLen)), pps: Array(UnsafeBufferPointer(start: pps, count: ppsLen)))
                headerSent = true
            }
        }
        var len = 0
        var ptr: UnsafeMutablePointer<Int8>?
        guard CMBlockBufferGetDataPointer(data, atOffset: 0, lengthAtOffsetOut: nil, totalLengthOut: &len, dataPointerOut: &ptr) == noErr, let ptr else { return }
        let avcc = Array(UnsafeBufferPointer(start: UnsafeRawPointer(ptr).assumingMemoryBound(to: UInt8.self), count: len))
        pub.video(avcc: avcc, key: key, ts: ms(CMSampleBufferGetPresentationTimeStamp(sb)))
    }

    private func ms(_ t: CMTime) -> UInt32 {
        guard let b = base else { return 0 }
        return UInt32(max(0, CMTimeGetSeconds(t - b) * 1000))
    }

    /// 一段麦克风 PCM（采集来的 CMSampleBuffer）→ AAC
    func encode(audio sb: CMSampleBuffer) {
        guard let fd = CMSampleBufferGetFormatDescription(sb), let asbd = CMAudioFormatDescriptionGetStreamBasicDescription(fd) else { return }
        var inDesc = asbd.pointee
        guard let inFmt = AVAudioFormat(streamDescription: &inDesc) else { return }
        if audioConv == nil {
            var out = AudioStreamBasicDescription(mSampleRate: sampleRate, mFormatID: kAudioFormatMPEG4AAC, mFormatFlags: 2, mBytesPerPacket: 0,
                                                  mFramesPerPacket: 1024, mBytesPerFrame: 0, mChannelsPerFrame: 1, mBitsPerChannel: 0, mReserved: 0)
            guard let of = AVAudioFormat(streamDescription: &out), let c = AVAudioConverter(from: inFmt, to: of) else { return }
            c.bitRate = sampleRate > 16000 ? 96_000 : 32_000
            audioConv = c
            audioOutFmt = of
            // AudioSpecificConfig：AAC LC(2) · 采样率序号 · 单声道
            let rates: [Double] = [96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000]
            let idx = UInt16(rates.firstIndex(of: sampleRate) ?? 4)
            let v: UInt16 = (2 << 11) | (idx << 7) | (1 << 3)
            pub.setAudioHeader(asc: [UInt8(v >> 8), UInt8(v & 0xFF)])
        }
        guard let conv = audioConv, let of = audioOutFmt else { return }
        let frames = AVAudioFrameCount(CMSampleBufferGetNumSamples(sb))
        guard let pcm = AVAudioPCMBuffer(pcmFormat: inFmt, frameCapacity: frames) else { return }
        pcm.frameLength = frames
        CMSampleBufferCopyPCMDataIntoAudioBufferList(sb, at: 0, frameCount: Int32(frames), into: pcm.mutableAudioBufferList)
        if muted || volume != 1, let ch = pcm.int16ChannelData {
            for k in 0..<Int(frames) { ch[0][k] = muted ? 0 : Int16(max(-32768, min(32767, Float(ch[0][k]) * volume))) }
        }
        bgm?.mix(into: pcm)
        let out = AVAudioCompressedBuffer(format: of, packetCapacity: 8, maximumPacketSize: conv.maximumOutputPacketSize)
        var fed = false
        var err: NSError?
        conv.convert(to: out, error: &err) { _, status in
            if fed { status.pointee = .noDataNow; return nil }
            fed = true
            status.pointee = .haveData
            return pcm
        }
        let pts = ms(CMSampleBufferGetPresentationTimeStamp(sb))
        for i in 0..<Int(out.packetCount) {
            guard let d = out.packetDescriptions?[i] else { continue }
            let p = out.data.advanced(by: Int(d.mStartOffset)).assumingMemoryBound(to: UInt8.self)
            pub.audio(aac: Array(UnsafeBufferPointer(start: p, count: Int(d.mDataByteSize))), ts: pts)
        }
    }

    func stop() {
        if let vt {
            VTCompressionSessionCompleteFrames(vt, untilPresentationTimeStamp: .invalid)
            VTCompressionSessionInvalidate(vt)
        }
        vt = nil
    }
}
