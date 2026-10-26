import UIKit

/// 输入框聚焦时弹系统键盘，把输入法交来的字交给引擎；键盘挡住输入框时把画面顶上去（`adjust-position`）。
/// [MiniProgramView] 内部用。与 Android 的 MiniImeConnection.kt + MiniKeyboard.kt 同一套规则。
///
/// 从前 iOS 的原生视图什么都没接：点输入框只有光标、不弹键盘（水费缴费页「其他金额」）。Flutter 插件有自己的
/// TextInput，所以 Flutter 示例上看不出来。
///
/// **输入法的对端是一块藏起来的 UITextView**（[field]，透明、1pt）：系统键盘、拼音组字（marked text）、
/// 联想、听写都按标准编辑框工作。它的文字是引擎里输入框值的镜像；每一次变化都把「前后的差」换算成引擎按键 ——
/// 删掉的字 → `backspace`，新出现的字 → 文字。拼音组字时组字区直接显示在输入框里（引擎没有组字区），
/// 上屏时换成最终的字（与 Android 同一个做法）。单行输入框的回车 → `enter`（引擎据此触发 confirm）。
///
/// 顶起交给引擎（`mr_app_set_keyboard_shift`）：引擎合成时把导航栏以下整体上移、自己按帧缓动、触摸坐标自己换回去。
/// 视图本身不动、不改尺寸（与微信一致）。键盘挡住视图的高度另外推给小程序的 `wx.onKeyboardHeightChange`。
final class MiniKeyboard: NSObject, UITextViewDelegate {
    private weak var host: UIView?
    private let sendText: (String) -> Void
    private let sendKey: (String) -> Void
    /// 目标上移量（逻辑像素，0 = 收回）
    private let onShift: (CGFloat) -> Void
    /// 键盘挡住视图的高度（逻辑像素）：`keyboardHeightChange`
    private let onHeight: (CGFloat) -> Void

    let field = UITextView(frame: CGRect(x: 0, y: 0, width: 1, height: 1))
    /// 当前聚焦的输入框（`inputFocus` 事件）；nil = 没有，键盘收起
    private var focus: [String: Any]?
    /// 最近一次聚焦的时刻：换输入框时键盘会先收再弹，这一下不算用户收起
    private var focusAt = Date.distantPast
    /// 镜像（输入法改 field 之前的样子）：差就按它算
    private var mirror = ""
    /// 自己改 field 的文字时不当成用户输入
    private var applying = false
    /// 键盘在屏幕上的位置（屏幕坐标）；收起时为 nil
    private var keyboardFrame: CGRect?
    private var reported: CGFloat = -1
    private var shift: CGFloat = 0

    init(host: UIView, sendText: @escaping (String) -> Void, sendKey: @escaping (String) -> Void,
         onShift: @escaping (CGFloat) -> Void, onHeight: @escaping (CGFloat) -> Void) {
        self.host = host
        self.sendText = sendText
        self.sendKey = sendKey
        self.onShift = onShift
        self.onHeight = onHeight
        super.init()
        field.delegate = self
        // 透明但留在层级里：alpha 0 的视图照样能当 first responder；不藏会露出系统的光标与选区
        field.alpha = 0
        field.autocorrectionType = .no
        field.spellCheckingType = .no
        field.smartQuotesType = .no
        field.smartDashesType = .no
        field.isAccessibilityElement = false
        host.addSubview(field)
        let nc = NotificationCenter.default
        nc.addObserver(self, selector: #selector(keyboardFrameChanged(_:)), name: UIResponder.keyboardWillChangeFrameNotification, object: nil)
        nc.addObserver(self, selector: #selector(keyboardHidden(_:)), name: UIResponder.keyboardWillHideNotification, object: nil)
    }

    deinit {
        NotificationCenter.default.removeObserver(self)
    }

    var active: Bool { focus != nil }

    // MARK: - 引擎事件

    func onFocus(_ ev: [String: Any]) {
        let changed = focus?["id"] as? String != ev["id"] as? String || focus?["inputType"] as? String != ev["inputType"] as? String
        focus = ev
        focusAt = Date()
        configure(ev)
        setMirror(ev["value"] as? String ?? "", cursor: (ev["cursor"] as? NSNumber)?.intValue)
        if !field.isFirstResponder {
            field.becomeFirstResponder()
        } else if changed {
            // 换了输入框 / 键盘类型：让键盘按新的类型重来（数字键盘、密码、回车键文字）
            field.reloadInputViews()
        }
        apply()
    }

    /// 页面改了聚焦中输入框的值（`setData` 格式化手机号、`bindinput` 返回新值）：换掉镜像。
    /// 不换的话下一次按旧内容算差，字落到错的位置
    func onValue(_ ev: [String: Any]) {
        guard let f = focus, f["id"] as? String == ev["id"] as? String, field.markedTextRange == nil else { return }
        setMirror(ev["value"] as? String ?? "", cursor: (ev["cursor"] as? NSNumber)?.intValue)
    }

    func onBlur() {
        guard focus != nil else { return }
        focus = nil
        setMirror("", cursor: 0)
        if field.isFirstResponder { field.resignFirstResponder() }
        apply()
    }

    private func configure(_ f: [String: Any]) {
        let textarea = f["tag"] as? String == "textarea"
        switch f["inputType"] as? String {
        case "number": field.keyboardType = .numberPad
        case "digit": field.keyboardType = .decimalPad
        case "idcard": field.keyboardType = .asciiCapable
        default: field.keyboardType = .default
        }
        field.isSecureTextEntry = f["password"] as? Bool ?? false
        switch f["confirmType"] as? String {
        case "search": field.returnKeyType = .search
        case "send": field.returnKeyType = .send
        case "next": field.returnKeyType = .next
        case "go": field.returnKeyType = .go
        case "done": field.returnKeyType = .done
        default: field.returnKeyType = textarea ? .default : .done
        }
    }

    private func setMirror(_ value: String, cursor: Int?) {
        applying = true
        if field.markedTextRange != nil { field.unmarkText() }
        field.text = value
        mirror = value
        let n = (value as NSString).length
        let at = min(max(cursor ?? n, 0), n)
        field.selectedRange = NSRange(location: at, length: 0)
        applying = false
    }

    // MARK: - 输入法 → 引擎

    func textView(_ tv: UITextView, shouldChangeTextIn range: NSRange, replacementText text: String) -> Bool {
        // 单行输入框的回车 = 确认：不进镜像（多一个换行，之后的差就对不上了）
        if text == "\n", focus?["tag"] as? String != "textarea" {
            sendKey("enter")
            return false
        }
        return focus != nil
    }

    func textViewDidChange(_ tv: UITextView) {
        if applying { return }
        let after = tv.text ?? ""
        sync(from: mirror, to: after)
        mirror = after
    }

    /// 取公共前后缀，中间删掉的按退格送、新出现的按文字送 —— 输入法的编辑都在光标处，引擎光标就在那段的末尾
    private func sync(from before: String, to after: String) {
        if before == after { return }
        let b = Array(before), a = Array(after)
        var p = 0
        while p < min(b.count, a.count), b[p] == a[p] { p += 1 }
        var q = 0
        while q < min(b.count, a.count) - p, b[b.count - 1 - q] == a[a.count - 1 - q] { q += 1 }
        let removed = b.count - p - q
        let added = String(a[p..<(a.count - q)])
        for _ in 0..<removed { sendKey("backspace") }
        if !added.isEmpty { sendText(added) }
    }

    // MARK: - 键盘高度与顶起

    @objc private func keyboardFrameChanged(_ n: Notification) {
        guard let r = (n.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? NSValue)?.cgRectValue else { return }
        keyboardFrame = r
        report()
        apply()
    }

    @objc private func keyboardHidden(_ n: Notification) {
        let wasShown = (keyboardFrame?.height ?? 0) > 0 && overlap() > 0
        keyboardFrame = nil
        report()
        apply()
        // 输入框还聚焦着、键盘却被用户收掉了（下滑收起）：与微信一致，输入框失焦。
        // 不失焦的话引擎以为键盘还在：光标一直闪，再点输入框只是「挪光标」，键盘就弹不出来了。
        // **只认「之前真弹出来过」的收起**：接着外接键盘（模拟器默认就是）时系统不弹软键盘，
        // 聚焦那一刻就发一条高度为 0 的 willHide —— 当成用户收起的话输入框刚聚焦就被失焦，敲不了字
        if wasShown, focus != nil, Date().timeIntervalSince(focusAt) > 0.5 {
            sendKey("escape")
        }
    }

    /// 键盘挡住视图的高度（逻辑像素）
    private func overlap() -> CGFloat {
        guard let host, let win = host.window, let kb = keyboardFrame, kb.height > 0 else { return 0 }
        let mine = host.convert(host.bounds, to: win)
        let kbInWin = win.convert(kb, from: win.screen.coordinateSpace)
        return max(0, mine.maxY - kbInWin.minY)
    }

    private func report() {
        let h = overlap().rounded()
        if h != reported {
            reported = h
            onHeight(h)
        }
    }

    private func apply() {
        let t = target()
        if t == shift { return }
        shift = t
        onShift(t)
    }

    /// 该上移多少：键盘挡住输入框底边（+ cursor-spacing）的那一截，最多键盘挡住视图的高度
    private func target() -> CGFloat {
        guard let f = focus, f["adjustPosition"] as? Bool ?? true, let host else { return 0 }
        let cover = overlap()
        guard cover > 0, let r = f["rect"] as? [String: Any] else { return 0 }
        let num = { (k: String) in CGFloat((r[k] as? NSNumber)?.doubleValue ?? 0) }
        let spacing = CGFloat((f["cursorSpacing"] as? NSNumber)?.doubleValue ?? 0)
        // 输入框与键盘之间至少留一点（微信实测约 10px）
        let need = num("y") + num("height") + spacing + 10 - (host.bounds.height - cover)
        return min(max(need, 0), cover)
    }
}
