import AppKit

/// 「灵犀 for Mac」的界面零件：颜色、圆形图标、卡片、小程序格子、可点的行。
/// 尺寸与配色照 macOS 版微信（左侧栏、会话列表、右侧内容区三栏），图标与商店配色与手机端示例同一套。
enum Theme {
    static let rail = NSColor(argb: 0xFFE7E7E7)
    static let list = NSColor(argb: 0xFFF0F0F0)
    static let content = NSColor(argb: 0xFFF7F7F7)
    static let hubBg = NSColor(argb: 0xFFF3F3F3)
    static let selected = NSColor(argb: 0xFFDCDCDC)
    static let hover = NSColor(argb: 0xFFE6E6E6)
    static let green = NSColor(argb: 0xFF07C160)
    static let text = NSColor(argb: 0xFF191919)
    static let sub = NSColor(argb: 0xFF8C8C8C)
    static let line = NSColor(argb: 0x14000000)
    private static let iconColors: [UInt32] = [0xFF07C160, 0xFF10AEFF, 0xFFFA9D3B, 0xFFE64340, 0xFF6467F0, 0xFF1485EE, 0xFF91D300, 0xFF576B95]

    /// 底色按 id 算（与管理页、手机端示例同一个哈希，几处颜色一样）
    static func color(_ id: String) -> NSColor {
        var h: UInt64 = 0
        for u in id.unicodeScalars { h = (h &* 31 &+ UInt64(u.value)) & 0xFFFF_FFFF }
        return NSColor(argb: iconColors[Int(h % UInt64(iconColors.count))])
    }
}

extension NSColor {
    convenience init(argb: UInt32) {
        self.init(srgbRed: CGFloat((argb >> 16) & 0xFF) / 255, green: CGFloat((argb >> 8) & 0xFF) / 255,
                  blue: CGFloat(argb & 0xFF) / 255, alpha: CGFloat(argb >> 24) / 255)
    }
}

/// y 朝下的视图（滚动区的 documentView 用：内容从顶上排起）
class FlippedView: NSView {
    override var isFlipped: Bool { true }
}

func label(_ s: String, _ size: CGFloat, _ color: NSColor = Theme.text, bold: Bool = false) -> NSTextField {
    let l = NSTextField(labelWithString: s)
    l.font = bold ? .boldSystemFont(ofSize: size) : .systemFont(ofSize: size)
    l.textColor = color
    l.lineBreakMode = .byTruncatingTail
    l.maximumNumberOfLines = 1
    l.cell?.truncatesLastVisibleLine = true
    return l
}

func fixed(_ v: NSView, w: CGFloat? = nil, h: CGFloat? = nil) {
    v.translatesAutoresizingMaskIntoConstraints = false
    if let w { v.widthAnchor.constraint(equalToConstant: w).isActive = true }
    if let h { v.heightAnchor.constraint(equalToConstant: h).isActive = true }
}

func pin(_ v: NSView, to p: NSView, _ i: NSEdgeInsets = NSEdgeInsets()) {
    v.translatesAutoresizingMaskIntoConstraints = false
    NSLayoutConstraint.activate([
        v.leadingAnchor.constraint(equalTo: p.leadingAnchor, constant: i.left),
        v.trailingAnchor.constraint(equalTo: p.trailingAnchor, constant: -i.right),
        v.topAnchor.constraint(equalTo: p.topAnchor, constant: i.top),
        v.bottomAnchor.constraint(equalTo: p.bottomAnchor, constant: -i.bottom),
    ])
}

/// 圆形图标：名字首字的圆，服务器有 icon.png 就后台拉下来盖上去（圆形裁剪）
final class IconView: NSView {
    private static let mem = NSCache<NSString, NSImage>()
    private static let pool: OperationQueue = {
        let q = OperationQueue()
        q.maxConcurrentOperationCount = 3
        return q
    }()
    private let size: CGFloat
    private let img = NSImageView()

    init(id: String, name: String, size: CGFloat, corner: CGFloat? = nil) {
        self.size = size
        super.init(frame: NSRect(x: 0, y: 0, width: size, height: size))
        wantsLayer = true
        layer?.cornerRadius = corner ?? size / 2
        layer?.masksToBounds = true
        layer?.backgroundColor = Theme.color(id).cgColor
        let l = label(name.first.map(String.init) ?? "?", size * 0.42, .white, bold: true)
        l.alignment = .center
        addSubview(l)
        l.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([l.centerXAnchor.constraint(equalTo: centerXAnchor), l.centerYAnchor.constraint(equalTo: centerYAnchor)])
        img.imageScaling = .scaleProportionallyUpOrDown
        addSubview(img)
        pin(img, to: self)
        fixed(self, w: size, h: size)
    }

    convenience init(store: MiniStore, app: MiniStore.App, size: CGFloat, corner: CGFloat? = nil) {
        self.init(id: app.id, name: app.name, size: size, corner: corner)
        guard !app.iconUrl.isEmpty else { return }
        let key = "\(app.id)@\(app.version)" as NSString
        if let hit = Self.mem.object(forKey: key) { img.image = hit; return }
        Self.pool.addOperation { [weak img] in
            guard let d = store.iconData(app), let im = NSImage(data: d) else { return }
            Self.mem.setObject(im, forKey: key)
            DispatchQueue.main.async { img?.image = im }
        }
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
}

/// 可点的区域：悬停变浅灰、按下变深灰（macOS 微信的列表行就是这样），右键 / 长按出菜单
class HoverControl: NSView {
    var onClick: (() -> Void)?
    var menuProvider: (() -> NSMenu?)?
    var normalColor: NSColor = .clear { didSet { refresh() } }
    var hoverColor = Theme.hover
    var selectedColor = Theme.selected
    var isSelected = false { didSet { refresh() } }
    private var hovering = false

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        refresh()
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        trackingAreas.forEach(removeTrackingArea)
        addTrackingArea(NSTrackingArea(rect: bounds, options: [.mouseEnteredAndExited, .activeInKeyWindow, .inVisibleRect], owner: self))
    }

    override func mouseEntered(with event: NSEvent) { hovering = true; refresh() }
    override func mouseExited(with event: NSEvent) { hovering = false; refresh() }

    override func mouseUp(with event: NSEvent) {
        if bounds.contains(convert(event.locationInWindow, from: nil)) { onClick?() }
    }

    override func mouseDown(with event: NSEvent) {}

    override func menu(for event: NSEvent) -> NSMenu? { menuProvider?() }

    private func refresh() {
        layer?.backgroundColor = (isSelected ? selectedColor : hovering ? hoverColor : normalColor).cgColor
    }

    override func resetCursorRects() { addCursorRect(bounds, cursor: .arrow) }
}

/// 一格小程序：圆图标 + 名字（hub 的「最近使用 / 我的常用 / 全部」、搜索结果都用它）
final class AppTile: HoverControl {
    init(store: MiniStore, app: MiniStore.App, iconSize: CGFloat = 48, fav: Bool = false) {
        super.init(frame: .zero)
        layer?.cornerRadius = 6
        hoverColor = NSColor(argb: 0x0A000000)
        let icon = IconView(store: store, app: app, size: iconSize)
        let name = label(app.name, 12, NSColor(argb: 0xFF4C4C4C))
        name.alignment = .center
        let col = NSStackView(views: [icon, name])
        col.orientation = .vertical
        col.spacing = 8
        col.edgeInsets = NSEdgeInsets(top: 10, left: 4, bottom: 8, right: 4)
        addSubview(col)
        pin(col, to: self)
        name.widthAnchor.constraint(lessThanOrEqualToConstant: iconSize + 34).isActive = true
        if fav {
            let star = label("★", 10, NSColor(argb: 0xFFFFB400))
            star.wantsLayer = true
            star.layer?.backgroundColor = NSColor.white.cgColor
            star.layer?.cornerRadius = 8
            star.alignment = .center
            icon.addSubview(star)
            star.frame = NSRect(x: iconSize - 16, y: 0, width: 16, height: 16)
        }
        toolTip = app.description.isEmpty ? app.name : "\(app.name)\n\(app.description)"
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
}

/// hub 里的白底圆角卡片：标题行（左标题、右「更多 >」）+ 内容
final class Card: NSView {
    let body = NSStackView()

    init(title: String, more: String? = nil, onMore: (() -> Void)? = nil) {
        super.init(frame: .zero)
        wantsLayer = true
        layer?.backgroundColor = NSColor.white.cgColor
        layer?.cornerRadius = 8
        let head = NSStackView()
        head.orientation = .horizontal
        head.addArrangedSubview(label(title, 13, NSColor(argb: 0xFF4C4C4C)))
        head.addArrangedSubview(NSView())
        if let more {
            let b = HoverControl()
            b.hoverColor = .clear
            let l = label(more + "  ›", 12, Theme.sub)
            b.addSubview(l)
            pin(l, to: b)
            b.onClick = onMore
            head.addArrangedSubview(b)
        }
        body.orientation = .vertical
        body.alignment = .leading
        body.spacing = 0
        let col = NSStackView(views: [head, body])
        col.orientation = .vertical
        col.alignment = .leading
        col.spacing = 6
        col.edgeInsets = NSEdgeInsets(top: 14, left: 16, bottom: 10, right: 16)
        addSubview(col)
        pin(col, to: self)
        head.widthAnchor.constraint(equalTo: col.widthAnchor, constant: -32).isActive = true
        body.widthAnchor.constraint(equalTo: col.widthAnchor, constant: -32).isActive = true
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
}

/// 一行小程序格子（不换行，放不下的截掉 —— 与 PC 微信的「最近使用」一样，点「更多」看全部）
func tileRow(_ tiles: [NSView], spacing: CGFloat = 14) -> NSStackView {
    let row = NSStackView(views: tiles)
    row.orientation = .horizontal
    row.spacing = spacing
    row.alignment = .top
    row.setHuggingPriority(.defaultLow, for: .horizontal)
    tiles.forEach { $0.setContentCompressionResistancePriority(.defaultLow, for: .horizontal) }
    return row
}

/// 自动换行的格子网格：每格定宽，按可用宽度算列数
final class TileGrid: NSView {
    private let tiles: [NSView]
    private let cellW: CGFloat
    private let cellH: CGFloat

    init(tiles: [NSView], cellW: CGFloat = 92, cellH: CGFloat = 96) {
        self.tiles = tiles
        self.cellW = cellW
        self.cellH = cellH
        super.init(frame: .zero)
        tiles.forEach { addSubview($0); $0.translatesAutoresizingMaskIntoConstraints = true }
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    override var isFlipped: Bool { true }

    private func cols(_ w: CGFloat) -> Int { max(1, Int(w / cellW)) }

    override func layout() {
        super.layout()
        let c = cols(bounds.width)
        for (i, t) in tiles.enumerated() {
            t.frame = NSRect(x: CGFloat(i % c) * cellW, y: CGFloat(i / c) * cellH, width: cellW, height: cellH)
        }
        let h = CGFloat((tiles.count + c - 1) / c) * cellH
        if abs(h - measured) > 0.5 {
            measured = h
            invalidateIntrinsicContentSize()
        }
    }

    private var measured: CGFloat = 96

    override var intrinsicContentSize: NSSize { NSSize(width: NSView.noIntrinsicMetric, height: measured) }
}
