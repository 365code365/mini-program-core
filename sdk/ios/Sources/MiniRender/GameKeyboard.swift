import UIKit

/// 小游戏的键盘（与 Android 的 GameKeyboard.kt 对应；引擎侧见 `src/js/prelude/game-keyboard.js`）。
///
/// 与微信一致：游戏画面不动，键盘上方浮一条原生输入框 + 确认键，游戏只收事件。
/// - `showKeyboard`：defaultValue / maxLength / multiple（多行）/ confirmType（done / next / search / go / send 对应回车键）；
/// - 输入每变一次 → `keyboardInput {value}`；回车（单行）或确认键 → `keyboardConfirm {value}`；
/// - 收起（hideKeyboard、用户把键盘收掉）→ `keyboardComplete {value}`；
/// - 键盘高度变化 → `keyboardHeightChange {height}`（逻辑像素），输入条贴着键盘顶上。
/// 输入条的样式没有取证（微信是白底 + 绿色确认键），按那个样子画个大概。主线程上用；[emit] 由调用方投递到引擎线程。
final class GameKeyboard: NSObject, UITextViewDelegate {
    private weak var host: UIView?
    private let emit: (_ name: String, _ json: String) -> Void
    private var bar: UIView?
    private var text: UITextView?
    private var button: UIButton?
    private var shown = false
    private var multiple = false
    private var maxLength = 100
    private var keyboardHeight: CGFloat = 0

    init(host: UIView, emit: @escaping (_ name: String, _ json: String) -> Void) {
        self.host = host
        self.emit = emit
        super.init()
        let nc = NotificationCenter.default
        nc.addObserver(self, selector: #selector(keyboardFrame(_:)), name: UIResponder.keyboardWillChangeFrameNotification, object: nil)
        nc.addObserver(self, selector: #selector(keyboardFrame(_:)), name: UIResponder.keyboardWillHideNotification, object: nil)
    }

    deinit { NotificationCenter.default.removeObserver(self) }

    private var value: String { text?.text ?? "" }

    private func send(_ name: String, _ key: String, _ v: Any) {
        emit(name, GameSensors.json([key: v]))
    }

    func show(_ o: [String: Any]) {
        guard let host else { return }
        let (bar, t, b) = ensureBar(host)
        multiple = o["multiple"] as? Bool ?? false
        maxLength = max(1, (o["maxLength"] as? NSNumber)?.intValue ?? 100)
        let type = o["confirmType"] as? String ?? "done"
        t.returnKeyType = multiple ? .default : [
            "next": UIReturnKeyType.next, "search": .search, "go": .go, "send": .send,
        ][type] ?? .done
        b.setTitle(["next": "下一步", "search": "搜索", "go": "前往", "send": "发送"][type] ?? "完成", for: .normal)
        t.text = String((o["defaultValue"] as? String ?? "").prefix(maxLength))
        bar.isHidden = false
        shown = true
        layout()
        t.becomeFirstResponder()
    }

    func update(_ o: [String: Any]) {
        text?.text = o["value"] as? String ?? ""
    }

    func hide() {
        guard shown else { return }
        shown = false
        let v = value
        text?.resignFirstResponder()
        bar?.isHidden = true
        send("keyboardComplete", "value", v)
    }

    private func ensureBar(_ host: UIView) -> (UIView, UITextView, UIButton) {
        if let bar, let text, let button { return (bar, text, button) }
        let bar = UIView()
        bar.backgroundColor = UIColor(red: 0xF7 / 255, green: 0xF7 / 255, blue: 0xF7 / 255, alpha: 1)
        let t = UITextView()
        t.backgroundColor = .white
        t.textColor = .black
        t.font = .systemFont(ofSize: 16)
        t.layer.cornerRadius = 4
        t.textContainerInset = UIEdgeInsets(top: 8, left: 6, bottom: 8, right: 6)
        t.delegate = self
        let b = UIButton(type: .system)
        b.backgroundColor = UIColor(red: 0x07 / 255, green: 0xC1 / 255, blue: 0x60 / 255, alpha: 1)
        b.setTitleColor(.white, for: .normal)
        b.layer.cornerRadius = 4
        b.addTarget(self, action: #selector(confirm), for: .touchUpInside)
        bar.addSubview(t)
        bar.addSubview(b)
        bar.isHidden = true
        host.addSubview(bar)
        self.bar = bar
        text = t
        button = b
        return (bar, t, b)
    }

    /// 输入条贴着键盘顶（键盘还没上来时贴底）
    func layout() {
        guard let host, let bar, let text, let button else { return }
        let w = host.bounds.width
        let h: CGFloat = multiple ? 96 : 52
        bar.frame = CGRect(x: 0, y: host.bounds.height - keyboardHeight - h, width: w, height: h)
        let bw: CGFloat = 72
        text.frame = CGRect(x: 8, y: 8, width: w - bw - 24, height: h - 16)
        button.frame = CGRect(x: w - bw - 8, y: (h - 36) / 2, width: bw, height: 36)
        host.bringSubviewToFront(bar)
    }

    @objc private func confirm() {
        if shown { send("keyboardConfirm", "value", value) }
    }

    @objc private func keyboardFrame(_ n: Notification) {
        guard let host, let window = host.window else { return }
        var height: CGFloat = 0
        if n.name != UIResponder.keyboardWillHideNotification, let end = n.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? CGRect {
            let f = host.convert(window.convert(end, from: nil), from: window)
            height = max(0, host.bounds.maxY - f.minY)
        }
        guard height != keyboardHeight else { return }
        let was = keyboardHeight
        keyboardHeight = height
        send("keyboardHeightChange", "height", Int(height))
        layout()
        // 用户自己把键盘收掉：当作收起
        if shown && was > 0 && height == 0 { hide() }
    }

    // ───────────────────────── UITextViewDelegate ─────────────────────────

    func textView(_ t: UITextView, shouldChangeTextIn range: NSRange, replacementText s: String) -> Bool {
        if s == "\n" && !multiple {
            confirm()
            return false
        }
        let cur = (t.text ?? "") as NSString
        return cur.length - range.length + (s as NSString).length <= maxLength
    }

    func textViewDidChange(_ t: UITextView) {
        if shown { send("keyboardInput", "value", value) }
    }
}
