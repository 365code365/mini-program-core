import XCTest
/// 真触摸验收：小程序二级页从屏幕左缘侧滑返回（引擎的 host/edge_back.rs）。
///
/// 从前示例的 MiniViewController 在每一页都挂一个 UIScreenEdgePanGestureRecognizer：它一认出手势，UIKit 就把交给
/// 引擎视图的触摸 cancel 掉，引擎的侧滑推出几个像素就收回去 —— 页面不跟手、半路松手不回弹，划过 60pt 直接跳回上一页。
/// 现在宿主的识别器只在栈底那一页接（收起小程序），二级页交给引擎。
///
/// 步骤（截图到 `$KIRO_UI_SHOTS`，默认 /tmp，文件名 eb-*.png，脚本拿 pixdiff 比）：
/// 1. 打开内置示例首页 → 点 `TAP_X/TAP_Y`（默认 iPhone 17 Pro 上「WXML 语法」那一项）进二级页 → 截 `eb-1-page2`
/// 2. 从左缘慢慢拖到屏宽 20% 停住再松手（不到 35% 的阈值）→ 等回弹动画 → 截 `eb-2-after-short`：应当仍是二级页、位置复原
/// 3. 从左缘拖到屏宽 70% 松手 → 截 `eb-3-after-long`：回到首页
///
///   xcodebuild test -scheme MiniRenderExample -destination 'platform=iOS Simulator,name=iPhone 17 Pro' -only-testing:MiniRenderExampleUITests/EdgeBackUITests
final class EdgeBackUITests: XCTestCase {
    func testPartialSwipeSpringsBackAndFullSwipeGoesBack() throws {
        let env = ProcessInfo.processInfo.environment
        let app = XCUIApplication()
        app.launchArguments = ["-autoOpen", "1"]
        app.launch()
        sleep(8)
        let home = shot(app, "0-home")
        let x = Double(env["TAP_X"] ?? "0.5") ?? 0.5
        let y = Double(env["TAP_Y"] ?? "0.727") ?? 0.727
        app.coordinate(withNormalizedOffset: CGVector(dx: x, dy: y)).tap()
        sleep(3)
        let page2 = shot(app, "1-page2")
        XCTAssertGreaterThan(diff(home, page2), 0.2, "点了列表项没进二级页（TAP_Y 不对？）")
        // 短划：从左缘 2pt 处按下，慢慢拖到 20% 停 0.3 秒再松手（速度低于「甩」的阈值）
        let start = app.coordinate(withNormalizedOffset: CGVector(dx: 0.005, dy: 0.5))
        let short = app.coordinate(withNormalizedOffset: CGVector(dx: 0.20, dy: 0.5))
        start.press(forDuration: 0.1, thenDragTo: short, withVelocity: 200, thenHoldForDuration: 0.3)
        sleep(1)
        let afterShort = shot(app, "2-after-short")
        XCTAssertLessThan(diff(page2, afterShort), 0.005, "不到阈值松手应当回弹、留在二级页（从前宿主的识别器抢走手势，直接跳回了首页）")
        // 长划：拖过 70% 松手 → 返回
        let long = app.coordinate(withNormalizedOffset: CGVector(dx: 0.70, dy: 0.5))
        start.press(forDuration: 0.1, thenDragTo: long, withVelocity: 600, thenHoldForDuration: 0.1)
        sleep(1)
        let afterLong = shot(app, "3-after-long")
        XCTAssertLessThan(diff(home, afterLong), 0.01, "过了阈值松手应当回到首页（不是关掉小程序）")
    }

    /// 两张截图不同的像素占比（跳过顶上 12%：状态栏的时钟会走）
    private func diff(_ a: XCUIScreenshot, _ b: XCUIScreenshot) -> Double {
        let (pa, w, h) = rgba(a), (pb, w2, h2) = rgba(b)
        guard w == w2, h == h2 else { return 1 }
        var n = 0, total = 0
        for y in (h * 12 / 100)..<h {
            for x in 0..<w {
                let i = (y * w + x) * 4
                total += 1
                if abs(Int(pa[i]) - Int(pb[i])) > 8 || abs(Int(pa[i + 1]) - Int(pb[i + 1])) > 8 || abs(Int(pa[i + 2]) - Int(pb[i + 2])) > 8 { n += 1 }
            }
        }
        return Double(n) / Double(max(total, 1))
    }

    private func rgba(_ s: XCUIScreenshot) -> ([UInt8], Int, Int) {
        guard let cg = s.image.cgImage else { return ([], 0, 0) }
        let (w, h) = (cg.width, cg.height)
        var buf = [UInt8](repeating: 0, count: w * h * 4)
        buf.withUnsafeMutableBytes { p in
            let ctx = CGContext(data: p.baseAddress, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
            ctx?.draw(cg, in: CGRect(x: 0, y: 0, width: w, height: h))
        }
        return (buf, w, h)
    }

    @discardableResult
    private func shot(_ app: XCUIApplication, _ name: String) -> XCUIScreenshot {
        let dir = ProcessInfo.processInfo.environment["KIRO_UI_SHOTS"] ?? "/tmp"
        let s = app.screenshot()
        let png = s.pngRepresentation
        try? png.write(to: URL(fileURLWithPath: "\(dir)/eb-\(name).png"))
        let a = XCTAttachment(data: png, uniformTypeIdentifier: "public.png")
        a.name = name
        a.lifetime = .keepAlways
        add(a)
        return s
    }
}
