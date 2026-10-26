import UIKit

/// 商店入口页与搜索页共用的零件：自动换行的标签行、标签、小节标题、小程序结果行、打开小程序。
/// 与 Android 示例 StoreWidgets.kt 同一套样子。
enum StoreWidgets {
    static let grayBg = UIColor(argb: 0xFFF7F7F7)
    static let sub = UIColor(argb: 0x8A000000)

    /// 打开一个商店小程序：记进「最近使用」，从下往上推出小程序页（微信也是这样）。
    /// [scene]：微信的场景值 —— 1001 小程序列表、1005 搜索、1089 下拉「最近使用」、1104 下拉「我的小程序」
    static func open(_ from: UIViewController, store: MiniStore, app: MiniStore.App, scene: Int = 1001) {
        StoreLocal().touch(app)
        let vc = MiniViewController(link: StoreLink(key: app.key, scene: scene), hint: app, store: store)
        present(vc, from: from)
    }

    /// 全屏、从下往上推（coverVertical 就是 Android 的 store_slide_up）
    static func present(_ vc: UIViewController, from: UIViewController) {
        vc.modalPresentationStyle = .fullScreen
        vc.modalTransitionStyle = .coverVertical
        var top = from
        while let p = top.presentedViewController { top = p }
        top.present(vc, animated: true)
    }

    static func chip(_ text: String, selected: Bool = false, onTap: @escaping () -> Void) -> UIButton {
        let b = UIButton(type: .custom)
        b.setTitle(text, for: .normal)
        b.titleLabel?.font = .systemFont(ofSize: 13)
        b.titleLabel?.lineBreakMode = .byTruncatingTail
        b.setTitleColor(selected ? StoreViews.green : UIColor(argb: 0xDE000000), for: .normal)
        b.backgroundColor = selected ? UIColor(argb: 0x1A07C160) : grayBg
        b.contentEdgeInsets = UIEdgeInsets(top: 6, left: 12, bottom: 6, right: 12)
        b.layer.cornerRadius = 15
        b.addAction(onTap)
        return b
    }

    /// 小节标题：左边粗体标题，右边可带一个灰色提示 / 按钮
    static func header(_ title: String, action: String? = nil, onAction: (() -> Void)? = nil) -> UIView {
        let row = UIStackView()
        row.alignment = .center
        row.isLayoutMarginsRelativeArrangement = true
        row.layoutMargins = UIEdgeInsets(top: 18, left: 16, bottom: 6, right: 8)
        let t = UILabel()
        t.text = title
        t.font = .boldSystemFont(ofSize: 14)
        t.textColor = .black
        row.addArrangedSubview(t)
        if let action {
            let b = UIButton(type: .custom)
            b.setTitle(action, for: .normal)
            b.titleLabel?.font = .systemFont(ofSize: 13)
            b.setTitleColor(sub, for: .normal)
            b.contentEdgeInsets = UIEdgeInsets(top: 4, left: 8, bottom: 4, right: 8)
            b.setContentHuggingPriority(.required, for: .horizontal)
            if let onAction { b.addAction(onAction) } else { b.isUserInteractionEnabled = false }
            row.addArrangedSubview(b)
        }
        return row
    }

    /// 搜索结果的一行：图标、名称（命中的字标绿）、分类 · 开发者、简介
    static func resultRow(_ vc: UIViewController, store: MiniStore, app: MiniStore.App, q: String) -> UIView {
        let row = PressableRow()
        row.accessibilityLabel = "打开小程序 \(app.name)"
        let h = UIStackView()
        h.alignment = .center
        h.spacing = 12
        h.isLayoutMarginsRelativeArrangement = true
        h.layoutMargins = UIEdgeInsets(top: 12, left: 16, bottom: 12, right: 16)
        h.isUserInteractionEnabled = false
        h.translatesAutoresizingMaskIntoConstraints = false
        row.addSubview(h)
        h.pinEdges(to: row)
        h.addArrangedSubview(IconView(store: store, app: app, size: 48))
        let col = UIStackView()
        col.axis = .vertical
        col.spacing = 2
        let name = UILabel()
        name.attributedText = highlight(app.name, q, base: .black)
        name.font = .systemFont(ofSize: 16)
        col.addArrangedSubview(name)
        let meta = [app.category, app.developer].filter { !$0.isEmpty }.joined(separator: " · ")
        if !meta.isEmpty { col.addArrangedSubview(label(meta, 12, sub)) }
        if !app.description.isEmpty { col.addArrangedSubview(label(app.description, 13, UIColor(argb: 0x99000000))) }
        h.addArrangedSubview(col)
        row.onTap = { [weak vc] in if let vc { open(vc, store: store, app: app, scene: 1005) } }
        // 长按：收藏到「我的小程序」（与入口页、下拉面板同一张操作单）
        row.onLongPress = { [weak vc] in if let vc { StoreSheets.appActions(vc, store: store, app: app, scene: 1005) } }
        return row
    }

    /// 名称里出现的关键词标绿（微信搜索结果就是这样）；每段关键词各标一次
    static func highlight(_ text: String, _ q: String, base: UIColor) -> NSAttributedString {
        let s = NSMutableAttributedString(string: text, attributes: [.foregroundColor: base])
        let ns = text as NSString
        for t in q.split(separator: " ") where !t.isEmpty {
            let r = ns.range(of: String(t), options: .caseInsensitive)
            if r.location != NSNotFound { s.addAttribute(.foregroundColor, value: StoreViews.green, range: r) }
        }
        return s
    }

    static func label(_ text: String, _ size: CGFloat, _ color: UIColor, bold: Bool = false, lines: Int = 1) -> UILabel {
        let l = UILabel()
        l.text = text
        l.font = bold ? .boldSystemFont(ofSize: size) : .systemFont(ofSize: size)
        l.textColor = color
        l.numberOfLines = lines
        l.lineBreakMode = .byTruncatingTail
        return l
    }
}

/// 一行可点、可长按的区域：按下时变灰（Android 的 list_selector_background）
class PressableRow: UIControl {
    var onTap: (() -> Void)?
    var onLongPress: (() -> Void)? {
        didSet {
            guard onLongPress != nil, gestureRecognizers?.contains(where: { $0 is UILongPressGestureRecognizer }) != true else { return }
            addGestureRecognizer(UILongPressGestureRecognizer(target: self, action: #selector(long(_:))))
        }
    }
    var pressedColor = UIColor(argb: 0x14000000)
    private var longFired = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        isAccessibilityElement = true
        accessibilityTraits = .button
        addTarget(self, action: #selector(tap), for: .touchUpInside)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    override var isHighlighted: Bool {
        didSet { backgroundColor = isHighlighted ? pressedColor : .clear }
    }

    override func beginTracking(_ touch: UITouch, with event: UIEvent?) -> Bool {
        longFired = false
        return super.beginTracking(touch, with: event)
    }

    @objc private func tap() {
        if !longFired { onTap?() }
    }

    @objc private func long(_ g: UILongPressGestureRecognizer) {
        guard g.state == .began else { return }
        longFired = true
        isHighlighted = false
        UIImpactFeedbackGenerator(style: .medium).impactOccurred()
        onLongPress?()
    }
}

/// 自动换行的一行行标签（搜索历史、热搜、分类）。子视图按自己的宽度排，放不下就折到下一行
final class FlowLayout: UIView {
    var gap: CGFloat = 8
    var insets = UIEdgeInsets(top: 4, left: 16, bottom: 4, right: 16)

    func setItems(_ views: [UIView]) {
        subviews.forEach { $0.removeFromSuperview() }
        views.forEach(addSubview)
        invalidateIntrinsicContentSize()
        setNeedsLayout()
    }

    private func arrange(width: CGFloat, apply: Bool) -> CGFloat {
        let maxW = width - insets.left - insets.right
        var x: CGFloat = 0, y: CGFloat = 0, lineH: CGFloat = 0
        for v in subviews {
            var s = v.sizeThatFits(CGSize(width: maxW, height: .greatestFiniteMagnitude))
            s.width = min(s.width, maxW)
            if x > 0, x + s.width > maxW {
                x = 0
                y += lineH + gap
                lineH = 0
            }
            if apply { v.frame = CGRect(x: insets.left + x, y: insets.top + y, width: s.width, height: s.height) }
            x += s.width + gap
            lineH = max(lineH, s.height)
        }
        return insets.top + y + lineH + insets.bottom
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        let h = arrange(width: bounds.width, apply: true)
        if abs(h - lastHeight) > 0.5 {
            lastHeight = h
            invalidateIntrinsicContentSize()
        }
    }

    private var lastHeight: CGFloat = 0

    override var intrinsicContentSize: CGSize {
        let w = bounds.width > 0 ? bounds.width : (superview?.bounds.width ?? UIScreen.main.bounds.width)
        return CGSize(width: UIView.noIntrinsicMetric, height: arrange(width: w, apply: false))
    }
}

extension UIView {
    func pinEdges(to v: UIView, insets: UIEdgeInsets = .zero) {
        translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            leadingAnchor.constraint(equalTo: v.leadingAnchor, constant: insets.left),
            trailingAnchor.constraint(equalTo: v.trailingAnchor, constant: -insets.right),
            topAnchor.constraint(equalTo: v.topAnchor, constant: insets.top),
            bottomAnchor.constraint(equalTo: v.bottomAnchor, constant: -insets.bottom),
        ])
    }
}
