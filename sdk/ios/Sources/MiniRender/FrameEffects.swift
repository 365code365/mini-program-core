import CoreImage
import CoreMedia
import CoreVideo

/// `<live-pusher>` 的美颜（`beauty` 0~9、`beauty-style`）、美白（`whiteness` 0~9）、色彩滤镜（`filter`）：
/// 与 Android `FrameEffects.kt` 同一套算法与参数，这边用 Core Image（GPU）：
/// - 磨皮：原图与高斯模糊图逐像素按颜色差加权（差得远的是边缘，留原图）、按 YCbCr 肤色软遮罩混合；
/// - 美白：对数曲线提亮；滤镜：每个名字一个 3×3 颜色矩阵 + 偏移（**按名字调的近似**，微信用腾讯云 SDK 的查找表）。
///
/// 相机每来一帧（`mini-camera` 队列上）处理一次，预览、拍照、推流都用处理过的那一帧（[CameraCapture]）。全关时不进来。
final class FrameEffects {
    struct Params: Equatable {
        var beauty: Float
        var white: Float
        var nature: Bool
        /// 三行，每行 (r, g, b, 偏移)；nil = standard（不调色）
        var filter: [Float]?
    }

    /// 属性 → 参数；全关返回 nil
    static func parse(_ attrs: [String: Any]) -> Params? {
        func level(_ k: String) -> Float {
            let v = Float("\(attrs[k] ?? "")".trimmingCharacters(in: .whitespaces)) ?? 0
            return min(max(v, 0), 9) / 9
        }
        let style = "\(attrs["beauty-style"] ?? "")".trimmingCharacters(in: .whitespaces)
        let name = "\(attrs["filter"] ?? "")".trimmingCharacters(in: .whitespaces)
        let p = Params(beauty: level("beauty"), white: level("whiteness"), nature: style == "nature", filter: filters[name])
        return p.beauty > 0 || p.white > 0 || p.filter != nil ? p : nil
    }

    private static func diag(_ r: Float, _ g: Float, _ b: Float, _ o: [Float]) -> [Float] {
        [r, 0, 0, o[0], 0, g, 0, o[1], 0, 0, b, o[2]]
    }

    private static func rows(_ m: [Float], _ o: [Float]) -> [Float] {
        [m[0], m[1], m[2], o[0], m[3], m[4], m[5], o[1], m[6], m[7], m[8], o[2]]
    }

    private static func saturation(_ s: Float) -> [Float] {
        let l: [Float] = [0.299, 0.587, 0.114]
        return (0..<9).map { i in (1 - s) * l[i % 3] + (i / 3 == i % 3 ? s : 0) }
    }

    /// 与 Android 同一张表（数值逐个对照 `FrameEffects.FILTERS`）
    static let filters: [String: [Float]] = {
        let sepia: [Float] = [0.393, 0.769, 0.189, 0.349, 0.686, 0.168, 0.272, 0.534, 0.131]
        let nostalgia = (0..<9).map { i in 0.7 * sepia[i] + (i / 3 == i % 3 ? 0.3 : 0) }
        return [
            "pink": diag(1.05, 0.97, 1.02, [0.03, 0, 0.04]),
            "nostalgia": rows(nostalgia, [0, 0, 0]),
            "blues": rows([0.9, 0, 0, 0, 0.98, 0.02, 0, 0.05, 1.1], [0, 0.01, 0.04]),
            "romantic": rows([1.08, 0.02, 0, 0, 0.98, 0, 0.02, 0, 1.04], [0.02, 0, 0.03]),
            "cool": rows([0.92, 0, 0, 0, 1, 0.02, 0, 0.02, 1.08], [0, 0.02, 0.05]),
            "fresher": rows(saturation(1.15), [0, 0.02, 0.02]),
            "solor": diag(0.87, 0.9, 0.92, [0.05, 0.06, 0.07]),
            "aestheticism": rows(saturation(0.85), [0.04, 0.04, 0.04]),
            "whitening": diag(1.05, 1.05, 1.05, [0.05, 0.05, 0.05]),
            "cerisered": diag(1.12, 0.96, 0.98, [0.02, 0, 0.01]),
        ]
    }()

    /// 一个颜色核把磨皮的混合、美白、滤镜一次做完（输入：原图、模糊图）
    private static let kernel: CIColorKernel? = CIColorKernel(source: """
        kernel vec4 fx(__sample o, __sample b, float beauty, float nature, float white, vec4 r0, vec4 r1, vec4 r2, float useFilter) {
            vec3 c = o.rgb;
            vec3 e = b.rgb - c;
            float w = exp(-dot(e, e) * 60.0);
            float cb = -0.1687 * c.r - 0.3313 * c.g + 0.5 * c.b;
            float cr = 0.5 * c.r - 0.4187 * c.g - 0.0813 * c.b;
            float skin = smoothstep(0.0, 0.03, cr) * (1.0 - smoothstep(0.17, 0.22, cr))
                * smoothstep(-0.24, -0.19, cb) * (1.0 - smoothstep(0.0, 0.04, cb));
            float mask = mix(max(skin, 0.5), skin * 0.7, nature);
            c = mix(c, b.rgb, clamp(beauty * 1.2, 0.0, 1.0) * mask * w);
            float beta = 1.0 + white * 4.0;
            c = white > 0.0 ? log(c * (beta - 1.0) + 1.0) / log(beta) : c;
            vec3 f = clamp(vec3(dot(r0.xyz, c) + r0.w, dot(r1.xyz, c) + r1.w, dot(r2.xyz, c) + r2.w), 0.0, 1.0);
            c = useFilter > 0.0 ? f : c;
            return vec4(c, o.a);
        }
        """)

    /// sRGB 作工作色彩空间：与 Android 着色器一样直接在编码值上算（Core Image 默认是线性空间，算出来颜色不一样）
    private let ctx = CIContext(options: [.workingColorSpace: CGColorSpace(name: CGColorSpace.sRGB) as Any, .cacheIntermediates: false])
    private var pool: CVPixelBufferPool?
    private var poolSize = (0, 0)
    private var warned = false

    /// 处理一帧（BGRA）。核编不过 / 出不了缓冲时返回 nil（调用方用原帧）
    func apply(_ src: CVPixelBuffer, _ p: Params) -> CVPixelBuffer? {
        guard let k = Self.kernel else {
            if !warned { warned = true; NSLog("mini-render 📡 美颜的颜色核编不过：美颜 / 美白 / 滤镜不生效") }
            return nil
        }
        let w = CVPixelBufferGetWidth(src), h = CVPixelBufferGetHeight(src)
        if pool == nil || poolSize != (w, h) {
            let attrs: [String: Any] = [kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
                                        kCVPixelBufferWidthKey as String: w, kCVPixelBufferHeightKey as String: h,
                                        kCVPixelBufferIOSurfacePropertiesKey as String: [:]]
            pool = nil
            CVPixelBufferPoolCreate(nil, nil, attrs as CFDictionary, &pool)
            poolSize = (w, h)
        }
        var dst: CVPixelBuffer?
        guard let pool, CVPixelBufferPoolCreatePixelBuffer(nil, pool, &dst) == kCVReturnSuccess, let out = dst else { return nil }
        let img = CIImage(cvPixelBuffer: src)
        let sigma = Double(1 + p.beauty * 3) * min(max(Double(min(w, h)) / 720, 0.5), 2)
        let blur = p.beauty > 0 ? img.clampedToExtent().applyingGaussianBlur(sigma: sigma).cropped(to: img.extent) : img
        let f = p.filter ?? [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0]
        let v = { (i: Int) in CIVector(x: CGFloat(f[i]), y: CGFloat(f[i + 1]), z: CGFloat(f[i + 2]), w: CGFloat(f[i + 3])) }
        let args: [Any] = [img, blur, p.beauty, p.nature ? 1 : 0, p.white, v(0), v(4), v(8), p.filter != nil ? 1 : 0]
        guard let res = k.apply(extent: img.extent, arguments: args) else { return nil }
        ctx.render(res, to: out)
        return out
    }

    /// 处理过的画面包成一个与原样本同时间戳的样本（录像 / 推流照收 CMSampleBuffer）
    static func wrap(_ pb: CVPixelBuffer, like sb: CMSampleBuffer) -> CMSampleBuffer? {
        var fmt: CMVideoFormatDescription?
        guard CMVideoFormatDescriptionCreateForImageBuffer(allocator: nil, imageBuffer: pb, formatDescriptionOut: &fmt) == noErr, let fmt else { return nil }
        var timing = CMSampleTimingInfo()
        guard CMSampleBufferGetSampleTimingInfo(sb, at: 0, timingInfoOut: &timing) == noErr else { return nil }
        var out: CMSampleBuffer?
        guard CMSampleBufferCreateReadyWithImageBuffer(allocator: nil, imageBuffer: pb, formatDescription: fmt, sampleTiming: &timing, sampleBufferOut: &out) == noErr else { return nil }
        return out
    }
}
