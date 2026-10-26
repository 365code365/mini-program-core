import UIKit

/// 微信式的下拉面板容器：[content]（首页）在上、[panel]（深色小程序面板）在后。
/// 首页滚到顶再往下拉，首页跟着手指下沉（阻尼 [damping]）、露出后面的面板；
/// 松手时拉过 [openPt] 就展开（首页沉到底只露 [peekPt] 那一截），否则弹回。
/// 展开后：点 / 上推露出的那一截收起；面板滚到底再往上推也跟手收起。
/// 与 Android 示例 PullDrawerLayout.kt、Flutter 示例 lib/store/store_drawer.dart 同一套阈值与手感。
///
/// 拉动开始之前触摸照常交给首页的滚动视图（点按、滚动都不受影响）；判定是「在顶上往下拉」之后才接管 ——
/// 接管时把滚动视图的手势停掉，同一次拖动里首页内容与面板只有一个在动。
final class PullDrawerView: UIView, UIGestureRecognizerDelegate {
    /// 松手时拉过这么多（已经算上阻尼）就展开
    static let openPt: CGFloat = 72
    static let peekPt: CGFloat = 64
    /// 手指走 1，页面走 0.55：拉起来发沉，与微信一样
    static let damping: CGFloat = 0.55

    /// 展开 / 收起（动画结束时，true = 展开）：入口页据此换状态栏颜色
    var onStateChanged: ((Bool) -> Void)?

    private let panel: UIView
    private let content: UIView
    private let contentScroll: UIScrollView
    private let panelScroll: UIScrollView
    private let scrim = UIView()
    private(set) var progress: CGFloat = 0
    private var animating = false
    private enum Mode { case none, pull, peek, panel }
    private var mode = Mode.none
    private var lastY: CGFloat = 0
    private var moved = false

    var isOpen: Bool { progress > 0 }

    init(panel: UIView, content: UIView, contentScroll: UIScrollView, panelScroll: UIScrollView) {
        self.panel = panel
        self.content = content
        self.contentScroll = contentScroll
        self.panelScroll = panelScroll
        super.init(frame: .zero)
        backgroundColor = .black
        for v in [panel, content, scrim] {
            addSubview(v)
            v.frame = bounds
            v.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        }
        content.layer.masksToBounds = true
        scrim.backgroundColor = .black
        scrim.alpha = 0
        scrim.isHidden = true
        panel.isHidden = true
        let pan = UIPanGestureRecognizer(target: self, action: #selector(onPan(_:)))
        pan.delegate = self
        addGestureRecognizer(pan)
        scrim.addGestureRecognizer(UITapGestureRecognizer(target: self, action: #selector(onScrimTap)))
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    /// 首页露在底下的高度（面板的内容要让出这么多）
    var peek: CGFloat { Self.peekPt + safeAreaInsets.bottom }
    private var travel: CGFloat { max(1, bounds.height - peek) }

    func open() { animate(to: 1) }

    func close() { animate(to: 0) }

    private func set(_ v: CGFloat) {
        progress = min(1, max(0, v))
        let p = progress
        let ty = p * travel
        content.transform = CGAffineTransform(translationX: 0, y: ty)
        content.layer.cornerRadius = 16 * p
        scrim.transform = content.transform
        scrim.alpha = 0.25 * p
        scrim.isHidden = p == 0
        panel.isHidden = p == 0
        panel.alpha = min(1, p * 1.6)
        let s = 0.94 + 0.06 * p
        panel.transform = CGAffineTransform(translationX: 0, y: -48 * (1 - p)).scaledBy(x: s, y: s)
    }

    private func animate(to target: CGFloat) {
        if target == 1, progress < 1 { UIImpactFeedbackGenerator(style: .light).impactOccurred() }
        animating = true
        // DecelerateInterpolator(1.6) ≈ easeOut 的三次曲线；300ms
        UIView.animate(withDuration: 0.3, delay: 0, options: [.curveEaseOut, .beginFromCurrentState, .allowUserInteraction], animations: {
            self.set(target)
        }, completion: { _ in
            self.animating = false
            self.onStateChanged?(self.progress > 0)
        })
    }

    // MARK: - 手势

    func gestureRecognizer(_ g: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith other: UIGestureRecognizer) -> Bool { true }

    private var contentAtTop: Bool { contentScroll.contentOffset.y <= -contentScroll.adjustedContentInset.top + 0.5 }
    private var panelAtBottom: Bool {
        let s = panelScroll
        return s.contentOffset.y + s.bounds.height >= s.contentSize.height + s.adjustedContentInset.bottom - 0.5
    }

    @objc private func onScrimTap() { if !animating { close() } }

    @objc private func onPan(_ g: UIPanGestureRecognizer) {
        if animating { return }
        let loc = g.location(in: self)
        let t = g.translation(in: self)
        switch g.state {
        case .began:
            moved = false
            lastY = t.y
            // 展开时露在底下那一截：整块归这里（点 = 收起，拖 = 跟手收）
            mode = progress > 0 && loc.y - t.y >= content.transform.ty ? .peek : .none
        case .changed:
            let vertical = abs(t.y) > 8 && abs(t.y) > abs(t.x)
            if mode == .none {
                if progress == 0, vertical, t.y > 0, contentAtTop { mode = .pull; freeze(contentScroll) }
                else if progress == 1, vertical, t.y < 0, panelAtBottom { mode = .panel; freeze(panelScroll) }
                if mode != .none { lastY = t.y }
            }
            guard mode != .none else { return }
            let d = t.y - lastY
            lastY = t.y
            if abs(t.y) > 8 { moved = true }
            set(progress + d * (mode == .pull ? Self.damping : 1) / travel)
        case .ended, .cancelled, .failed:
            let vy = g.velocity(in: self).y
            switch mode {
            case .none: break
            case .peek where !moved: close()
            case .pull: progress * travel >= Self.openPt || vy > 1200 ? open() : close()
            default: vy < -300 || progress < 0.85 ? close() : open()
            }
            if mode != .none { unfreeze() }
            mode = .none
        default: break
        }
    }

    /// 接管之后滚动视图别再跟着动（停掉它这一次手势，松手后再放开）
    private var frozen: UIScrollView?
    private func freeze(_ s: UIScrollView) {
        frozen = s
        let top = -s.adjustedContentInset.top
        if s === contentScroll { s.setContentOffset(CGPoint(x: 0, y: top), animated: false) }
        s.panGestureRecognizer.isEnabled = false
        s.panGestureRecognizer.isEnabled = true
        s.isScrollEnabled = false
    }

    private func unfreeze() {
        frozen?.isScrollEnabled = true
        frozen = nil
    }
}
