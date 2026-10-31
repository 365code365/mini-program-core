import QuartzCore
import UIKit
#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

/// 高性能+ 的 App 端（[MiniWebGameView] 在 `plus` 时持有）：游戏 JS 在 WKWebView 里跑（JIT），画布 / WebGL 命令每帧一包交到这里，
/// 由一个回放端引擎实例（`mr_game_create` 带 `"remote": true`，src/game/remote）经 ANGLE 直接画进 [metalLayer] ——
/// 与普通模式同一条上屏路径（[GameSession]）。WKWebView 透明地盖在上面，只画 Toast / Modal 这些覆盖层、接触摸。
///
/// 两种模式的好处兼得：JS 有 JIT（高性能模式），画面不经 WebKit 的 GPU 进程与合成器（普通模式的 ANGLE → CAMetalLayer）。
/// 主画布是 2D 时走 CPU 帧（[imageLayer]），与普通模式一样。
final class WebGamePlusRenderer: NSObject, GameSessionUI {
    let session: GameSession
    private let metalLayer = CAMetalLayer()
    private let imageLayer = CALayer()
    private weak var host: UIView?
    private var layerPixels = CGSize.zero
    /// 回放端的回调（图片解码完、上下文丢失、帧尾 ack）要在页面里执行
    var evalJS: ((String) -> Void)?
    var onFirstFrame: (() -> Void)?

    init(host: UIView, dir: String) {
        self.host = host
        session = GameSession(dir: dir, remote: true)
        super.init()
        for l in [imageLayer, metalLayer] {
            l.anchorPoint = .zero
            l.isHidden = true
            l.actions = ["contents": NSNull(), "bounds": NSNull(), "position": NSNull(), "transform": NSNull(), "hidden": NSNull()]
            // 垫在所有子视图（WKWebView）下面
            host.layer.insertSublayer(l, at: 0)
        }
        imageLayer.contentsGravity = .resize
        imageLayer.magnificationFilter = .linear
        metalLayer.isOpaque = true
        metalLayer.contentsScale = 1
        metalLayer.pixelFormat = .bgra8Unorm
        session.ui = self
    }

    /// 建回放端实例（选项与页面那一端同一份，带 remote:true）并挂上图层
    func start(dataDir: String, width: UInt32, height: UInt32, dpr: Float, options: String) {
        session.create(dataDir: dataDir, width: width, height: height, dpr: dpr, options: options)
        session.attachLayer(metalLayer)
    }

    func replay(_ b64: String, done: ((String) -> Void)? = nil) { session.replay(b64, done: done) }

    func layout(_ bounds: CGRect) {
        imageLayer.frame = bounds
        let px = layerPixels
        guard px.width > 0, px.height > 0, bounds.width > 0 else { return }
        metalLayer.transform = CATransform3DIdentity
        metalLayer.bounds = CGRect(origin: .zero, size: px)
        metalLayer.drawableSize = px
        metalLayer.position = .zero
        metalLayer.transform = CATransform3DMakeScale(bounds.width / px.width, bounds.height / px.height, 1)
    }

    func resize(width: UInt32, height: UInt32, dpr: Float, orientation: String) {
        session.resize(width: width, height: height, dpr: dpr, orientation: orientation)
    }
    func setSafeArea(_ i: UIEdgeInsets) { session.setSafeArea(i) }
    func foreground() { session.foreground() }
    func background() { session.background() }
    func dispose() { session.dispose() }

    // ───────────────────────── GameSessionUI ─────────────────────────

    func gameLayerWant(width: Int, height: Int) {
        layerPixels = CGSize(width: width, height: height)
        if let h = host { layout(h.bounds) }
        session.layerSized(width: width, height: height)
    }

    func gameShowLayer(metal: Bool) {
        metalLayer.isHidden = !metal
        imageLayer.isHidden = metal
        if metal { imageLayer.contents = nil }
    }

    func gameCpuFrame(_ image: CGImage) { imageLayer.contents = image }
    func gameFirstFrame() { onFirstFrame?() }
    /// 胶囊由页面那一端的实例摆（mr_wg_menu_button，同一套算法）
    func gameMenuButton(_ rect: CGRect) {}

    func gameEvents(_ events: [[String: Any]]) {
        for ev in events where ev["type"] as? String == "wgr" {
            if let js = ev["js"] as? String { evalJS?(js) }
        }
    }
}
