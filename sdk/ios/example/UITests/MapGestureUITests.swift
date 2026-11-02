import XCTest
/// 真触摸验收：按在 SDK 自带的地图（`MiniMapView`，同层渲染）上拖动，拖的是**地图**、页面不滚（引擎 `sameLayerGestures`
/// 把这串触摸整串交给宿主，host/engine_native_touch.rs）；按在地图下面拖，滚的是页面。
///
/// 1. 深链打开内置示例的 `pages/map/map` → 截 `mg-0-rest`
/// 2. 在地图上横着拖 → 截 `mg-1-dragged`：地图那一截变了（瓦片、标记跟着走），地图下面 MapContext 那张卡片一个像素都没动（页面没滚；
///    再往下的「属性」卡片会随 regionchange 改中心点文字，不拿它判）
/// 3. 在地图下面的卡片上往上拖 → 截 `mg-2-page`：下面那截变了（页面滚了）
///
///   xcodebuild test -scheme MiniRenderExample -destination 'platform=iOS Simulator,name=iPhone 17 Pro' -only-testing:MiniRenderExampleUITests/MapGestureUITests
final class MapGestureUITests: XCTestCase {
    func testDragOnMapPansMapNotPage() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-autoOpen", "1", "-route", "pages/map/map"]
        app.launch()
        sleep(8)
        let rest = shot(app, "0-rest")
        // 地图在导航栏下面，约占屏高 0.12~0.50（640rpx 高）
        let a = app.coordinate(withNormalizedOffset: CGVector(dx: 0.75, dy: 0.30))
        let b = app.coordinate(withNormalizedOffset: CGVector(dx: 0.25, dy: 0.33))
        a.press(forDuration: 0.1, thenDragTo: b, withVelocity: 300, thenHoldForDuration: 0.3)
        sleep(2)
        let dragged = shot(app, "1-dragged")
        XCTAssertGreaterThan(diff(rest, dragged, 0.16, 0.46), 0.05, "在地图上拖，地图应当跟着走")
        XCTAssertLessThan(diff(rest, dragged, 0.53, 0.72), 0.002, "在地图上拖，页面不该滚（触摸没整串交给地图？）")
        let c = app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.85))
        let d = app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.65))
        c.press(forDuration: 0.05, thenDragTo: d, withVelocity: 400, thenHoldForDuration: 0.3)
        sleep(2)
        let page = shot(app, "2-page")
        XCTAssertGreaterThan(diff(dragged, page, 0.53, 0.72), 0.02, "在地图下面拖，页面应当滚")
    }

    /// 两张截图在 [y0, y1)（屏高比例）这一截里不同的像素占比
    private func diff(_ a: XCUIScreenshot, _ b: XCUIScreenshot, _ y0: Double, _ y1: Double) -> Double {
        let (pa, w, h) = rgba(a), (pb, w2, h2) = rgba(b)
        guard w == w2, h == h2, w > 0 else { return 1 }
        var n = 0, total = 0
        for y in Int(Double(h) * y0)..<Int(Double(h) * y1) {
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
        try? png.write(to: URL(fileURLWithPath: "\(dir)/mg-\(name).png"))
        let att = XCTAttachment(data: png, uniformTypeIdentifier: "public.png")
        att.name = name
        att.lifetime = .keepAlways
        add(att)
        return s
    }
}
