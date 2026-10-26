import UIKit

/// 入口页（桌面图标进来的就是它）。灰底上一张张白卡片，与 Android 示例 StoreActivity.kt 一一对应：
/// - 顶上一条搜索框 → [SearchViewController]（历史、热搜、联想、按分类筛）；
/// - 「最近使用」：本机打开过的，最新在前（[StoreLocal]，离线也在）；
/// - 分类一排，点了进搜索页看这个分类；
/// - 「全部小程序」：服务器上的（tools/mini-store），四列圆图标，收藏过的右上角带星；
/// - 最底下是开发用的：内置示例、上传诊断日志、诊断开关。
/// 滚到顶再往下拉（或点标题栏右边的 ⊞）：微信式的下拉面板（[StorePanel]，「最近使用」「我的小程序」）。
/// 任何一个图标长按：操作单（收藏 / 移出、从最近使用删除，[StoreSheets.appActions]）。
/// 点一个小程序 → [MiniViewController] 从下往上推出来，由它盖 loading 层、下载解包、打开。
final class StoreViewController: UIViewController {
    private static let pageBg = UIColor(argb: 0xFFF2F3F5)
    private static let categoryOrder = ["美食", "购物", "出行", "生活服务", "医疗健康", "金融", "工具", "教育", "娱乐", "资讯", "政务", "其他"]

    private let store = MiniStore()
    private let scroll = UIScrollView()
    private let col = UIStackView()
    private let status = StoreWidgets.label("", 12, UIColor(argb: 0x73000000), lines: 0)
    private let recentRow = UIStackView()
    private var recentBox: UIView!
    private let catBox = FlowLayout()
    private var gridSlot = UIView()
    private var drawer: PullDrawerView!
    private var panel: StorePanel!
    private var apps: [MiniStore.App] = []
    private var drawerOpen = false

    override func viewDidLoad() {
        super.viewDidLoad()
        // 白底：状态栏那一截跟标题栏连成一片；内容区的灰底在 scroll 上
        let page = UIView()
        page.backgroundColor = .white
        let bar = titleBar()
        page.addSubview(bar)
        scroll.backgroundColor = Self.pageBg
        scroll.alwaysBounceVertical = true
        page.addSubview(scroll)
        bar.translatesAutoresizingMaskIntoConstraints = false
        scroll.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            bar.topAnchor.constraint(equalTo: page.safeAreaLayoutGuide.topAnchor),
            bar.leadingAnchor.constraint(equalTo: page.leadingAnchor),
            bar.trailingAnchor.constraint(equalTo: page.trailingAnchor),
            bar.heightAnchor.constraint(equalToConstant: 56),
            scroll.topAnchor.constraint(equalTo: bar.bottomAnchor),
            scroll.leadingAnchor.constraint(equalTo: page.leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: page.trailingAnchor),
            scroll.bottomAnchor.constraint(equalTo: page.bottomAnchor),
        ])
        col.axis = .vertical
        col.spacing = 12
        scroll.addSubview(col)
        col.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            col.topAnchor.constraint(equalTo: scroll.contentLayoutGuide.topAnchor, constant: 12),
            col.leadingAnchor.constraint(equalTo: scroll.frameLayoutGuide.leadingAnchor, constant: 12),
            col.trailingAnchor.constraint(equalTo: scroll.frameLayoutGuide.trailingAnchor, constant: -12),
            col.bottomAnchor.constraint(equalTo: scroll.contentLayoutGuide.bottomAnchor, constant: -24),
        ])
        col.addArrangedSubview(searchBar())

        recentRow.isLayoutMarginsRelativeArrangement = true
        recentRow.layoutMargins = UIEdgeInsets(top: 0, left: 8, bottom: 0, right: 8)
        let recentScroll = UIScrollView()
        recentScroll.showsHorizontalScrollIndicator = false
        recentScroll.addSubview(recentRow)
        recentRow.pinEdges(to: recentScroll)
        recentRow.heightAnchor.constraint(equalTo: recentScroll.heightAnchor).isActive = true
        recentScroll.heightAnchor.constraint(equalToConstant: 92).isActive = true
        recentBox = card("最近使用", hint: "长按可删除", content: [recentScroll])
        recentBox.isHidden = true
        col.addArrangedSubview(recentBox)

        catBox.insets = UIEdgeInsets(top: 4, left: 16, bottom: 0, right: 16)
        col.addArrangedSubview(card("分类", content: [catBox]))

        let statusWrap = UIView()
        statusWrap.addSubview(status)
        status.pinEdges(to: statusWrap, insets: UIEdgeInsets(top: 0, left: 16, bottom: 0, right: 16))
        col.addArrangedSubview(card("全部小程序", action: "刷新", onAction: { [weak self] in self?.refresh() }, content: [statusWrap, gridSlot]))
        col.addArrangedSubview(card("开发调试", content: [
            row("内置示例", "App 里的演示小程序（assets/demo）") { [weak self] in
                guard let self else { return }
                StoreWidgets.present(MiniViewController(), from: self)
            },
            // 真机出问题、手边没线：把本进程日志传到 mini-store，开发机上 tools/mini-store/diag.sh 拉回来
            row("上传诊断日志", "把运行日志、机型与版本传到服务器，交给开发分析") { [weak self] in
                guard let self else { return }
                DiagUpload.ask(self, baseUrl: self.store.baseUrl)
            },
            diagRow(),
        ]))

        panel = StorePanel(host: self, store: store) { [weak self] in self?.openSearch() }
        drawer = PullDrawerView(panel: panel, content: page, contentScroll: scroll, panelScroll: panel.scroll)
        drawer.onStateChanged = { [weak self] open in
            self?.drawerOpen = open
            self?.setNeedsStatusBarAppearanceUpdate()
        }
        drawer.frame = view.bounds
        drawer.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        view.addSubview(drawer)
        NotificationCenter.default.addObserver(self, selector: #selector(localChanged), name: StoreLocal.changed, object: nil)
        refresh()
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        navigationController?.setNavigationBarHidden(true, animated: animated)
        showRecent()
        panel.refresh()
    }

    /// 面板是深色的：展开时状态栏图标换白
    override var preferredStatusBarStyle: UIStatusBarStyle {
        if drawerOpen { return .lightContent }
        if #available(iOS 13.0, *) { return .darkContent }
        return .default
    }

    @objc private func localChanged() {
        showRecent()
        showGrid()
        panel.refresh()
    }

    private func openSearch(category: String = "") {
        navigationController?.pushViewController(SearchViewController(store: store, category: category), animated: true)
    }

    // MARK: - 零件

    private func titleBar() -> UIView {
        let bar = UIStackView()
        bar.alignment = .center
        bar.backgroundColor = .white
        bar.isLayoutMarginsRelativeArrangement = true
        bar.layoutMargins = UIEdgeInsets(top: 0, left: 16, bottom: 0, right: 4)
        let t = UIStackView()
        t.axis = .vertical
        t.addArrangedSubview(StoreWidgets.label("灵犀原生", 20, .black, bold: true))
        t.addArrangedSubview(StoreWidgets.label("小程序引擎演示 · 下拉打开小程序面板", 11, UIColor(argb: 0x61000000)))
        bar.addArrangedSubview(t)
        let b = UIButton(type: .custom)
        b.setTitle("⊞", for: .normal)
        b.titleLabel?.font = .systemFont(ofSize: 22)
        b.setTitleColor(UIColor(argb: 0xDE000000), for: .normal)
        b.accessibilityLabel = "小程序面板"
        b.addAction { [weak self] in self?.drawer.open() }
        b.widthAnchor.constraint(equalToConstant: 48).isActive = true
        b.heightAnchor.constraint(equalToConstant: 48).isActive = true
        bar.addArrangedSubview(b)
        return bar
    }

    /// 白底圆角卡片：标题行（可带右边的提示 / 按钮）+ 往里加的内容
    private func card(_ title: String, hint: String = "", action: String? = nil, onAction: (() -> Void)? = nil, content: [UIView]) -> UIView {
        let c = UIStackView()
        c.axis = .vertical
        c.backgroundColor = .white
        c.layer.cornerRadius = 14
        c.clipsToBounds = true
        c.isLayoutMarginsRelativeArrangement = true
        c.layoutMargins = UIEdgeInsets(top: 0, left: 0, bottom: 12, right: 0)
        let h = StoreWidgets.header(title, action: action ?? (hint.isEmpty ? nil : hint), onAction: onAction)
        (h as? UIStackView)?.layoutMargins = UIEdgeInsets(top: 14, left: 16, bottom: 6, right: 8)
        c.addArrangedSubview(h)
        content.forEach(c.addArrangedSubview)
        // iOS 13 的 UIStackView 不画背景色：包一层
        let wrap = UIView()
        wrap.backgroundColor = .white
        wrap.layer.cornerRadius = 14
        wrap.clipsToBounds = true
        wrap.addSubview(c)
        c.pinEdges(to: wrap)
        return wrap
    }

    /// 白底圆角的假搜索框：点了进搜索页（微信也是这样，入口页上不直接输入）
    private func searchBar() -> UIView {
        let b = UIButton(type: .custom)
        b.setTitle("🔍  搜索小程序", for: .normal)
        b.titleLabel?.font = .systemFont(ofSize: 15)
        b.setTitleColor(UIColor(argb: 0x61000000), for: .normal)
        b.backgroundColor = .white
        b.layer.cornerRadius = 10
        b.accessibilityLabel = "搜索小程序"
        b.heightAnchor.constraint(equalToConstant: 40).isActive = true
        b.addAction { [weak self] in self?.openSearch() }
        return b
    }

    private func row(_ title: String, _ sub: String, onTap: @escaping () -> Void) -> PressableRow {
        let r = PressableRow()
        r.accessibilityLabel = title
        let v = UIStackView()
        v.axis = .vertical
        v.isUserInteractionEnabled = false
        v.isLayoutMarginsRelativeArrangement = true
        v.layoutMargins = UIEdgeInsets(top: 12, left: 16, bottom: 12, right: 16)
        v.addArrangedSubview(StoreWidgets.label(title, 16, .black))
        v.addArrangedSubview(StoreWidgets.label(sub, 12, UIColor(argb: 0x73000000), lines: 0))
        r.addSubview(v)
        v.pinEdges(to: r)
        r.onTap = onTap
        return r
    }

    /// 测试用的诊断开关：打开后小程序里每一帧慢在哪、为什么整帧都打到日志（Xcode 控制台 / ios-debug.sh 的
    /// native.log 里看 `〔慢：…〕`）。下一次打开小程序生效
    private func diagRow() -> UIView {
        var sub: UILabel?
        func label() {
            let on = UserDefaults.standard.bool(forKey: MiniViewController.prefDiag)
            sub?.text = on ? "已打开：Xcode 控制台看〔慢：…〕、整帧原因、[生命周期]" : "关着（点一下打开；下一次打开小程序生效）"
        }
        let r = row("诊断日志", "") {
            let d = UserDefaults.standard
            d.set(!d.bool(forKey: MiniViewController.prefDiag), forKey: MiniViewController.prefDiag)
            label()
        }
        sub = (r.subviews.first as? UIStackView)?.arrangedSubviews.last as? UILabel
        label()
        return r
    }

    // MARK: - 数据

    private func refresh() {
        status.text = "正在连接 \(store.baseUrl) …"
        DispatchQueue.global(qos: .userInitiated).async { [store] in
            let l = store.list()
            DispatchQueue.main.async { [weak self] in
                guard let self else { return }
                self.status.text = l.offline ? "⚠ 服务器连不上，显示上次的列表（\(l.error ?? "")）"
                    : l.apps.isEmpty ? "服务器上还没有小程序" : "来自 \(store.baseUrl) · \(l.apps.count) 个"
                NSLog("[商店] 列表 %d 个 offline=%@", l.apps.count, l.offline ? "true" : "false")
                self.apps = l.apps
                self.showCategories()
                self.showGrid()
            }
        }
    }

    private func showRecent() {
        guard recentBox != nil else { return }
        let recent = StoreLocal().recent()
        recentBox.isHidden = recent.isEmpty
        recentRow.arrangedSubviews.forEach { $0.removeFromSuperview() }
        let w = (view.bounds.width - 24 - 16) / 4
        for a in recent {
            let c = cell(a)
            c.widthAnchor.constraint(equalToConstant: w).isActive = true
            recentRow.addArrangedSubview(c)
        }
    }

    private func showGrid() {
        let fav = Set(StoreLocal().favorites().map(\.id))
        let grid = AppGrid(apps: apps, padding: 8) { [unowned self] in self.cell($0, fav: fav.contains($0.id)) }
        guard let parent = gridSlot.superview as? UIStackView, let i = parent.arrangedSubviews.firstIndex(of: gridSlot) else { return }
        gridSlot.removeFromSuperview()
        parent.insertArrangedSubview(grid, at: i)
        gridSlot = grid
    }

    private func cell(_ app: MiniStore.App, fav: Bool = false) -> UIView {
        let c = AppCell(store: store, app: app, nameColor: .black, fav: fav)
        c.accessibilityLabel = "打开小程序 \(app.name)\(fav ? "（已收藏）" : "")，长按更多操作"
        c.onTap = { [weak self] in
            guard let self else { return }
            StoreWidgets.open(self, store: self.store, app: app)
        }
        c.onLongPress = { [weak self] in
            guard let self else { return }
            StoreSheets.appActions(self, store: self.store, app: app)
        }
        return c
    }

    /// 分类按列表现算（离线时用缓存的列表也能出来），顺序跟服务器的固定分类表走
    private func showCategories() {
        var counts: [String: Int] = [:]
        for a in apps { counts[a.category.isEmpty ? "其他" : a.category, default: 0] += 1 }
        catBox.setItems(Self.categoryOrder.filter { (counts[$0] ?? 0) > 0 }.map { c in
            StoreWidgets.chip("\(c) \(counts[c]!)") { [weak self] in self?.openSearch(category: c) }
        })
    }
}
