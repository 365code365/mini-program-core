import AppKit

/// 输入框聚焦时把键盘接到引擎：系统输入法（拼音组字、联想、听写）、方向键、退格、回车、复制粘贴。
/// [MiniProgramView] 内部用。与 iOS 的 MiniKeyboard.swift、Android 的 MiniImeConnection.kt 同一个做法：
///
/// **输入法的对端是一块藏起来的 NSTextView**（[field]，透明、1pt）：它当 first responder，系统输入法按标准编辑框工作。
/// 它的文字是引擎里输入框值的镜像；每次变化都把「前后的差」换算成引擎按键 —— 删掉的字 → `backspace`，
/// 新出现的字 → 文字。拼音组字时组字区直接显示在输入框里（引擎没有组字区），上屏时换成最终的字。
///
/// **候选窗的位置**：系统输入法把候选窗摆在对端编辑框的组字区下面（`firstRect(forCharacterRange:)`）。
/// 从前这块编辑框停在视图左上角 (0,0)，候选窗就压在窗口标题栏上、离输入框老远。现在聚焦时把它挪到
/// 输入框那个矩形上（`inputFocus` 事件的 `rect`，分栏时加上活页面那一栏的 x），字号与输入框相近，
/// 候选窗就出在输入框里正在打的字下面。
///
/// **组字也要同步**：NSTextView 改组字区（`setMarkedText`）时不一定发 `textDidChange`，
/// 只靠通知的话拼音打了半天输入框里还是占位文字，选完字才一下子出来。所以 [MirrorTextView] 在组字区、
/// 上屏、取消组字之后都回调一次，与通知一起按「前后的差」送给引擎（差为空时什么都不送，重复回调无害）。
///
/// 只挪光标、不改字的命令（方向键、Home / End、全选、Esc）在 [doCommandBy] 里截下来转成引擎的功能键名，
/// NSTextView 自己也照做一遍，镜像里的光标跟引擎保持一致。
final class MiniKeyboard: NSObject, NSTextViewDelegate {
    private let sendText: (String) -> Void
    private let sendKey: (String) -> Void
    let field = MirrorTextView(frame: NSRect(x: 0, y: 0, width: 1, height: 1))
    private(set) var focus: [String: Any]?
    private var mirror = ""
    private var applying = false

    init(host: NSView, sendText: @escaping (String) -> Void, sendKey: @escaping (String) -> Void) {
        self.sendText = sendText
        self.sendKey = sendKey
        super.init()
        field.delegate = self
        field.onEdit = { [weak self] in self?.textChanged() }
        field.drawsBackground = false
        field.font = .systemFont(ofSize: 15)
        field.textContainer?.lineFragmentPadding = 0
        // 不折行（单行输入框打长了也在一行上，组字区位置跟输入框里的一致）
        field.isHorizontallyResizable = true
        field.textContainer?.widthTracksTextView = false
        field.textContainer?.containerSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
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

    /// `originX`：活页面那一栏在视图里的左边 x（分栏时右栏是一栏宽，不分栏为 0）
    func onFocus(_ ev: [String: Any], originX: CGFloat = 0) {
        focus = ev
        place(ev["rect"] as? [String: Any], originX: originX)
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

    /// 对端编辑框盖到输入框上（透明，看不见）：候选窗跟着它的组字区走
    private func place(_ r: [String: Any]?, originX: CGFloat) {
        let n = { (k: String) in CGFloat((r?[k] as? NSNumber)?.doubleValue ?? 0) }
        let h = max(n("height"), 20)
        field.frame = NSRect(x: originX + n("x"), y: n("y"), width: max(n("width"), 40), height: h)
        // 一行字竖直居中，组字区的下沿就在输入框里那行字下面
        let line = field.font.map { $0.ascender - $0.descender + $0.leading } ?? 18
        field.textContainerInset = NSSize(width: 0, height: max(0, (h - line) / 2))
    }

    // MARK: - 输入法 → 引擎

    func textDidChange(_ notification: Notification) {
        textChanged()
    }

    private func textChanged() {
        if applying || focus == nil { return }
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

/// 对端编辑框：组字区变化（拼音打到一半）、上屏、取消组字之后都回调 [onEdit]
final class MirrorTextView: NSTextView {
    var onEdit: (() -> Void)?

    /// 盖在输入框上但不接鼠标：点输入框挪光标、拖选仍交给引擎
    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    override func setMarkedText(_ string: Any, selectedRange: NSRange, replacementRange: NSRange) {
        super.setMarkedText(string, selectedRange: selectedRange, replacementRange: replacementRange)
        onEdit?()
    }

    override func unmarkText() {
        super.unmarkText()
        onEdit?()
    }

    override func insertText(_ string: Any, replacementRange: NSRange) {
        super.insertText(string, replacementRange: replacementRange)
        onEdit?()
    }
}
