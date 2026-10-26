import UIKit

/// 下拉出来的深色小程序面板（微信聊天页往下拉的那一屏）：搜索框、「最近使用」、「我的小程序」。
/// 点图标打开，长按弹操作单（[StoreSheets.appActions]）。数据都是本机记的，[refresh] 重画。
/// 与 Android 示例 StorePanel.kt、Flutter 示例 lib/store/store_panel.dart 同一套布局与场景值。
final class StorePanel: UIView {
    /// 微信的场景值：1089 下拉「最近使用」、1104 下拉「我的小程序」
    static let sceneRecent = 1089
    static let sceneFavorite = 1104

    /// 面板里的滚动区（[PullDrawerView] 要知道它是不是滚到底了）
    let scroll = UIScrollView()
    private let body = UIStackView()
    private let gradient = CAGradientLayer()
    private let store: MiniStore
    private weak var host: UIViewController?
    private var bodyBottom: NSLayoutConstraint?
    private var colTop: NSLayoutConstraint?

    init(host: UIViewController, store: MiniStore, onSearch: @escaping () -> Void) {
        self.host = host
        self.store = store
        super.init(frame: .zero)
        gradient.colors = [UIColor(argb: 0xFF3B3B4F).cgColor, UIColor(argb: 0xFF262633).cgColor, UIColor(argb: 0xFF1C1C25).cgColor]
        layer.insertSublayer(gradient, at: 0)
        let col = UIStackView()
        col.axis = .vertical
        col.translatesAutoresizingMaskIntoConstraints = false
        addSubview(col)
        let title = StoreWidgets.label("小程序", 17, .white, bold: true)
        title.textAlignment = .center
        title.heightAnchor.constraint(equalToConstant: 48).isActive = true
        col.addArrangedSubview(title)
        let search = UIButton(type: .custom)
        search.setTitle("🔍  搜索小程序", for: .normal)
        search.titleLabel?.font = .systemFont(ofSize: 15)
        search.setTitleColor(UIColor(argb: 0x8AFFFFFF), for: .normal)
        search.backgroundColor = UIColor(argb: 0x1AFFFFFF)
        search.layer.cornerRadius = 10
        search.accessibilityLabel = "搜索小程序"
        search.addAction(onSearch)
        let searchWrap = UIView()
        searchWrap.addSubview(search)
        search.pinEdges(to: searchWrap, insets: UIEdgeInsets(top: 0, left: 16, bottom: 0, right: 16))
        search.heightAnchor.constraint(equalToConstant: 38).isActive = true
        col.addArrangedSubview(searchWrap)
        scroll.showsVerticalScrollIndicator = false
        scroll.alwaysBounceVertical = true
        scroll.contentInsetAdjustmentBehavior = .never
        col.addArrangedSubview(scroll)
        body.axis = .vertical
        body.translatesAutoresizingMaskIntoConstraints = false
        scroll.addSubview(body)
        let top = col.topAnchor.constraint(equalTo: topAnchor)
        let bottom = body.bottomAnchor.constraint(equalTo: scroll.contentLayoutGuide.bottomAnchor)
        colTop = top
        bodyBottom = bottom
        NSLayoutConstraint.activate([
            top,
            col.leadingAnchor.constraint(equalTo: leadingAnchor),
            col.trailingAnchor.constraint(equalTo: trailingAnchor),
            col.bottomAnchor.constraint(equalTo: bottomAnchor),
            body.topAnchor.constraint(equalTo: scroll.contentLayoutGuide.topAnchor),
            body.leadingAnchor.constraint(equalTo: scroll.frameLayoutGuide.leadingAnchor),
            body.trailingAnchor.constraint(equalTo: scroll.frameLayoutGuide.trailingAnchor),
            bottom,
        ])
        refresh()
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    override func layoutSubviews() {
        super.layoutSubviews()
        gradient.frame = bounds
    }

    /// 顶上让出状态栏；底下让出首页露出来的那一截（PullDrawerView.peekPt）+ 底部横条
    override func safeAreaInsetsDidChange() {
        super.safeAreaInsetsDidChange()
        colTop?.constant = safeAreaInsets.top
        bodyBottom?.constant = -(PullDrawerView.peekPt + 24 + safeAreaInsets.bottom)
    }

    func refresh() {
        let local = StoreLocal()
        let recent = local.recent()
        let fav = local.favorites()
        body.arrangedSubviews.forEach { $0.removeFromSuperview() }
        body.addArrangedSubview(header("最近使用", recent.isEmpty ? "" : "长按可删除"))
        body.addArrangedSubview(recent.isEmpty ? empty("打开过的小程序会出现在这里") : grid(recent, Self.sceneRecent))
        body.addArrangedSubview(header("我的小程序", fav.isEmpty ? "" : "\(fav.count) 个"))
        body.addArrangedSubview(fav.isEmpty ? empty("长按任意小程序，选「添加到我的小程序」\n或在小程序里点右上角 ••• 收藏")
                                            : grid(fav, Self.sceneFavorite))
        let foot = StoreWidgets.label("上推或点下方返回", 12, UIColor(argb: 0x3DFFFFFF))
        foot.textAlignment = .center
        let footWrap = UIView()
        footWrap.addSubview(foot)
        foot.pinEdges(to: footWrap, insets: UIEdgeInsets(top: 28, left: 0, bottom: 0, right: 0))
        body.addArrangedSubview(footWrap)
    }

    private func header(_ title: String, _ hint: String) -> UIView {
        let row = UIStackView()
        row.alignment = .center
        row.isLayoutMarginsRelativeArrangement = true
        row.layoutMargins = UIEdgeInsets(top: 22, left: 20, bottom: 10, right: 20)
        row.addArrangedSubview(StoreWidgets.label(title, 14, UIColor(argb: 0xB3FFFFFF), bold: true))
        let h = StoreWidgets.label(hint, 12, UIColor(argb: 0x4DFFFFFF))
        h.setContentHuggingPriority(.required, for: .horizontal)
        h.textAlignment = .right
        row.addArrangedSubview(h)
        return row
    }

    private func empty(_ s: String) -> UIView {
        let wrap = UIView()
        let box = UIView()
        box.backgroundColor = UIColor(argb: 0x0DFFFFFF)
        box.layer.cornerRadius = 12
        box.layer.borderWidth = 1
        box.layer.borderColor = UIColor(argb: 0x1FFFFFFF).cgColor
        wrap.addSubview(box)
        box.pinEdges(to: wrap, insets: UIEdgeInsets(top: 0, left: 16, bottom: 0, right: 16))
        let l = StoreWidgets.label(s, 13, UIColor(argb: 0x61FFFFFF), lines: 0)
        l.textAlignment = .center
        let p = NSMutableParagraphStyle()
        p.lineHeightMultiple = 1.3
        p.alignment = .center
        l.attributedText = NSAttributedString(string: s, attributes: [.paragraphStyle: p, .font: l.font!, .foregroundColor: l.textColor!])
        box.addSubview(l)
        l.pinEdges(to: box, insets: UIEdgeInsets(top: 22, left: 16, bottom: 22, right: 16))
        return wrap
    }

    private func grid(_ apps: [MiniStore.App], _ scene: Int) -> UIView {
        AppGrid(apps: apps, padding: 8) { [weak self] app in
            guard let self else { return UIView() }
            return self.cell(app, scene)
        }
    }

    private func cell(_ app: MiniStore.App, _ scene: Int) -> UIView {
        let c = AppCell(store: store, app: app, nameColor: UIColor(argb: 0xB3FFFFFF), fav: false)
        c.pressedColor = UIColor(argb: 0x14FFFFFF)
        c.accessibilityLabel = "打开小程序 \(app.name)，长按更多操作"
        c.onTap = { [weak self] in
            guard let self, let host = self.host else { return }
            StoreWidgets.open(host, store: self.store, app: app, scene: scene)
        }
        c.onLongPress = { [weak self] in
            guard let self, let host = self.host else { return }
            StoreSheets.appActions(host, store: self.store, app: app, scene: scene)
        }
        return c
    }
}

/// 一格小程序：50pt 圆图标 + 12pt 名字（入口页、面板共用）。收藏过的右上角压一颗白底金星
final class AppCell: PressableRow {
    init(store: MiniStore, app: MiniStore.App, nameColor: UIColor, fav: Bool) {
        super.init(frame: .zero)
        let v = UIStackView()
        v.axis = .vertical
        v.alignment = .center
        v.spacing = 8
        v.isUserInteractionEnabled = false
        v.isLayoutMarginsRelativeArrangement = true
        v.layoutMargins = UIEdgeInsets(top: 10, left: 4, bottom: 10, right: 4)
        addSubview(v)
        v.pinEdges(to: self)
        let icon = IconView(store: store, app: app, size: 50)
        icon.widthAnchor.constraint(equalToConstant: 50).isActive = true
        icon.heightAnchor.constraint(equalToConstant: 50).isActive = true
        if fav {
            let star = UILabel()
            star.text = "★"
            star.font = .systemFont(ofSize: 11)
            star.textAlignment = .center
            star.textColor = StoreSheets.gold
            star.backgroundColor = .white
            star.layer.cornerRadius = 8.5
            star.layer.masksToBounds = true
            star.frame = CGRect(x: 50 - 17, y: 0, width: 17, height: 17)
            icon.addSubview(star)
        }
        v.addArrangedSubview(icon)
        let name = StoreWidgets.label(app.name, 12, nameColor)
        name.textAlignment = .center
        v.addArrangedSubview(name)
        name.widthAnchor.constraint(equalTo: v.layoutMarginsGuide.widthAnchor).isActive = true
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
}

/// 四列网格：每格宽 = (父宽 - 左右 padding) / 4，行高按内容
final class AppGrid: UIView {
    private let cells: [UIView]
    private let padding: CGFloat

    init(apps: [MiniStore.App], padding: CGFloat, make: (MiniStore.App) -> UIView) {
        cells = apps.map(make)
        self.padding = padding
        super.init(frame: .zero)
        cells.forEach(addSubview)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    private var cellW: CGFloat { max(1, (bounds.width - padding * 2) / 4) }
    private var rowH: CGFloat {
        cells.first?.systemLayoutSizeFitting(CGSize(width: cellW, height: 0), withHorizontalFittingPriority: .required,
                                             verticalFittingPriority: .fittingSizeLevel).height ?? 0
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        let w = cellW, h = rowH
        for (i, c) in cells.enumerated() {
            c.frame = CGRect(x: padding + CGFloat(i % 4) * w, y: CGFloat(i / 4) * h, width: w, height: h)
        }
        // 行高要等知道宽度才量得准：变了才通知父视图重排（每次都通知会无限 layout）
        if abs(h - measuredRowH) > 0.5 {
            measuredRowH = h
            invalidateIntrinsicContentSize()
        }
    }

    /// 宽度未知时先按 92pt 一行估（10 + 50 + 8 + 14 + 10）
    private var measuredRowH: CGFloat = 92

    override var intrinsicContentSize: CGSize {
        CGSize(width: UIView.noIntrinsicMetric, height: CGFloat((cells.count + 3) / 4) * measuredRowH)
    }
}
