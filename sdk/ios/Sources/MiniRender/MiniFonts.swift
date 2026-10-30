import Foundation

/// 把随 SDK 带的中文字体（思源黑体 CN 常规 / 粗体，Fonts/）交给引擎：建引擎实例之前设 `MINI_MAIN_FONT` / `MINI_BOLD_FONT`。
///
/// 为什么要自带字体：iOS 的苹方在 `FontServices.framework/CorePrivate/PingFangUI.ttc`，是苹果私有的 `hvgl` 轮廓格式
/// （没有 glyf / CFF 表），引擎的光栅化器读不了；系统里也没有别的中文黑体（只有日文的 Hiragino）。
/// 不给的话引擎退回内置的 ArialUnicode：西文像 Arial、汉字发虚、标题没有真粗体只能描粗 —— iOS 上整页难看就是这个原因。
/// 思源黑体与苹方同为现代黑体，字重、字面大小接近，许可是 SIL OFL 1.1（Fonts/LICENSE-SourceHanSans.txt）。
///
/// 已经设过这两个环境变量（宿主自己指定了字体）就不覆盖。
enum MiniFonts {
    private static let once: Void = {
        guard ProcessInfo.processInfo.environment["MINI_MAIN_FONT"] == nil else { return }
        guard let regular = find("SourceHanSansCN-Regular") else {
            NSLog("⚠️ mini-render：没找到随包字体 SourceHanSansCN-Regular.otf，中文会退到内置字体")
            return
        }
        setenv("MINI_MAIN_FONT", regular, 0)
        if let bold = find("SourceHanSansCN-Bold") {
            setenv("MINI_BOLD_FONT", bold, 0)
        }
    }()

    /// 建引擎实例之前调（多次调用只生效一次）
    static func install() {
        _ = once
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
                if let u = sub.url(forResource: name, withExtension: "otf") ?? sub.url(forResource: name, withExtension: "otf", subdirectory: "Fonts") {
                    return u.path
                }
            }
        }
        return nil
    }
}
