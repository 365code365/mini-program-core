import AppKit

/// 右边的聊天窗：标题、消息气泡（对方白底、自己绿底）、小程序分享卡片（点了打开小程序），底下一个输入框。
/// 演示用：输入框回车只是把字作为自己的消息加进来。
final class ChatView: NSView {
    enum Message {
        case text(String, mine: Bool)
        case miniProgram(MiniStore.App, mine: Bool)
        /// 内置示例（App 里自带的演示小程序）
        case demo
    }

    private unowned let main: MainWindowController
    private let list = NSStackView()
    private let scroll = NSScrollView()
    private let input = NSTextField()

    init(main: MainWindowController, title: String, messages: [Message]) {
        self.main = main
        super.init(frame: .zero)
        let head = label(title, 15, Theme.text, bold: true)
        let line = NSView()
        line.wantsLayer = true
        line.layer?.backgroundColor = Theme.line.cgColor
        list.orientation = .vertical
        list.alignment = .leading
        list.spacing = 16
        list.edgeInsets = NSEdgeInsets(top: 18, left: 24, bottom: 18, right: 24)
        let doc = FlippedView()
        doc.addSubview(list)
        list.translatesAutoresizingMaskIntoConstraints = false
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        scroll.documentView = doc
        doc.translatesAutoresizingMaskIntoConstraints = false
        let bottom = NSView()
        bottom.wantsLayer = true
        bottom.layer?.backgroundColor = Theme.content.cgColor
        input.placeholderString = "发消息（演示：回车发出去）"
        input.isBordered = false
        input.drawsBackground = false
        input.focusRingType = .none
        input.font = .systemFont(ofSize: 14)
        input.target = self
        input.action = #selector(send)
        bottom.addSubview(input)
        for v in [head, line, scroll, bottom] {
            v.translatesAutoresizingMaskIntoConstraints = false
            addSubview(v)
        }
        input.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            head.topAnchor.constraint(equalTo: topAnchor, constant: 38),
            head.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 24),
            line.topAnchor.constraint(equalTo: topAnchor, constant: 72),
            line.leadingAnchor.constraint(equalTo: leadingAnchor),
            line.trailingAnchor.constraint(equalTo: trailingAnchor),
            line.heightAnchor.constraint(equalToConstant: 1),
            scroll.topAnchor.constraint(equalTo: line.bottomAnchor),
            scroll.leadingAnchor.constraint(equalTo: leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: trailingAnchor),
            scroll.bottomAnchor.constraint(equalTo: bottom.topAnchor),
            bottom.leadingAnchor.constraint(equalTo: leadingAnchor),
            bottom.trailingAnchor.constraint(equalTo: trailingAnchor),
            bottom.bottomAnchor.constraint(equalTo: bottomAnchor),
            bottom.heightAnchor.constraint(equalToConstant: 120),
            input.topAnchor.constraint(equalTo: bottom.topAnchor, constant: 16),
            input.leadingAnchor.constraint(equalTo: bottom.leadingAnchor, constant: 24),
            input.trailingAnchor.constraint(equalTo: bottom.trailingAnchor, constant: -24),
            doc.widthAnchor.constraint(equalTo: scroll.contentView.widthAnchor),
            list.topAnchor.constraint(equalTo: doc.topAnchor),
            list.leadingAnchor.constraint(equalTo: doc.leadingAnchor),
            list.trailingAnchor.constraint(equalTo: doc.trailingAnchor),
            list.bottomAnchor.constraint(equalTo: doc.bottomAnchor),
        ])
        let sep = NSView()
        sep.wantsLayer = true
        sep.layer?.backgroundColor = Theme.line.cgColor
        sep.translatesAutoresizingMaskIntoConstraints = false
        bottom.addSubview(sep)
        NSLayoutConstraint.activate([sep.topAnchor.constraint(equalTo: bottom.topAnchor), sep.leadingAnchor.constraint(equalTo: bottom.leadingAnchor),
                                     sep.trailingAnchor.constraint(equalTo: bottom.trailingAnchor), sep.heightAnchor.constraint(equalToConstant: 1)])
        messages.forEach(append)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    @objc private func send() {
        let s = input.stringValue.trimmingCharacters(in: .whitespaces)
        guard !s.isEmpty else { return }
        input.stringValue = ""
        append(.text(s, mine: true))
        DispatchQueue.main.async { self.scroll.documentView?.scroll(NSPoint(x: 0, y: self.list.frame.maxY)) }
    }

    private func append(_ m: Message) {
        let bubble: NSView
        let mine: Bool
        switch m {
        case let .text(s, me):
            mine = me
            let l = NSTextField(wrappingLabelWithString: s)
            l.font = .systemFont(ofSize: 14)
            l.textColor = Theme.text
            l.preferredMaxLayoutWidth = 360
            let b = NSView()
            b.wantsLayer = true
            b.layer?.cornerRadius = 4
            b.layer?.backgroundColor = (me ? NSColor(argb: 0xFF95EC69) : NSColor.white).cgColor
            b.addSubview(l)
            pin(l, to: b, NSEdgeInsets(top: 9, left: 12, bottom: 9, right: 12))
            bubble = b
        case let .miniProgram(app, me):
            mine = me
            bubble = miniCard(name: app.name, title: app.description.isEmpty ? app.name : app.description,
                              icon: IconView(store: main.store, app: app, size: 20)) { [weak main] in main?.open(app, scene: 1007) }
        case .demo:
            mine = false
            bubble = miniCard(name: "内置示例", title: "mini_render 能力演示：组件、接口、宿主能力", icon: IconView(id: "demo", name: "示", size: 20)) {
                [weak main] in main?.openDemo()
            }
        }
        let avatar = IconView(id: mine ? "me" : "peer", name: mine ? "灵" : "友", size: 34, corner: 4)
        let row = NSStackView()
        row.orientation = .horizontal
        row.alignment = .top
        row.spacing = 10
        if mine {
            row.addArrangedSubview(NSView())
            row.addArrangedSubview(bubble)
            row.addArrangedSubview(avatar)
        } else {
            row.addArrangedSubview(avatar)
            row.addArrangedSubview(bubble)
            row.addArrangedSubview(NSView())
        }
        list.addArrangedSubview(row)
        row.widthAnchor.constraint(equalTo: list.widthAnchor, constant: -48).isActive = true
    }

    /// 小程序分享卡片：顶上小图标 + 名字，中间标题，底下「小程序」（微信聊天里的样子）
    private func miniCard(name: String, title: String, icon: NSView, onClick: @escaping () -> Void) -> NSView {
        let c = HoverControl()
        c.normalColor = .white
        c.hoverColor = NSColor(argb: 0xFFF5F5F5)
        c.layer?.cornerRadius = 4
        let top = NSStackView(views: [icon, label(name, 12, Theme.sub)])
        top.spacing = 6
        let t = NSTextField(wrappingLabelWithString: title)
        t.font = .systemFont(ofSize: 14)
        t.textColor = Theme.text
        t.maximumNumberOfLines = 2
        t.preferredMaxLayoutWidth = 220
        let thumb = NSView()
        thumb.wantsLayer = true
        thumb.layer?.backgroundColor = NSColor(argb: 0xFFF2F3F5).cgColor
        fixed(thumb, w: 220, h: 120)
        let line = NSView()
        line.wantsLayer = true
        line.layer?.backgroundColor = Theme.line.cgColor
        fixed(line, w: 220, h: 1)
        let foot = NSStackView(views: [NSImageView(image: NSImage(systemSymbolName: "link.circle", accessibilityDescription: nil) ?? NSImage()),
                                       label("小程序", 11, Theme.sub)])
        foot.spacing = 4
        let col = NSStackView(views: [top, t, thumb, line, foot])
        col.orientation = .vertical
        col.alignment = .leading
        col.spacing = 8
        col.edgeInsets = NSEdgeInsets(top: 10, left: 12, bottom: 8, right: 12)
        c.addSubview(col)
        pin(col, to: c)
        c.onClick = onClick
        c.toolTip = "打开小程序「\(name)」"
        return c
    }
}
