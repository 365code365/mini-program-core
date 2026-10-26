import AppKit

/// 主窗口：照 macOS 版微信的三栏 —— 左侧栏（头像、聊天、通讯录、收藏、发现，底下手机与菜单）、
/// 中间列表（会话 / 通讯录 / 收藏 / 发现）、右边内容区（聊天、小程序面板、空白时一个微信式的灰气泡）。
/// 红绿灯压在左侧栏顶上（透明标题栏 + fullSizeContentView）。
final class MainWindowController: NSWindowController {
    enum Tab: Int, CaseIterable { case chats, contacts, favorites, discover }

    let store = MiniStore()
    private(set) var apps: [MiniStore.App] = []
    private let rail = NSStackView()
    private var railButtons: [Tab: RailButton] = [:]
    private let middle = NSView()
    private let content = NSView()
    private var tab = Tab.chats
    private var badge: NSTextField?

    init() {
        let w = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1024, height: 720),
                         styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
                         backing: .buffered, defer: false)
        w.titlebarAppearsTransparent = true
        w.titleVisibility = .hidden
        w.title = "灵犀"
        w.minSize = NSSize(width: 820, height: 560)
        w.setFrameAutosaveName("main")
        super.init(window: w)
        build()
        if w.frame.origin == .zero { w.center() }
        NotificationCenter.default.addObserver(self, selector: #selector(localChanged), name: StoreLocal.changed, object: nil)
        NotificationCenter.default.addObserver(self, selector: #selector(appsArrived), name: Self.appsChanged, object: nil)
        refreshApps()
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    private func build() {
        let root = NSView()
        window!.contentView = root
        // 左侧栏
        let railBox = NSView()
        railBox.wantsLayer = true
        railBox.layer?.backgroundColor = Theme.rail.cgColor
        rail.orientation = .vertical
        rail.spacing = 14
        rail.alignment = .centerX
        rail.edgeInsets = NSEdgeInsets(top: 44, left: 0, bottom: 14, right: 0)
        let avatar = IconView(id: "me", name: "灵", size: 36, corner: 6)
        avatar.toolTip = "我（演示账号）"
        rail.addArrangedSubview(avatar)
        rail.setCustomSpacing(26, after: avatar)
        let items: [(Tab, String, String)] = [
            (.chats, "message", "聊天"), (.contacts, "person.2", "通讯录"),
            (.favorites, "cube", "收藏"), (.discover, "safari", "发现"),
        ]
        for (t, sym, tip) in items {
            let b = RailButton(symbol: sym, tip: tip) { [weak self] in self?.select(t) }
            railButtons[t] = b
            rail.addArrangedSubview(b)
        }
        // 聊天未读角标（演示数据）
        if let chat = railButtons[.chats] {
            let b = label("3", 10, .white, bold: true)
            b.alignment = .center
            b.wantsLayer = true
            b.layer?.backgroundColor = NSColor(argb: 0xFFFA5151).cgColor
            b.layer?.cornerRadius = 8
            chat.addSubview(b)
            b.frame = NSRect(x: 24, y: 0, width: 16, height: 16)
            badge = b
        }
        let spacer = NSView()
        spacer.setContentHuggingPriority(.init(1), for: .vertical)
        rail.addArrangedSubview(spacer)
        rail.addArrangedSubview(RailButton(symbol: "iphone", tip: "手机（演示）") {})
        rail.addArrangedSubview(RailButton(symbol: "line.3.horizontal", tip: "设置") { [weak self] in self?.showSettingsMenu() })
        railBox.addSubview(rail)
        pin(rail, to: railBox)

        middle.wantsLayer = true
        middle.layer?.backgroundColor = Theme.list.cgColor
        content.wantsLayer = true
        content.layer?.backgroundColor = Theme.content.cgColor
        let sep = NSView()
        sep.wantsLayer = true
        sep.layer?.backgroundColor = Theme.line.cgColor
        for v in [railBox, middle, sep, content] {
            v.translatesAutoresizingMaskIntoConstraints = false
            root.addSubview(v)
        }
        NSLayoutConstraint.activate([
            railBox.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            railBox.topAnchor.constraint(equalTo: root.topAnchor),
            railBox.bottomAnchor.constraint(equalTo: root.bottomAnchor),
            railBox.widthAnchor.constraint(equalToConstant: 64),
            middle.leadingAnchor.constraint(equalTo: railBox.trailingAnchor),
            middle.topAnchor.constraint(equalTo: root.topAnchor),
            middle.bottomAnchor.constraint(equalTo: root.bottomAnchor),
            middle.widthAnchor.constraint(equalToConstant: 280),
            sep.leadingAnchor.constraint(equalTo: middle.trailingAnchor),
            sep.topAnchor.constraint(equalTo: root.topAnchor),
            sep.bottomAnchor.constraint(equalTo: root.bottomAnchor),
            sep.widthAnchor.constraint(equalToConstant: 1),
            content.leadingAnchor.constraint(equalTo: sep.trailingAnchor),
            content.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            content.topAnchor.constraint(equalTo: root.topAnchor),
            content.bottomAnchor.constraint(equalTo: root.bottomAnchor),
        ])
        select(.chats)
    }

    // MARK: - 切换

    func select(_ t: Tab) {
        tab = t
        railButtons.forEach { $0.value.isSelected = $0.key == t }
        let v: NSView
        switch t {
        case .chats: v = ChatListPane(main: self)
        case .contacts: v = ContactsPane(main: self)
        case .favorites: v = FavoritesPane(main: self)
        case .discover: v = DiscoverPane(main: self)
        }
        setMiddle(v)
        showEmpty()
    }

    private func setMiddle(_ v: NSView) {
        middle.subviews.forEach { $0.removeFromSuperview() }
        middle.addSubview(v)
        pin(v, to: middle)
    }

    func showContent(_ v: NSView) {
        content.subviews.forEach { $0.removeFromSuperview() }
        content.addSubview(v)
        pin(v, to: content)
    }

    /// 右边没选中东西时：中间一个灰色的双气泡（微信的空白页）
    func showEmpty() {
        let v = NSView()
        let img = NSImageView(image: NSImage(systemSymbolName: "bubble.left.and.bubble.right.fill", accessibilityDescription: nil) ?? NSImage())
        img.symbolConfiguration = .init(pointSize: 72, weight: .regular)
        img.contentTintColor = NSColor(argb: 0xFFDEDEDE)
        v.addSubview(img)
        img.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([img.centerXAnchor.constraint(equalTo: v.centerXAnchor), img.centerYAnchor.constraint(equalTo: v.centerYAnchor)])
        showContent(v)
    }

    /// 发现 → 小程序（或游戏：只看娱乐分类）
    func showHub(gamesOnly: Bool = false) {
        showContent(HubView(main: self, gamesOnly: gamesOnly))
    }

    func showSearch(_ q: String = "") {
        showContent(SearchView(main: self, query: q))
    }

    /// 打开一个商店小程序（PC 微信：手机大小的独立窗口）
    func open(_ app: MiniStore.App, scene: Int = 1001) {
        StoreLocal().touch(app)
        MiniWindowController.open(link: StoreLink(key: app.key, scene: scene), hint: app, store: store)
    }

    func openDemo() {
        MiniWindowController.open(link: nil, store: store)
    }

    // MARK: - 数据

    func refreshApps(_ done: (() -> Void)? = nil) {
        DispatchQueue.global(qos: .userInitiated).async { [store] in
            let l = store.list()
            DispatchQueue.main.async { [weak self] in
                guard let self else { return }
                NSLog("[商店] 列表 %d 个 offline=%@", l.apps.count, l.offline ? "true" : "false")
                self.apps = l.apps
                self.offlineReason = l.offline ? l.error : nil
                NotificationCenter.default.post(name: Self.appsChanged, object: nil)
                done?()
            }
        }
    }

    static let appsChanged = Notification.Name("MainWindow.appsChanged")
    private(set) var offlineReason: String?

    @objc private func localChanged() {
        if tab == .favorites { setMiddle(FavoritesPane(main: self)) }
    }

    /// 商店列表回来了：会话里的小程序卡片要用真的小程序
    @objc private func appsArrived() {
        if tab == .chats { setMiddle(ChatListPane(main: self)) }
    }

    /// 长按 / 右键一个小程序：收藏 / 移出、从最近使用删除、打开（与手机端示例的操作单同一组）
    func menu(for app: MiniStore.App, scene: Int = 1001) -> NSMenu {
        let local = StoreLocal()
        let m = NSMenu()
        m.addItem(MenuAction("打开") { [weak self] in self?.open(app, scene: scene) })
        m.addItem(MenuAction(local.isFavorite(app.id) ? "从「我的小程序」移除" : "添加到「我的小程序」") { local.toggleFavorite(app) })
        if local.recent().contains(where: { $0.id == app.id }) {
            m.addItem(MenuAction("从「最近使用」删除") { local.removeRecent(app.id) })
        }
        return m
    }

    private func showSettingsMenu() {
        let m = NSMenu()
        m.autoenablesItems = false
        let diag = UserDefaults.standard.bool(forKey: "diag")
        m.addItem(MenuAction(diag ? "✓ 诊断日志（帧耗时归因，下次打开小程序生效）" : "诊断日志（帧耗时归因，下次打开小程序生效）") {
            UserDefaults.standard.set(!diag, forKey: "diag")
        })
        m.addItem(MenuAction("上传诊断日志") { [weak self] in
            guard let self, let w = self.window else { return }
            DiagUpload.ask(w, baseUrl: self.store.baseUrl)
        })
        m.addItem(.separator())
        m.addItem(withTitle: "商店：\(store.baseUrl)", action: nil, keyEquivalent: "").isEnabled = false
        guard let v = window?.contentView else { return }
        m.popUp(positioning: nil, at: NSPoint(x: 56, y: 40), in: v)
    }
}

/// 左侧栏的一个图标按钮：选中时图标变绿（发现是实心的绿圆指南针，与微信一样）
final class RailButton: HoverControl {
    private let img = NSImageView()
    private let symbol: String

    init(symbol: String, tip: String, onClick: @escaping () -> Void) {
        self.symbol = symbol
        super.init(frame: .zero)
        self.onClick = onClick
        toolTip = tip
        layer?.cornerRadius = 8
        hoverColor = NSColor(argb: 0x0F000000)
        selectedColor = NSColor(argb: 0x14000000)
        img.symbolConfiguration = .init(pointSize: 19, weight: .regular)
        addSubview(img)
        img.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([img.centerXAnchor.constraint(equalTo: centerXAnchor), img.centerYAnchor.constraint(equalTo: centerYAnchor)])
        fixed(self, w: 40, h: 40)
        update()
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    override var isSelected: Bool { didSet { update() } }

    private func update() {
        let name = isSelected ? (NSImage(systemSymbolName: symbol + ".fill", accessibilityDescription: nil) != nil ? symbol + ".fill" : symbol) : symbol
        img.image = NSImage(systemSymbolName: name, accessibilityDescription: toolTip)
        img.contentTintColor = isSelected ? Theme.green : NSColor(argb: 0xFF5F5F5F)
    }
}
