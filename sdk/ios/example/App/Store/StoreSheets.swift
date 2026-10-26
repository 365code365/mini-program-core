import UIKit

/// 两个底部弹层，照微信的样子（与 Android 示例 StoreSheets.kt、Flutter 示例 lib/store/store_sheets.dart 同一套）：
/// - [appActions]：长按一个小程序 —— 添加到 / 移出「我的小程序」、从「最近使用」删除、打开；
/// - [capsuleMenu]：小程序里胶囊的「•••」—— 顶上小程序名（点了看关于），两排圆角方块，最下面隔开一条「取消」。
enum StoreSheets {
    static let sheetBg = UIColor(argb: 0xFFF7F7F7)
    static let danger = UIColor(argb: 0xFFFA5151)
    static let gold = UIColor(argb: 0xFFFFB400)

    /// 胶囊菜单的一格。[glyph] 是一个字符图标（与 Android 示例同一组字符）
    struct Item {
        let glyph: String
        let label: String
        var enabled = true
        var active = false
        let onTap: () -> Void
    }

    static func appActions(_ vc: UIViewController, store: MiniStore, app: MiniStore.App, scene: Int = 1001) {
        let local = StoreLocal()
        let fav = local.isFavorite(app.id)
        let inRecent = local.recent().contains { $0.id == app.id }
        let sheet = BottomSheet()
        let head = UIStackView()
        head.alignment = .center
        head.spacing = 12
        head.backgroundColor = .white
        head.isLayoutMarginsRelativeArrangement = true
        head.layoutMargins = UIEdgeInsets(top: 18, left: 20, bottom: 16, right: 20)
        head.addArrangedSubview(IconView(store: store, app: app, size: 44))
        let col = UIStackView()
        col.axis = .vertical
        col.spacing = 3
        col.addArrangedSubview(StoreWidgets.label(app.name, 16, .black, bold: true))
        let sub = app.description.isEmpty ? app.developer : app.description
        if !sub.isEmpty { col.addArrangedSubview(StoreWidgets.label(sub, 12, UIColor(argb: 0x73000000))) }
        head.addArrangedSubview(col)
        sheet.add(head)
        sheet.add(divider())
        sheet.add(option(fav ? "从「我的小程序」移除" : "添加到「我的小程序」") { [weak sheet] in
            sheet?.dismiss {
                let now = local.toggleFavorite(app)
                toast(now ? "已添加到「我的小程序」，下拉入口页可见" : "已从「我的小程序」移除")
            }
        })
        if inRecent {
            sheet.add(divider())
            sheet.add(option("从「最近使用」删除", color: danger) { [weak sheet] in
                sheet?.dismiss {
                    local.removeRecent(app.id)
                    toast("已从「最近使用」删除")
                }
            })
        }
        sheet.add(divider())
        sheet.add(option("打开") { [weak sheet, weak vc] in
            sheet?.dismiss { if let vc { StoreWidgets.open(vc, store: store, app: app, scene: scene) } }
        })
        sheet.addGap(8)
        sheet.add(option("取消") { [weak sheet] in sheet?.dismiss() })
        sheet.show(from: vc)
    }

    static func capsuleMenu(_ vc: UIViewController, icon: UIView, name: String, subtitle: String,
                            onHeader: @escaping () -> Void, top: [Item], bottom: [Item]) {
        let sheet = BottomSheet()
        let head = PressableRow()
        head.accessibilityLabel = "关于\(name)"
        let h = UIStackView()
        h.alignment = .center
        h.isUserInteractionEnabled = false
        h.isLayoutMarginsRelativeArrangement = true
        h.layoutMargins = UIEdgeInsets(top: 16, left: 20, bottom: 14, right: 16)
        head.addSubview(h)
        h.pinEdges(to: head)
        icon.translatesAutoresizingMaskIntoConstraints = false
        icon.widthAnchor.constraint(equalToConstant: icon.bounds.width).isActive = true
        icon.heightAnchor.constraint(equalToConstant: icon.bounds.height).isActive = true
        h.addArrangedSubview(icon)
        h.setCustomSpacing(10, after: icon)
        let n = StoreWidgets.label(name, 15, .black, bold: true)
        h.addArrangedSubview(n)
        h.setCustomSpacing(2, after: n)
        let chevron = StoreWidgets.label("›", 18, UIColor(argb: 0x61000000))
        chevron.setContentHuggingPriority(.defaultLow, for: .horizontal)
        h.addArrangedSubview(chevron)
        h.addArrangedSubview(StoreWidgets.label(subtitle, 12, UIColor(argb: 0x61000000)))
        head.onTap = { [weak sheet] in sheet?.dismiss(onHeader) }
        sheet.add(head)
        sheet.add(divider(inset: 20))
        sheet.add(row(top, sheet))
        sheet.add(divider(inset: 20))
        sheet.add(row(bottom, sheet))
        sheet.addGap(8)
        sheet.add(option("取消") { [weak sheet] in sheet?.dismiss() })
        sheet.show(from: vc)
    }

    /// 转发：微信是发给好友；示例复制一条能打开它的链接（外部 App / 浏览器里点开即进小程序）
    static func copyLink(_ app: MiniStore.App, route: String?) {
        var c = URLComponents()
        c.queryItems = [URLQueryItem(name: "appid", value: app.key)]
        if let route, !route.isEmpty { c.queryItems?.append(URLQueryItem(name: "path", value: route)) }
        UIPasteboard.general.string = "minirender://open?" + (c.percentEncodedQuery ?? "")
        toast("链接已复制：在别的 App 或浏览器里打开即可进入「\(app.name)」")
    }

    /// Android 的 Toast：屏幕下方一条半透明黑底白字，2 秒后淡出
    static func toast(_ s: String) {
        guard let win = UIApplication.shared.connectedScenes.compactMap({ ($0 as? UIWindowScene)?.windows.first { $0.isKeyWindow } }).first else { return }
        let l = PaddedLabel()
        l.text = s
        l.font = .systemFont(ofSize: 14)
        l.textColor = .white
        l.numberOfLines = 0
        l.textAlignment = .center
        l.backgroundColor = UIColor(argb: 0xCC333333)
        l.layer.cornerRadius = 18
        l.layer.masksToBounds = true
        l.alpha = 0
        l.translatesAutoresizingMaskIntoConstraints = false
        win.addSubview(l)
        NSLayoutConstraint.activate([
            l.centerXAnchor.constraint(equalTo: win.centerXAnchor),
            l.widthAnchor.constraint(lessThanOrEqualTo: win.widthAnchor, constant: -64),
            l.bottomAnchor.constraint(equalTo: win.safeAreaLayoutGuide.bottomAnchor, constant: -64),
        ])
        UIView.animate(withDuration: 0.2, animations: { l.alpha = 1 }) { _ in
            UIView.animate(withDuration: 0.25, delay: 2, options: [], animations: { l.alpha = 0 }) { _ in l.removeFromSuperview() }
        }
    }

    // MARK: - 零件

    private static func row(_ items: [Item], _ sheet: BottomSheet) -> UIView {
        let scroll = UIScrollView()
        scroll.showsHorizontalScrollIndicator = false
        let h = UIStackView()
        h.isLayoutMarginsRelativeArrangement = true
        h.layoutMargins = UIEdgeInsets(top: 16, left: 12, bottom: 14, right: 12)
        scroll.addSubview(h)
        h.pinEdges(to: scroll)
        h.heightAnchor.constraint(equalTo: scroll.heightAnchor).isActive = true
        for it in items { h.addArrangedSubview(cell(it, sheet)) }
        scroll.heightAnchor.constraint(equalToConstant: 16 + 52 + 8 + 30 + 14).isActive = true
        return scroll
    }

    private static func cell(_ it: Item, _ sheet: BottomSheet) -> UIView {
        let c = PressableRow()
        c.pressedColor = .clear
        c.isEnabled = it.enabled
        c.accessibilityLabel = it.label.replacingOccurrences(of: "\n", with: "")
        let v = UIStackView()
        v.axis = .vertical
        v.alignment = .center
        v.spacing = 8
        v.isUserInteractionEnabled = false
        c.addSubview(v)
        v.pinEdges(to: c)
        let g = UILabel()
        g.text = it.glyph
        g.font = .systemFont(ofSize: 22)
        g.textAlignment = .center
        g.textColor = !it.enabled ? UIColor(argb: 0x42000000) : it.active ? gold : UIColor(argb: 0xDE000000)
        g.backgroundColor = .white
        g.layer.cornerRadius = 12
        g.layer.masksToBounds = true
        g.widthAnchor.constraint(equalToConstant: 52).isActive = true
        g.heightAnchor.constraint(equalToConstant: 52).isActive = true
        v.addArrangedSubview(g)
        let l = StoreWidgets.label(it.label, 11, it.enabled ? UIColor(argb: 0x8A000000) : UIColor(argb: 0x42000000), lines: 2)
        l.textAlignment = .center
        v.addArrangedSubview(l)
        c.widthAnchor.constraint(equalToConstant: 76).isActive = true
        if it.enabled { c.onTap = { [weak sheet] in sheet?.dismiss(it.onTap) } }
        return c
    }

    private static func option(_ title: String, color: UIColor = UIColor(argb: 0xDE000000), onTap: @escaping () -> Void) -> UIView {
        let r = PressableRow()
        r.backgroundColor = .white
        r.accessibilityLabel = title
        let l = StoreWidgets.label(title, 17, color)
        l.textAlignment = .center
        r.addSubview(l)
        l.pinEdges(to: r)
        r.heightAnchor.constraint(greaterThanOrEqualToConstant: 56).isActive = true
        r.onTap = onTap
        return r
    }

    private static func divider(inset: CGFloat = 0) -> UIView {
        let wrap = UIView()
        wrap.backgroundColor = .white
        let line = UIView()
        line.backgroundColor = UIColor(argb: 0x1F000000)
        wrap.addSubview(line)
        line.pinEdges(to: wrap, insets: UIEdgeInsets(top: 0, left: inset, bottom: 0, right: inset))
        wrap.heightAnchor.constraint(equalToConstant: 1 / UIScreen.main.scale).isActive = true
        return wrap
    }
}

/// 贴底、从下滑入的弹层（Android 是一个 gravity=BOTTOM 的 Dialog + 输入法的进出动画）：
/// 顶上两角 12pt 圆角、背后 45% 黑罩，点罩子收起。
final class BottomSheet: UIViewController {
    private let stack = UIStackView()
    private let dim = UIView()
    private let card = UIView()

    init() {
        super.init(nibName: nil, bundle: nil)
        modalPresentationStyle = .overFullScreen
        stack.axis = .vertical
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    func add(_ v: UIView) { stack.addArrangedSubview(v) }

    func addGap(_ h: CGFloat) {
        let g = UIView()
        g.heightAnchor.constraint(equalToConstant: h).isActive = true
        stack.addArrangedSubview(g)
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        dim.backgroundColor = UIColor(white: 0, alpha: 0.45)
        dim.alpha = 0
        view.addSubview(dim)
        dim.pinEdges(to: view)
        dim.addGestureRecognizer(UITapGestureRecognizer(target: self, action: #selector(tapDim)))
        card.backgroundColor = StoreSheets.sheetBg
        card.layer.cornerRadius = 12
        card.layer.maskedCorners = [.layerMinXMinYCorner, .layerMaxXMinYCorner]
        card.clipsToBounds = true
        card.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(card)
        card.addSubview(stack)
        stack.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            card.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            card.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            card.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            stack.topAnchor.constraint(equalTo: card.topAnchor),
            stack.leadingAnchor.constraint(equalTo: card.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: card.trailingAnchor),
            // 底部横条那一截：「取消」的白底一直铺到屏幕底边
            stack.bottomAnchor.constraint(equalTo: view.safeAreaLayoutGuide.bottomAnchor),
        ])
        let fill = UIView()
        fill.backgroundColor = .white
        fill.translatesAutoresizingMaskIntoConstraints = false
        card.insertSubview(fill, belowSubview: stack)
        NSLayoutConstraint.activate([
            fill.leadingAnchor.constraint(equalTo: card.leadingAnchor),
            fill.trailingAnchor.constraint(equalTo: card.trailingAnchor),
            fill.topAnchor.constraint(equalTo: stack.bottomAnchor),
            fill.bottomAnchor.constraint(equalTo: card.bottomAnchor),
        ])
    }

    func show(from vc: UIViewController) {
        var top = vc
        while let p = top.presentedViewController { top = p }
        UIImpactFeedbackGenerator(style: .medium).impactOccurred()
        top.present(self, animated: false) {
            self.view.layoutIfNeeded()
            self.card.transform = CGAffineTransform(translationX: 0, y: self.card.bounds.height)
            UIView.animate(withDuration: 0.25, delay: 0, options: .curveEaseOut) {
                self.dim.alpha = 1
                self.card.transform = .identity
            }
        }
    }

    func dismiss(_ then: (() -> Void)? = nil) {
        UIView.animate(withDuration: 0.2, delay: 0, options: .curveEaseIn, animations: {
            self.dim.alpha = 0
            self.card.transform = CGAffineTransform(translationX: 0, y: self.card.bounds.height)
        }, completion: { _ in
            self.presentingViewController?.dismiss(animated: false) { then?() }
        })
    }

    @objc private func tapDim() { dismiss() }
}

/// 带内边距的标签（toast 用）
final class PaddedLabel: UILabel {
    var insets = UIEdgeInsets(top: 9, left: 16, bottom: 9, right: 16)
    override func drawText(in rect: CGRect) { super.drawText(in: rect.inset(by: insets)) }
    override var intrinsicContentSize: CGSize {
        let s = super.intrinsicContentSize
        return CGSize(width: s.width + insets.left + insets.right, height: s.height + insets.top + insets.bottom)
    }
    override func textRect(forBounds b: CGRect, limitedToNumberOfLines n: Int) -> CGRect {
        let r = super.textRect(forBounds: b.inset(by: insets), limitedToNumberOfLines: n)
        return r.inset(by: UIEdgeInsets(top: -insets.top, left: -insets.left, bottom: -insets.bottom, right: -insets.right))
    }
}
