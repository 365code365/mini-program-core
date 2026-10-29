import UIKit

/// 小游戏右上角的胶囊「••• | ⊙」（与小程序引擎画的那个同一套尺寸与配色，src/host/capsule.rs；Android 是 GameCapsule.kt）。
///
/// 原生 UIView 叠在游戏图层上面，不画进游戏画面：WebGL 直接上屏时引擎往帧里叠东西要每帧多一次整帧合成，
/// 原生视图由 Core Animation 合成，游戏那一帧不多花。位置由 [MiniGameView] 按引擎给的矩形摆
/// （`mr_game_menu_button`，与 wx.getMenuButtonBoundingClientRect 同一个数）。落在胶囊里的触摸归胶囊，不交给游戏。
final class GameCapsule: UIView {
    /// wx.setMenuStyle('dark')：深底白图标
    var dark = false { didSet { setNeedsDisplay() } }
    var onTap: ((_ action: String) -> Void)?
    /// 按着的那一半：0 = 没按，1 = 「•••」，2 = 「⊙」
    private var pressedHalf = 0 { didSet { if oldValue != pressedHalf { setNeedsDisplay() } } }

    override init(frame: CGRect) {
        super.init(frame: frame)
        isOpaque = false
        backgroundColor = .clear
        contentMode = .redraw
        isAccessibilityElement = false
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    private func gray(_ white: CGFloat, _ alpha: CGFloat) -> UIColor { UIColor(white: white, alpha: alpha) }

    override func draw(_ rect: CGRect) {
        guard let ctx = UIGraphicsGetCurrentContext() else { return }
        let w = bounds.width, h = bounds.height
        let hair = 1 / max(1, contentScaleFactor) // 0.5pt @2x / 0.33pt @3x：至少一个物理像素
        let bg = dark ? gray(0, 38 / 255) : gray(1, 153 / 255)
        let line = dark ? gray(1, 64 / 255) : gray(0, 20 / 255)
        let divider = dark ? gray(1, 64 / 255) : gray(0, 51 / 255)
        let icon: UIColor = dark ? .white : .black
        let press = dark ? gray(1, 46 / 255) : gray(0, 26 / 255)
        let pill = UIBezierPath(roundedRect: bounds, cornerRadius: h / 2)
        bg.setFill()
        pill.fill()
        if pressedHalf != 0 {
            ctx.saveGState()
            UIRectClip(pressedHalf == 1 ? CGRect(x: 0, y: 0, width: w / 2, height: h) : CGRect(x: w / 2, y: 0, width: w / 2, height: h))
            press.setFill()
            pill.fill()
            ctx.restoreGState()
        }
        line.setStroke()
        let border = UIBezierPath(roundedRect: bounds.insetBy(dx: hair / 2, dy: hair / 2), cornerRadius: h / 2 - hair / 2)
        border.lineWidth = hair
        border.stroke()
        // 竖分隔线：高度占 18/32
        divider.setFill()
        let dh = h * 18 / 32
        UIRectFill(CGRect(x: w / 2 - hair / 2, y: (h - dh) / 2, width: hair, height: dh))
        // 左半：三个点，中间那个大一圈
        icon.setFill()
        let cy = h / 2, mx = w / 4
        for (off, r) in [(-6.2, 1.7), (0.0, 2.4), (6.2, 1.7)] as [(CGFloat, CGFloat)] {
            UIBezierPath(ovalIn: CGRect(x: mx + off - r, y: cy - r, width: r * 2, height: r * 2)).fill()
        }
        // 右半：圆环 + 圆心
        let cx = w * 0.75
        icon.setStroke()
        let ringR: CGFloat = 7.4 - 1.9 / 2
        let ring = UIBezierPath(ovalIn: CGRect(x: cx - ringR, y: cy - ringR, width: ringR * 2, height: ringR * 2))
        ring.lineWidth = 1.9
        ring.stroke()
        UIBezierPath(ovalIn: CGRect(x: cx - 2.7, y: cy - 2.7, width: 5.4, height: 5.4)).fill()
    }

    private func half(_ t: UITouch?) -> Int {
        guard let p = t?.location(in: self), bounds.contains(p) else { return 0 }
        return p.x < bounds.width / 2 ? 1 : 2
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) { pressedHalf = half(touches.first) }

    // 移出按下的那一半就不算点击（与引擎画的胶囊同一条规则）
    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        if pressedHalf != 0 && half(touches.first) != pressedHalf { pressedHalf = 0 }
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        let h = pressedHalf
        pressedHalf = 0
        if h != 0 && half(touches.first) == h { onTap?(h == 1 ? "menu" : "close") }
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) { pressedHalf = 0 }
}
