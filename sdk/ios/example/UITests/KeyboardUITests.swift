import XCTest

/// 真触摸验收：小程序里点输入框要弹系统键盘、敲的字要进输入框（MiniProgramView 的 MiniKeyboard）。
///
/// 从前 iOS 原生视图没有接键盘：生活缴费「水费缴费」点「其他金额」，输入框有光标、键盘不出来。
/// 这里按真机路径走一遍：商店打开 life-pay 的缴费页 → 点「其他金额」（页面 `focus` 自动聚焦）→ 等键盘 →
/// 敲 `12.5` → 键盘收起前后各截一张图到 `$KIRO_UI_SHOTS`（默认 /tmp），人可以对照看输入框里的字。
///
///   xcodebuild test -scheme MiniRenderExample -destination 'platform=iOS Simulator,name=iPhone 17 Pro' \
///     TEST_RUNNER_TAP_X=0.2 TEST_RUNNER_TAP_Y=0.69
/// 点击位置按屏幕比例给（`TEST_RUNNER_` 前缀的构建设置会成为测试进程的环境变量），默认是 iPhone 17 Pro 上「其他金额」的位置。
final class KeyboardUITests: XCTestCase {
    func testTapInputShowsKeyboardAndTypes() throws {
        let env = ProcessInfo.processInfo.environment
        let app = XCUIApplication()
        app.launchArguments = ["-autoOpen", "0", "-open", "life-pay", "-route", "pages/pay/pay"]
        app.launch()
        // 商店：检查更新 → 下载 → 解包 → 第一帧
        sleep(12)
        shot(app, "1-page")
        let x = Double(env["TAP_X"] ?? "0.2") ?? 0.2
        let y = Double(env["TAP_Y"] ?? "0.69") ?? 0.69
        app.coordinate(withNormalizedOffset: CGVector(dx: x, dy: y)).tap()
        sleep(1)
        shot(app, "1b-custom")
        // 再点一下出现的输入框（「其他金额」下面那一行）
        let iy = Double(env["INPUT_Y"] ?? "0.755") ?? 0.755
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: iy)).tap()
        let kb = app.keyboards.firstMatch
        XCTAssertTrue(kb.waitForExistence(timeout: 5), "点了输入框，系统键盘没弹出来")
        sleep(1)
        shot(app, "1c-keyboard")
        // 按键盘上的键（typeText 要找「有焦点的元素」，而接输入法的是一块藏起来的 UITextView，它不进无障碍树）。
        // type="digit" 是数字键盘（decimalPad）：小数点键的标签随地区是「.」或「,」
        for k in ["1", "2", ".", "5"] {
            let key = kb.keys[k].exists ? kb.keys[k] : kb.buttons[k]
            XCTAssertTrue(key.exists, "键盘上没有「\(k)」键（不是数字键盘？）")
            key.tap()
        }
        sleep(1)
        shot(app, "2-typed")
    }

    private func shot(_ app: XCUIApplication, _ name: String) {
        let dir = ProcessInfo.processInfo.environment["KIRO_UI_SHOTS"] ?? "/tmp"
        let png = app.screenshot().pngRepresentation
        try? png.write(to: URL(fileURLWithPath: "\(dir)/kb-\(name).png"))
        let a = XCTAttachment(data: png, uniformTypeIdentifier: "public.png")
        a.name = name
        a.lifetime = .keepAlways
        add(a)
    }
}
