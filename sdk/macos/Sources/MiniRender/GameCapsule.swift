import AppKit

/// 小游戏右上角的胶囊「••• | ⊙」（macOS；与 iOS 的 GameCapsule.swift、Android 的 GameCapsule.kt 同一套尺寸与配色，
/// 小程序引擎画的那个在 src/host/capsule.rs）。原生 NSView 叠在游戏图层上面，不画进游戏画面；
/// 位置由 [MiniGameView] 按引擎给的矩形摆（`mr_game_menu_button`）。落在胶囊里的点击归胶囊，不交给游戏。
final class GameCapsule: NSView {
    var dark = false { didSet { needsDisplay = true } }
    var onTap: ((_ action: String) -> Void)?
    /// 按着的那一半：0 = 没按，1 = 「•••」，2 = 「⊙」
    private var pressedHalf = 0 { didSet { if oldValue != pressedHalf { needsDisplay = true } } }

    override var isFlipped: Bool { true }
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    private func gray(_ white: CGFloat, _ alpha: CGFloat) -> NSColor { NSColor(white: white, alpha: alpha) }

    override func draw(_ dirtyRect: NSRect) {
        let w = bounds.width, h = bounds.height
        let hair = 1 / max(1, window?.backingScaleFactor ?? 2)
        let bg = dark ? gray(0, 38 / 255) : gray(1, 153 / 255)
        let line = dark ? gray(1, 64 / 255) : gray(0, 20 / 255)
        let divider = dark ? gray(1, 64 / 255) : gray(0, 51 / 255)
        let icon: NSColor = dark ? .white : .black
        let press = dark ? gray(1, 46 / 255) : gray(0, 26 / 255)
        let pill = NSBezierPath(roundedRect: bounds, xRadius: h / 2, yRadius: h / 2)
        bg.setFill()
        pill.fill()
        if pressedHalf != 0 {
            NSGraphicsContext.saveGraphicsState()
            NSBezierPath(rect: pressedHalf == 1 ? NSRect(x: 0, y: 0, width: w / 2, height: h) : NSRect(x: w / 2, y: 0, width: w / 2, height: h)).addClip()
            press.setFill()
            pill.fill()
            NSGraphicsContext.restoreGraphicsState()
        }
        line.setStroke()
        let border = NSBezierPath(roundedRect: bounds.insetBy(dx: hair / 2, dy: hair / 2), xRadius: h / 2 - hair / 2, yRadius: h / 2 - hair / 2)
        border.lineWidth = hair
        border.stroke()
        // 竖分隔线：高度占 18/32
        divider.setFill()
        let dh = h * 18 / 32
        NSRect(x: w / 2 - hair / 2, y: (h - dh) / 2, width: hair, height: dh).fill()
        // 左半：三个点，中间那个大一圈
        icon.setFill()
        let cy = h / 2, mx = w / 4
        for (off, r) in [(-6.2, 1.7), (0.0, 2.4), (6.2, 1.7)] as [(CGFloat, CGFloat)] {
            NSBezierPath(ovalIn: NSRect(x: mx + off - r, y: cy - r, width: r * 2, height: r * 2)).fill()
        }
        // 右半：圆环 + 圆心
        let cx = w * 0.75
        icon.setStroke()
        let ringR: CGFloat = 7.4 - 1.9 / 2
        let ring = NSBezierPath(ovalIn: NSRect(x: cx - ringR, y: cy - ringR, width: ringR * 2, height: ringR * 2))
        ring.lineWidth = 1.9
        ring.stroke()
        NSBezierPath(ovalIn: NSRect(x: cx - 2.7, y: cy - 2.7, width: 5.4, height: 5.4)).fill()
    }

    private func half(_ e: NSEvent) -> Int {
        let p = convert(e.locationInWindow, from: nil)
        guard bounds.contains(p) else { return 0 }
        return p.x < bounds.width / 2 ? 1 : 2
    }

    override func mouseDown(with e: NSEvent) { pressedHalf = half(e) }

    // 移出按下的那一半就不算点击（与引擎画的胶囊同一条规则）
    override func mouseDragged(with e: NSEvent) {
        if pressedHalf != 0 && half(e) != pressedHalf { pressedHalf = 0 }
    }

    override func mouseUp(with e: NSEvent) {
        let h = pressedHalf
        pressedHalf = 0
        if h != 0 && half(e) == h { onTap?(h == 1 ? "menu" : "close") }
    }
}
