import AppKit
import XCTest
@testable import MiniRender

/// 输入法对端（MiniKeyboard）：组字区要同步给引擎、候选窗要出在输入框那里。
/// 不起真的输入法：直接按系统输入法调 NSTextInputClient 的顺序调 setMarkedText / insertText。
final class MiniKeyboardTests: XCTestCase {
    private var window: NSWindow!
    private var host: NSView!
    private var sent: [String] = []
    private var kb: MiniKeyboard!

    override func setUp() {
        window = NSWindow(contentRect: NSRect(x: 100, y: 100, width: 900, height: 760), styleMask: [.titled], backing: .buffered, defer: false)
        host = FlippedHost(frame: window.contentView!.bounds)
        window.contentView!.addSubview(host)
        sent = []
        kb = MiniKeyboard(host: host, sendText: { [unowned self] in sent.append($0) }, sendKey: { [unowned self] in sent.append("<\($0)>") })
    }

    private func focus(originX: CGFloat = 0) {
        kb.onFocus(["id": "msg", "value": "", "cursor": 0, "tag": "input",
                    "rect": ["x": 16, "y": 700, "width": 300, "height": 40]], originX: originX)
    }

    func test拼音组字时就送给引擎_选字后换成汉字() {
        focus()
        kb.field.setMarkedText("ni", selectedRange: NSRange(location: 2, length: 0), replacementRange: NSRange(location: NSNotFound, length: 0))
        XCTAssertEqual(sent, ["ni"], "组字区一变就送（从前要等选完字）")
        kb.field.setMarkedText("ni h", selectedRange: NSRange(location: 4, length: 0), replacementRange: NSRange(location: NSNotFound, length: 0))
        XCTAssertEqual(sent, ["ni", " h"])
        kb.field.insertText("你好", replacementRange: NSRange(location: NSNotFound, length: 0))
        XCTAssertEqual(sent, ["ni", " h", "<backspace>", "<backspace>", "<backspace>", "<backspace>", "你好"])
        XCTAssertEqual(kb.field.string, "你好")
    }

    func test候选窗出在输入框那里_分栏时加上右栏的x() {
        focus(originX: 450)
        XCTAssertEqual(kb.field.frame, NSRect(x: 466, y: 700, width: 300, height: 40))
        kb.field.setMarkedText("ni", selectedRange: NSRange(location: 2, length: 0), replacementRange: NSRange(location: NSNotFound, length: 0))
        // 系统输入法按这个（屏幕坐标）摆候选窗：要落在输入框的屏幕矩形里
        let r = kb.field.firstRect(forCharacterRange: NSRange(location: 0, length: 2), actualRange: nil)
        let box = window.convertToScreen(host.convert(NSRect(x: 466, y: 700, width: 300, height: 40), to: nil))
        XCTAssertTrue(box.insetBy(dx: -1, dy: -1).contains(r), "组字区 \(r) 应在输入框 \(box) 里")
        XCTAssertNil(kb.field.hitTest(NSPoint(x: 470, y: 710)), "不抢输入框上的鼠标")
    }
}

private final class FlippedHost: NSView {
    override var isFlipped: Bool { true }
}
