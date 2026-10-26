import AppKit

/// 发现 → 小程序（PC 微信的小程序面板）：顶上「小程序」+ 搜索、内置示例两个按钮；底下一张张白卡片 ——
/// 最近使用（更多 >）、我的常用（我的小程序 >）、小游戏、按分类的全部小程序。点开在独立窗口，右键出操作单。
/// `gamesOnly`：发现 → 游戏，只看「娱乐」分类。
final class HubView: NSView {
    private unowned let main: MainWindowController
    private let gamesOnly: Bool
    private let col = NSStackView()
    private var observers: [NSObjectProtocol] = []

    init(main: MainWindowController, gamesOnly: Bool) {
        self.main = main
        self.gamesOnly = gamesOnly
        super.init(frame: .zero)
        let title = label(gamesOnly ? "游戏" : "小程序", 15, Theme.text, bold: true)
        let search = iconButton("magnifyingglass", "搜索小程序") { [weak main] in main?.showSearch() }
        let demo = iconButton("macwindow.on.rectangle", "打开内置示例") { [weak main] in main?.openDemo() }
        let head = NSStackView(views: [title, NSView(), search, demo])
        head.spacing = 12
        head.edgeInsets = NSEdgeInsets(top: 0, left: 24, bottom: 0, right: 20)
        col.orientation = .vertical
        col.alignment = .leading
        col.spacing = 16
        col.edgeInsets = NSEdgeInsets(top: 8, left: 24, bottom: 24, right: 24)
        let doc = FlippedView()
        doc.addSubview(col)
        col.translatesAutoresizingMaskIntoConstraints = false
        let scroll = NSScrollView()
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        scroll.documentView = doc
        doc.translatesAutoresizingMaskIntoConstraints = false
        wantsLayer = true
        layer?.backgroundColor = Theme.hubBg.cgColor
        for v in [head, scroll] as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = false
            addSubview(v)
        }
        NSLayoutConstraint.activate([
            head.topAnchor.constraint(equalTo: topAnchor, constant: 30),
            head.leadingAnchor.constraint(equalTo: leadingAnchor),
            head.trailingAnchor.constraint(equalTo: trailingAnchor),
            head.heightAnchor.constraint(equalToConstant: 34),
            scroll.topAnchor.constraint(equalTo: head.bottomAnchor, constant: 8),
            scroll.leadingAnchor.constraint(equalTo: leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: trailingAnchor),
            scroll.bottomAnchor.constraint(equalTo: bottomAnchor),
            doc.widthAnchor.constraint(equalTo: scroll.contentView.widthAnchor),
            col.topAnchor.constraint(equalTo: doc.topAnchor),
            col.leadingAnchor.constraint(equalTo: doc.leadingAnchor),
            col.trailingAnchor.constraint(equalTo: doc.trailingAnchor),
            col.bottomAnchor.constraint(equalTo: doc.bottomAnchor),
        ])
        let nc = NotificationCenter.default
        for n in [StoreLocal.changed, MainWindowController.appsChanged] {
            observers.append(nc.addObserver(forName: n, object: nil, queue: .main) { [weak self] _ in self?.rebuild() })
        }
        rebuild()
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    deinit { observers.forEach(NotificationCenter.default.removeObserver) }

    private func iconButton(_ sym: String, _ tip: String, _ action: @escaping () -> Void) -> NSView {
        let b = HoverControl()
        b.layer?.cornerRadius = 5
        b.toolTip = tip
        let img = NSImageView(image: NSImage(systemSymbolName: sym, accessibilityDescription: tip) ?? NSImage())
        img.symbolConfiguration = .init(pointSize: 15, weight: .regular)
        img.contentTintColor = NSColor(argb: 0xFF4C4C4C)
        b.addSubview(img)
        pin(img, to: b, NSEdgeInsets(top: 5, left: 5, bottom: 5, right: 5))
        fixed(b, w: 28, h: 28)
        b.onClick = action
        return b
    }

    private func add(_ card: NSView) {
        col.addArrangedSubview(card)
        card.widthAnchor.constraint(equalTo: col.widthAnchor, constant: -48).isActive = true
    }

    private func tile(_ a: MiniStore.App, scene: Int, fav: Set<String> = []) -> AppTile {
        let t = AppTile(store: main.store, app: a, fav: fav.contains(a.id))
        t.onClick = { [weak main] in main?.open(a, scene: scene) }
        t.menuProvider = { [weak main] in main?.menu(for: a, scene: scene) }
        fixed(t, w: 86)
        return t
    }

    private func rebuild() {
        col.arrangedSubviews.forEach { $0.removeFromSuperview() }
        let local = StoreLocal()
        let favIds = Set(local.favorites().map(\.id))
        let apps = gamesOnly ? main.apps.filter { $0.category == "娱乐" } : main.apps
        if let off = main.offlineReason {
            let warn = label("⚠ 服务器连不上，显示上次的列表（\(off)）", 12, NSColor(argb: 0xFFE64340))
            col.addArrangedSubview(warn)
        }
        if !gamesOnly {
            let recent = local.recent()
            let c1 = Card(title: "最近使用", more: recent.isEmpty ? nil : "更多") { [weak self] in self?.showAll("最近使用", recent, 1089) }
            c1.body.addArrangedSubview(recent.isEmpty ? label("打开过的小程序会出现在这里", 12, Theme.sub)
                                                      : tileRow(recent.prefix(6).map { tile($0, scene: 1089) }))
            add(c1)
            let fav = local.favorites()
            let c2 = Card(title: "我的常用", more: "我的小程序") { [weak main] in main?.select(.favorites) }
            c2.body.addArrangedSubview(fav.isEmpty ? label("右键任意小程序，选「添加到我的小程序」", 12, Theme.sub)
                                                   : tileRow(fav.prefix(6).map { tile($0, scene: 1104) }))
            add(c2)
        }
        let games = main.apps.filter { $0.category == "娱乐" }
        if !games.isEmpty, !gamesOnly {
            let c3 = Card(title: "小游戏", more: "全部小游戏") { [weak main] in main?.showHub(gamesOnly: true) }
            c3.body.addArrangedSubview(tileRow(games.prefix(6).map { gameTile($0) }))
            add(c3)
        }
        // 全部：按服务器的固定分类顺序分组
        let order = ["美食", "购物", "出行", "生活服务", "医疗健康", "金融", "工具", "教育", "娱乐", "资讯", "政务", "其他"]
        var groups: [String: [MiniStore.App]] = [:]
        for a in apps { groups[a.category.isEmpty ? "其他" : a.category, default: []].append(a) }
        for c in order {
            guard let list = groups[c], !list.isEmpty else { continue }
            let card = Card(title: "\(c) · \(list.count)")
            card.body.addArrangedSubview(TileGrid(tiles: list.map { tile($0, scene: 1001, fav: favIds) }))
            add(card)
            card.body.arrangedSubviews.last?.widthAnchor.constraint(equalTo: card.body.widthAnchor).isActive = true
        }
        if apps.isEmpty, main.offlineReason == nil {
            col.addArrangedSubview(label("正在连接 \(main.store.baseUrl) …", 12, Theme.sub))
        }
        if !gamesOnly {
            let dev = Card(title: "内置示例")
            let t = HoverControl()
            t.layer?.cornerRadius = 6
            let row = NSStackView(views: [IconView(id: "demo", name: "示", size: 40),
                                          label("mini_render 能力演示（App 里自带，不用联网）", 13)])
            row.spacing = 12
            row.edgeInsets = NSEdgeInsets(top: 6, left: 4, bottom: 6, right: 4)
            t.addSubview(row)
            pin(row, to: t)
            t.onClick = { [weak main] in main?.openDemo() }
            dev.body.addArrangedSubview(t)
            add(dev)
        }
    }

    /// 小游戏的大方块（PC 微信的小游戏卡片是封面图；商店没有封面，用底色 + 首字）
    private func gameTile(_ a: MiniStore.App) -> NSView {
        let t = HoverControl()
        t.layer?.cornerRadius = 6
        t.hoverColor = NSColor(argb: 0x0A000000)
        let cover = IconView(store: main.store, app: a, size: 112, corner: 6)
        let name = label(a.name, 13)
        let sub = label(a.description, 11, NSColor(argb: 0xFFFA9D3B))
        sub.widthAnchor.constraint(lessThanOrEqualToConstant: 112).isActive = true
        let col = NSStackView(views: [cover, name, sub])
        col.orientation = .vertical
        col.alignment = .leading
        col.spacing = 4
        t.addSubview(col)
        pin(col, to: t)
        t.onClick = { [weak main] in main?.open(a, scene: 1001) }
        t.menuProvider = { [weak main] in main?.menu(for: a) }
        return t
    }

    private func showAll(_ title: String, _ apps: [MiniStore.App], _ scene: Int) {
        col.arrangedSubviews.forEach { $0.removeFromSuperview() }
        let back = HoverControl()
        let bl = label("‹ 返回", 13, NSColor(argb: 0xFF576B95))
        back.addSubview(bl)
        pin(bl, to: back, NSEdgeInsets(top: 2, left: 2, bottom: 2, right: 2))
        back.onClick = { [weak self] in self?.rebuild() }
        col.addArrangedSubview(back)
        let card = Card(title: title)
        card.body.addArrangedSubview(TileGrid(tiles: apps.map { tile($0, scene: scene) }))
        add(card)
        card.body.arrangedSubviews.last?.widthAnchor.constraint(equalTo: card.body.widthAnchor).isActive = true
    }
}

/// 搜一搜 · 小程序：顶上搜索框，回车问服务器（`/api/apps?q=`，与手机端示例同一个接口），结果一行一个
final class SearchView: NSView, NSSearchFieldDelegate {
    private unowned let main: MainWindowController
    private let field = NSSearchField()
    private let list = NSStackView()
    private let status = label("", 12, Theme.sub)
    private var seq = 0

    init(main: MainWindowController, query: String) {
        self.main = main
        super.init(frame: .zero)
        wantsLayer = true
        layer?.backgroundColor = NSColor.white.cgColor
        field.placeholderString = "搜索小程序"
        field.sendsSearchStringImmediately = false
        field.target = self
        field.action = #selector(go)
        field.stringValue = query
        list.orientation = .vertical
        list.alignment = .leading
        list.spacing = 0
        let doc = FlippedView()
        doc.addSubview(list)
        list.translatesAutoresizingMaskIntoConstraints = false
        let scroll = NSScrollView()
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.documentView = doc
        doc.translatesAutoresizingMaskIntoConstraints = false
        for v in [field, status, scroll] as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = false
            addSubview(v)
        }
        NSLayoutConstraint.activate([
            field.topAnchor.constraint(equalTo: topAnchor, constant: 34),
            field.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 24),
            field.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -24),
            status.topAnchor.constraint(equalTo: field.bottomAnchor, constant: 12),
            status.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 24),
            scroll.topAnchor.constraint(equalTo: status.bottomAnchor, constant: 8),
            scroll.leadingAnchor.constraint(equalTo: leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: trailingAnchor),
            scroll.bottomAnchor.constraint(equalTo: bottomAnchor),
            doc.widthAnchor.constraint(equalTo: scroll.contentView.widthAnchor),
            list.topAnchor.constraint(equalTo: doc.topAnchor),
            list.leadingAnchor.constraint(equalTo: doc.leadingAnchor),
            list.trailingAnchor.constraint(equalTo: doc.trailingAnchor),
            list.bottomAnchor.constraint(equalTo: doc.bottomAnchor),
        ])
        if !query.isEmpty { go() } else { status.stringValue = "输入名字、分类或标签，回车搜索" }
        DispatchQueue.main.async { self.window?.makeFirstResponder(self.field) }
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    @objc private func go() {
        let q = field.stringValue.trimmingCharacters(in: .whitespaces)
        guard !q.isEmpty else { return }
        StoreLocal().addHistory(q)
        seq += 1
        let my = seq
        status.stringValue = "搜索中…"
        list.arrangedSubviews.forEach { $0.removeFromSuperview() }
        let store = main.store
        DispatchQueue.global().async {
            let r = Result { try store.search(q) }
            DispatchQueue.main.async { [weak self] in
                guard let self, my == self.seq else { return }
                switch r {
                case let .success((apps, total)):
                    self.status.stringValue = total == 0 ? "没有找到「\(q)」相关的小程序，换个词试试" : "共 \(total) 个小程序"
                    apps.forEach { self.row($0) }
                case let .failure(e):
                    self.status.stringValue = "搜索失败：\(e.localizedDescription)"
                }
            }
        }
    }

    private func row(_ a: MiniStore.App) {
        let r = HoverControl()
        let texts = NSStackView(views: [label(a.name, 15), label([a.category, a.developer].filter { !$0.isEmpty }.joined(separator: " · "), 12, Theme.sub),
                                        label(a.description, 12, NSColor(argb: 0x99000000))])
        texts.orientation = .vertical
        texts.alignment = .leading
        texts.spacing = 3
        let h = NSStackView(views: [IconView(store: main.store, app: a, size: 44), texts])
        h.spacing = 12
        h.edgeInsets = NSEdgeInsets(top: 10, left: 24, bottom: 10, right: 24)
        r.addSubview(h)
        pin(h, to: r)
        r.onClick = { [weak main] in main?.open(a, scene: 1005) }
        r.menuProvider = { [weak main] in main?.menu(for: a, scene: 1005) }
        list.addArrangedSubview(r)
        r.widthAnchor.constraint(equalTo: list.widthAnchor).isActive = true
    }
}
