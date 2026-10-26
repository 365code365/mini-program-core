import AppKit

/// 中间列表的几种：会话、通讯录、收藏、发现。都是一个可滚动的竖排列表，顶上一条标题 / 搜索框。
class ListPane: NSView {
    unowned let main: MainWindowController
    let stack = NSStackView()
    private var rows: [HoverControl] = []

    init(main: MainWindowController, title: String? = nil, search: Bool = false) {
        self.main = main
        super.init(frame: .zero)
        let head = NSStackView()
        head.orientation = .horizontal
        head.edgeInsets = NSEdgeInsets(top: 0, left: 14, bottom: 0, right: 10)
        if search {
            let f = NSSearchField()
            f.placeholderString = "搜索"
            f.target = self
            f.action = #selector(searched(_:))
            f.sendsSearchStringImmediately = false
            head.addArrangedSubview(f)
            let plus = HoverControl()
            plus.layer?.cornerRadius = 4
            let img = NSImageView(image: NSImage(systemSymbolName: "plus", accessibilityDescription: "发起") ?? NSImage())
            img.contentTintColor = NSColor(argb: 0xFF5F5F5F)
            plus.addSubview(img)
            pin(img, to: plus, NSEdgeInsets(top: 4, left: 4, bottom: 4, right: 4))
            fixed(plus, w: 26, h: 26)
            head.addArrangedSubview(plus)
        } else if let title {
            head.addArrangedSubview(label(title, 15, Theme.text, bold: true))
            head.addArrangedSubview(NSView())
        }
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 0
        let doc = FlippedView()
        doc.addSubview(stack)
        stack.translatesAutoresizingMaskIntoConstraints = false
        let scroll = NSScrollView()
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        scroll.documentView = doc
        doc.translatesAutoresizingMaskIntoConstraints = false
        for v in [head, scroll] as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = false
            addSubview(v)
        }
        NSLayoutConstraint.activate([
            head.topAnchor.constraint(equalTo: topAnchor, constant: 34),
            head.leadingAnchor.constraint(equalTo: leadingAnchor),
            head.trailingAnchor.constraint(equalTo: trailingAnchor),
            head.heightAnchor.constraint(equalToConstant: 30),
            scroll.topAnchor.constraint(equalTo: head.bottomAnchor, constant: 12),
            scroll.leadingAnchor.constraint(equalTo: leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: trailingAnchor),
            scroll.bottomAnchor.constraint(equalTo: bottomAnchor),
            doc.widthAnchor.constraint(equalTo: scroll.contentView.widthAnchor),
            stack.topAnchor.constraint(equalTo: doc.topAnchor),
            stack.leadingAnchor.constraint(equalTo: doc.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: doc.trailingAnchor),
            stack.bottomAnchor.constraint(equalTo: doc.bottomAnchor),
        ])
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    /// 搜索框回车：小程序搜索（PC 微信的搜索框也能搜小程序）
    @objc private func searched(_ f: NSSearchField) {
        let q = f.stringValue.trimmingCharacters(in: .whitespaces)
        if !q.isEmpty { main.showSearch(q) }
    }

    /// 一行：左图标 + 标题 / 副标题 + 右上角时间。选中时变灰（同时只有一行选中）
    @discardableResult
    func addRow(icon: NSView, title: String, sub: String = "", trailing: String = "", height: CGFloat = 64,
                selectedGreen: Bool = false, onClick: @escaping () -> Void) -> HoverControl {
        let r = HoverControl()
        let t = label(title, 14)
        let texts = NSStackView(views: [t])
        texts.orientation = .vertical
        texts.alignment = .leading
        texts.spacing = 4
        if !sub.isEmpty { texts.addArrangedSubview(label(sub, 12, Theme.sub)) }
        let h = NSStackView(views: [icon, texts])
        h.orientation = .horizontal
        h.alignment = .centerY
        h.spacing = 12
        h.edgeInsets = NSEdgeInsets(top: 0, left: 14, bottom: 0, right: 14)
        if !trailing.isEmpty {
            h.addArrangedSubview(NSView())
            let tr = label(trailing, 11, NSColor(argb: 0xFFB2B2B2))
            h.addArrangedSubview(tr)
        }
        r.addSubview(h)
        pin(h, to: r)
        fixed(r, h: height)
        if selectedGreen {
            r.selectedColor = Theme.green
        }
        r.onClick = { [weak self, weak r] in
            guard let self, let r else { return }
            self.rows.forEach { $0.isSelected = $0 === r }
            self.recolor(green: selectedGreen)
            onClick()
        }
        rows.append(r)
        stack.addArrangedSubview(r)
        r.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
        return r
    }

    /// 绿底选中（发现列表）时文字变白，其余行恢复黑字
    private func recolor(green: Bool) {
        guard green else { return }
        for r in rows {
            for case let l as NSTextField in allLabels(r) { l.textColor = r.isSelected ? .white : Theme.text }
        }
    }

    private func allLabels(_ v: NSView) -> [NSView] { v.subviews.flatMap { [$0] + allLabels($0) } }

    func addSection(_ title: String) {
        let l = label(title, 12, Theme.sub)
        let wrap = NSView()
        wrap.addSubview(l)
        pin(l, to: wrap, NSEdgeInsets(top: 12, left: 14, bottom: 6, right: 14))
        stack.addArrangedSubview(wrap)
    }
}

// MARK: - 聊天

/// 会话列表（演示数据）：几个人、几个群，有两条是小程序分享卡片。点了右边出聊天
final class ChatListPane: ListPane {
    struct Chat {
        let id: String
        let name: String
        let last: String
        let time: String
        let messages: [ChatView.Message]
    }

    init(main: MainWindowController) {
        super.init(main: main, search: true)
        let pick = Array(main.apps.prefix(6))
        func card(_ i: Int) -> ChatView.Message {
            pick.indices.contains(i) ? .miniProgram(pick[i], mine: i % 2 == 1) : .text("（商店还没连上，晚点再看看）", mine: false)
        }
        let chats = [
            Chat(id: "team", name: "引擎研发群", last: "[小程序] \(pick.first?.name ?? "小程序")", time: "16:45", messages: [
                .text("新版 Mac 端 SDK 出来了，小程序直接在独立窗口里开", mine: false),
                .text("我试一下这个", mine: true), card(0),
                .text("GPU 合成，跟手机上一个画法", mine: false)]),
            Chat(id: "xy", name: "丁小雨", last: "走起！吃火锅去", time: "16:42", messages: [
                .text("晚上一起吃饭？", mine: true), .text("走起！吃火锅去", mine: false), card(1)]),
            Chat(id: "pm", name: "产品-王磊", last: "[小程序] \(pick.dropFirst(2).first?.name ?? "小程序")", time: "16:12", messages: [
                .text("这个页面帮忙看看手感", mine: false), card(2), .text("滑起来很顺", mine: true)]),
            Chat(id: "file", name: "文件传输助手", last: "内置示例", time: "13:57", messages: [
                .demo, .text("App 里自带的演示小程序（assets/demo），不用联网", mine: false)]),
            Chat(id: "hr", name: "孙悦（HR）", last: "入职材料记得带齐", time: "昨天", messages: [.text("入职材料记得带齐", mine: false)]),
            Chat(id: "fam", name: "家人群", last: "[小程序] \(pick.dropFirst(3).first?.name ?? "小程序")", time: "星期六", messages: [
                .text("这个挺好用的", mine: false), card(3)]),
        ]
        for c in chats {
            addRow(icon: IconView(id: c.id, name: c.name, size: 40, corner: 6), title: c.name, sub: c.last, trailing: c.time) { [weak main] in
                guard let main else { return }
                main.showContent(ChatView(main: main, title: c.name, messages: c.messages))
            }
        }
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
}

// MARK: - 通讯录 / 收藏 / 发现

final class ContactsPane: ListPane {
    init(main: MainWindowController) {
        super.init(main: main, title: "通讯录")
        for (id, n) in [("new", "新的朋友"), ("group", "群聊"), ("tag", "标签"), ("gzh", "公众号")] {
            addRow(icon: IconView(id: id, name: n, size: 36, corner: 6), title: n, height: 56) {}
        }
        addSection("联系人")
        for n in ["丁小雨", "孙悦（HR）", "王磊（组长）", "白露（设计组）", "张伟（房东）"] {
            addRow(icon: IconView(id: n, name: n, size: 36, corner: 6), title: n, height: 56) {}
        }
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
}

/// 收藏：本机收藏过的小程序（「我的小程序」），点了打开、右键出操作单
final class FavoritesPane: ListPane {
    init(main: MainWindowController) {
        super.init(main: main, title: "我的小程序")
        let fav = StoreLocal().favorites()
        if fav.isEmpty {
            addSection("右键任意小程序，选「添加到我的小程序」")
        }
        for a in fav {
            let r = addRow(icon: IconView(store: main.store, app: a, size: 36), title: a.name, sub: a.description, height: 60) { [weak main] in
                main?.open(a, scene: 1104)
            }
            r.menuProvider = { [weak main] in main?.menu(for: a, scene: 1104) }
        }
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
}

/// 发现：朋友圈、视频号、搜一搜、游戏、小程序（PC 微信的发现页，选中那一行绿底白字）
final class DiscoverPane: ListPane {
    init(main: MainWindowController) {
        super.init(main: main, title: "发现")
        let rows: [(String, NSColor, String, () -> Void)] = [
            ("camera.aperture", NSColor(argb: 0xFFFA9D3B), "朋友圈", { [weak main] in main?.showPlaceholder("朋友圈") }),
            ("play.rectangle", NSColor(argb: 0xFFFA9D3B), "视频号", { [weak main] in main?.showPlaceholder("视频号") }),
            ("magnifyingglass", NSColor(argb: 0xFFFA5151), "搜一搜", { [weak main] in main?.showSearch() }),
            ("gamecontroller", NSColor(argb: 0xFF10AEFF), "游戏", { [weak main] in main?.showHub(gamesOnly: true) }),
            ("link.circle", NSColor(argb: 0xFF6467F0), "小程序", { [weak main] in main?.showHub() }),
        ]
        for (sym, color, title, action) in rows {
            let img = NSImageView(image: NSImage(systemSymbolName: sym, accessibilityDescription: title) ?? NSImage())
            img.contentTintColor = color
            img.symbolConfiguration = .init(pointSize: 16, weight: .regular)
            fixed(img, w: 22, h: 22)
            addRow(icon: img, title: title, height: 52, selectedGreen: true, onClick: action)
        }
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
}

extension MainWindowController {
    func showPlaceholder(_ title: String) {
        let v = NSView()
        let l = label("「\(title)」是演示占位：这个示例只演示小程序", 13, Theme.sub)
        v.addSubview(l)
        l.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([l.centerXAnchor.constraint(equalTo: v.centerXAnchor), l.centerYAnchor.constraint(equalTo: v.centerYAnchor)])
        showContent(v)
    }
}
