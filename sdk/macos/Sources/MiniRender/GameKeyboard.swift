import AppKit

/// 小游戏的键盘（macOS；与 iOS / Android 的 GameKeyboard 同一套事件，引擎侧见 `src/js/prelude/game-keyboard.js`）。
///
/// PC 微信里 wx.showKeyboard 也是在游戏窗口底部浮一条原生输入框 + 确认键：游戏画面不动，游戏只收事件。
/// - `showKeyboard`：defaultValue / maxLength / multiple / confirmType（按钮文字）；
/// - 输入每变一次 → `keyboardInput {value}`；回车（单行）或确认键 → `keyboardConfirm {value}`；
/// - 收起（hideKeyboard、Esc）→ `keyboardComplete {value}`；
/// - 桌面没有软键盘：`keyboardHeightChange` 报输入条的高度（游戏据此把输入框挪到它上面）。
/// 主线程上用；[emit] 由调用方投递到引擎线程。
final class GameKeyboard: NSObject, NSTextFieldDelegate {
    private weak var host: NSView?
    private let emit: (_ name: String, _ json: String) -> Void
    private var bar: NSView?
    private var field: NSTextField?
    private var button: NSButton?
    private var shown = false
    private var multiple = false
    private var maxLength = 100
    private let barHeight: CGFloat = 48

    init(host: NSView, emit: @escaping (_ name: String, _ json: String) -> Void) {
        self.host = host
        self.emit = emit
    }

    private var value: String { field?.stringValue ?? "" }

    private func send(_ name: String, _ key: String, _ v: Any) {
        let j = (try? JSONSerialization.data(withJSONObject: [key: v])).map { String(decoding: $0, as: UTF8.self) } ?? "{}"
        emit(name, j)
    }

    func show(_ o: [String: Any]) {
        guard let host else { return }
        let (bar, f, b) = ensureBar(host)
        multiple = o["multiple"] as? Bool ?? false
        maxLength = max(1, (o["maxLength"] as? NSNumber)?.intValue ?? 100)
        let type = o["confirmType"] as? String ?? "done"
        b.title = ["next": "下一步", "search": "搜索", "go": "前往", "send": "发送"][type] ?? "完成"
        f.stringValue = String((o["defaultValue"] as? String ?? "").prefix(maxLength))
        f.usesSingleLineMode = !multiple
        bar.isHidden = false
        shown = true
        layout()
        host.window?.makeFirstResponder(f)
        send("keyboardHeightChange", "height", Int(barHeight))
    }

    func update(_ o: [String: Any]) {
        field?.stringValue = o["value"] as? String ?? ""
    }

    func hide() {
        guard shown else { return }
        shown = false
        let v = value
        bar?.isHidden = true
        if let h = host { h.window?.makeFirstResponder(h) }
        send("keyboardHeightChange", "height", 0)
        send("keyboardComplete", "value", v)
    }

    private func ensureBar(_ host: NSView) -> (NSView, NSTextField, NSButton) {
        if let bar, let field, let button { return (bar, field, button) }
        let bar = NSView()
        bar.wantsLayer = true
        bar.layer?.backgroundColor = NSColor(white: 0xF7 / 255, alpha: 1).cgColor
        let f = NSTextField()
        f.font = .systemFont(ofSize: 14)
        f.focusRingType = .none
        f.delegate = self
        let b = NSButton(title: "完成", target: self, action: #selector(confirm))
        b.bezelStyle = .rounded
        b.bezelColor = NSColor(red: 0x07 / 255, green: 0xC1 / 255, blue: 0x60 / 255, alpha: 1)
        b.keyEquivalent = ""
        bar.addSubview(f)
        bar.addSubview(b)
        bar.isHidden = true
        host.addSubview(bar)
        self.bar = bar
        field = f
        button = b
        return (bar, f, b)
    }

    /// 输入条贴着视图底边（视图是 flipped 的）
    func layout() {
        guard let host, let bar, let field, let button else { return }
        let w = host.bounds.width
        bar.frame = NSRect(x: 0, y: host.bounds.height - barHeight, width: w, height: barHeight)
        let bw: CGFloat = 72
        field.frame = NSRect(x: 10, y: 10, width: w - bw - 30, height: barHeight - 20)
        button.frame = NSRect(x: w - bw - 10, y: 8, width: bw, height: barHeight - 16)
        host.addSubview(bar, positioned: .above, relativeTo: nil)
    }

    @objc private func confirm() {
        if shown { send("keyboardConfirm", "value", value) }
    }

    // ───────────────────────── NSTextFieldDelegate ─────────────────────────

    func controlTextDidChange(_ n: Notification) {
        guard let f = field else { return }
        if (f.stringValue as NSString).length > maxLength { f.stringValue = String(f.stringValue.prefix(maxLength)) }
        if shown { send("keyboardInput", "value", value) }
    }

    func control(_ control: NSControl, textView: NSTextView, doCommandBy sel: Selector) -> Bool {
        switch sel {
        case #selector(NSResponder.insertNewline(_:)):
            if multiple { return false }
            confirm()
            return true
        case #selector(NSResponder.cancelOperation(_:)):
            hide()
            return true
        default:
            return false
        }
    }
}
