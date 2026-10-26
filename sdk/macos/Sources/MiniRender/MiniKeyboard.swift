import AppKit

/// 输入框聚焦时把键盘接到引擎：系统输入法（拼音组字、联想、听写）、方向键、退格、回车、复制粘贴。
/// [MiniProgramView] 内部用。与 iOS 的 MiniKeyboard.swift、Android 的 MiniImeConnection.kt 同一个做法：
///
/// **输入法的对端是一块藏起来的 NSTextView**（[field]，透明、1pt）：它当 first responder，系统输入法按标准编辑框工作。
/// 它的文字是引擎里输入框值的镜像；每次变化都把「前后的差」换算成引擎按键 —— 删掉的字 → `backspace`，
/// 新出现的字 → 文字。拼音组字时组字区直接显示在输入框里（引擎没有组字区），上屏时换成最终的字。
///
/// 只挪光标、不改字的命令（方向键、Home / End、全选、Esc）在 [doCommandBy] 里截下来转成引擎的功能键名，
/// NSTextView 自己也照做一遍，镜像里的光标跟引擎保持一致。
final class MiniKeyboard: NSObject, NSTextViewDelegate {
    private let sendText: (String) -> Void
    private let sendKey: (String) -> Void
    let field = NSTextView(frame: NSRect(x: 0, y: 0, width: 1, height: 1))
    private(set) var focus: [String: Any]?
    private var mirror = ""
    private var applying = false

    init(host: NSView, sendText: @escaping (String) -> Void, sendKey: @escaping (String) -> Void) {
        self.sendText = sendText
        self.sendKey = sendKey
        super.init()
        field.delegate = self
        field.alphaValue = 0
        field.isRichText = false
        field.isAutomaticQuoteSubstitutionEnabled = false
        field.isAutomaticDashSubstitutionEnabled = false
        field.isAutomaticTextReplacementEnabled = false
        field.isAutomaticSpellingCorrectionEnabled = false
        field.isContinuousSpellCheckingEnabled = false
        field.allowsUndo = false
        field.setAccessibilityElement(false)
        host.addSubview(field)
    }

    var active: Bool { focus != nil }

    func onFocus(_ ev: [String: Any]) {
        focus = ev
        setMirror(ev["value"] as? String ?? "", cursor: (ev["cursor"] as? NSNumber)?.intValue)
        if field.window?.firstResponder !== field { field.window?.makeFirstResponder(field) }
    }

    /// 页面改了聚焦中输入框的值（`setData` 格式化、`bindinput` 返回新值）：换掉镜像，不然下一次按旧内容算差
    func onValue(_ ev: [String: Any]) {
        guard let f = focus, f["id"] as? String == ev["id"] as? String, !field.hasMarkedText() else { return }
        setMirror(ev["value"] as? String ?? "", cursor: (ev["cursor"] as? NSNumber)?.intValue)
    }

    func onBlur() {
        guard focus != nil else { return }
        focus = nil
        setMirror("", cursor: 0)
        if field.window?.firstResponder === field { field.window?.makeFirstResponder(field.superview) }
    }

    private func setMirror(_ value: String, cursor: Int?) {
        applying = true
        if field.hasMarkedText() { field.unmarkText() }
        field.string = value
        mirror = value
        let n = (value as NSString).length
        field.setSelectedRange(NSRange(location: min(max(cursor ?? n, 0), n), length: 0))
        applying = false
    }

    // MARK: - 输入法 → 引擎

    func textDidChange(_ notification: Notification) {
        if applying { return }
        let after = field.string
        sync(from: mirror, to: after)
        mirror = after
    }

    func textView(_ tv: NSTextView, shouldChangeTextIn range: NSRange, replacementString text: String?) -> Bool {
        focus != nil
    }

    /// 不改字的命令：转成引擎的功能键名（键名见 mini_render.h 的 mr_app_key）
    func textView(_ tv: NSTextView, doCommandBy sel: Selector) -> Bool {
        let textarea = focus?["tag"] as? String == "textarea"
        let map: [Selector: String] = [
            #selector(NSResponder.moveLeft(_:)): "left",
            #selector(NSResponder.moveRight(_:)): "right",
            #selector(NSResponder.moveUp(_:)): "up",
            #selector(NSResponder.moveDown(_:)): "down",
            #selector(NSResponder.moveToBeginningOfLine(_:)): "home",
            #selector(NSResponder.moveToEndOfLine(_:)): "end",
            #selector(NSResponder.moveToLeftEndOfLine(_:)): "home",
            #selector(NSResponder.moveToRightEndOfLine(_:)): "end",
            #selector(NSResponder.moveWordLeft(_:)): "word-left",
            #selector(NSResponder.moveWordRight(_:)): "word-right",
            #selector(NSResponder.moveToBeginningOfDocument(_:)): "doc-start",
            #selector(NSResponder.moveToEndOfDocument(_:)): "doc-end",
            #selector(NSResponder.moveLeftAndModifySelection(_:)): "shift+left",
            #selector(NSResponder.moveRightAndModifySelection(_:)): "shift+right",
            #selector(NSResponder.selectAll(_:)): "selectall",
        ]
        if let k = map[sel] {
            sendKey(k)
            return false // NSTextView 自己也挪光标：镜像的光标与引擎一致
        }
        switch sel {
        case #selector(NSResponder.insertNewline(_:)), #selector(NSResponder.insertLineBreak(_:)):
            // 单行输入框的回车 = 确认；textarea 换行走文字差（"\n"）
            if !textarea {
                sendKey("enter")
                return true
            }
            return false
        case #selector(NSResponder.cancelOperation(_:)):
            // Esc：输入框失焦（引擎派发 bindblur、回 inputBlur）
            sendKey("escape")
            return true
        default:
            return false
        }
    }

    /// 取公共前后缀，中间删掉的按退格送、新出现的按文字送
    private func sync(from before: String, to after: String) {
        if before == after { return }
        let b = Array(before), a = Array(after)
        var p = 0
        while p < min(b.count, a.count), b[p] == a[p] { p += 1 }
        var q = 0
        while q < min(b.count, a.count) - p, b[b.count - 1 - q] == a[a.count - 1 - q] { q += 1 }
        for _ in 0..<(b.count - p - q) { sendKey("backspace") }
        let added = String(a[p..<(a.count - q)])
        if !added.isEmpty { sendText(added) }
    }
}
