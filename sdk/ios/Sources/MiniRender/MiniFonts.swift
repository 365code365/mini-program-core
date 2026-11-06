import Foundation
import CoreText
/// 让引擎在 iOS 上按微信 iOS（WebKit：西文 SF、汉字苹方）的口径排字：建引擎实例之前设好环境变量。
///
/// - 汉字：苹方在 `FontServices.framework/CorePrivate/PingFangUI.ttc`，是苹果私有的 `hvgl` 轮廓格式，引擎读不了，
///   所以随包带 MiniRender Sans（Fonts/，思源黑体按墨量取字重对到苹方 + Inter，tools/ios-fonts/README.md）。
/// - 西文：系统的 SF（`/System/Library/Fonts/Core/SFUI.ttf`）是普通的 glyf 可变字体，读得到就直接用，
///   字形、字宽与微信一样；读不到时用 MiniRender Sans 里的 Inter。
/// - `MINI_LINE_BOX=ios@<屏幕倍率>`：行盒、基线、汉字字宽、emoji 大小按 WebKit 的口径（引擎 text/ios_box.rs）。
///
/// 宿主自己设过 `MINI_MAIN_FONT` 就整个不覆盖。
enum MiniFonts {
    private static let lock = NSLock()
    private static var done = false
    /// 建引擎实例之前调（多次调用只生效一次）。`dpr` 是引擎的屏幕倍率：行盒要先按 CSS 像素取整再乘它
    static func install(dpr: Float) {
        lock.lock(); defer { lock.unlock() }
        guard !done else { return }
        done = true
        // 彩色 emoji：iOS 的 Apple Color Emoji 不在 macOS 那个路径（在 Fonts/CoreAddition 下，字形是 emjc，
        // 引擎 text/emoji/emjc.rs），按 CoreText 问到的路径告诉引擎。从前找不到，emoji 全画成豆腐块（farm001 商店的「🌱 种子」）
        if ProcessInfo.processInfo.environment["MINI_EMOJI_FONT"] == nil,
           let url = CTFontCopyAttribute(CTFontCreateWithName("AppleColorEmoji" as CFString, 17, nil), kCTFontURLAttribute) as? URL {
            setenv("MINI_EMOJI_FONT", url.path, 0)
        }
        guard ProcessInfo.processInfo.environment["MINI_MAIN_FONT"] == nil else { return }
        guard let regular = find("MiniRenderSans-Regular") else {
            NSLog("⚠️ mini-render：没找到随包字体 MiniRenderSans-Regular.ttf，中文会退到内置字体")
            return
        }
        setenv("MINI_MAIN_FONT", regular, 0)
        if let bold = find("MiniRenderSans-Bold") {
            setenv("MINI_BOLD_FONT", bold, 0)
        }
        // CSS 的 bold 在 SF 上就是 Bold（700）；opsz 17 是 CoreText 给正文字号用的 Text 设计
        let sf = systemFontPath()
        if FileManager.default.isReadableFile(atPath: sf) {
            setenv("MINI_LATIN_FONT", "\(sf)@400@17", 0)
            setenv("MINI_LATIN_BOLD_FONT", "\(sf)@700@17", 0)
        } else {
            NSLog("⚠️ mini-render：读不到 %@，西文改用随包字体里的 Inter", sf)
        }
        setenv("MINI_LINE_BOX", "ios@" + String(format: "%g", Double(dpr)), 0)
    }
    /// 系统字体文件的路径：问 CoreText 要（模拟器上系统根目录不是 `/`，而是运行时的 RuntimeRoot），
    /// 要不到就用真机上的固定位置
    private static func systemFontPath() -> String {
        let font = CTFontCreateUIFontForLanguage(.system, 17, nil)
        if let font, let url = CTFontCopyAttribute(font, kCTFontURLAttribute) as? URL {
            return url.path
        }
        return "/System/Library/Fonts/Core/SFUI.ttf"
    }
    private final class Token {}
    /// SPM 的资源在 Bundle.module；CocoaPods 的在 MiniRenderFonts.bundle（resource_bundles）里，
    /// 那个 bundle 放在框架或主 App 的 bundle 下
    private static func find(_ name: String) -> String? {
        var bundles: [Bundle] = []
        #if SWIFT_PACKAGE
        bundles.append(Bundle.module)
        #endif
        bundles.append(Bundle(for: Token.self))
        bundles.append(Bundle.main)
        for b in bundles {
            for sub in [b, b.url(forResource: "MiniRenderFonts", withExtension: "bundle").flatMap(Bundle.init(url:))].compactMap({ $0 }) {
                if let u = sub.url(forResource: name, withExtension: "ttf") ?? sub.url(forResource: name, withExtension: "ttf", subdirectory: "Fonts") {
                    return u.path
                }
            }
        }
        return nil
    }
}
