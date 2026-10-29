import UIKit

/// 全屏跑一个小游戏的控制器（与 Android 示例的 GameActivity 对应）：藏状态栏与 home 横条、按游戏要的方向转屏、
/// `wx.exitMiniProgram` 关掉本页、`wx.restartMiniProgram` 换一个新实例（从 game.js 重新跑）。
///
/// 用 `present(_, animated:)` 弹出（`modalPresentationStyle` 已是 `.fullScreen`）：方向由**被弹出的**控制器决定，
/// 放进 UINavigationController 的栈里时导航控制器不问子控制器。转屏要 Info.plist 的 UISupportedInterfaceOrientations 里有横屏。
open class MiniGameViewController: UIViewController {
    public let gameDir: String
    public let dataDir: String?
    public let options: [String: Any]
    public private(set) var gameView: MiniGameView!
    /// 引擎事件（exit / restart 之外的，含 hostCall）：宿主据此接支付、广告、登录 …（用 [gameView] 的 hostReply 回）
    public var onHostEvent: ((_ type: String, _ event: [String: Any]) -> Void)?
    /// 第一帧上屏（只回调一次；restart 之后的新实例不再回调）
    public var onFirstFrame: (() -> Void)?
    /// 本页关掉了（exit / 宿主调 [close]）
    public var onClose: (() -> Void)?
    private var firstFrameSent = false

    public init(gameDir: String, dataDir: String? = nil, options: [String: Any] = [:]) {
        self.gameDir = gameDir
        self.dataDir = dataDir
        self.options = options
        super.init(nibName: nil, bundle: nil)
        modalPresentationStyle = .fullScreen
    }

    public required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    open override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .black
        newGame()
    }

    private func newGame() {
        let v = MiniGameView(gameDir: gameDir, dataDir: dataDir, options: options)
        v.frame = view.bounds
        v.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        v.onOrientationChange = { [weak self] _ in self?.applyOrientation() }
        v.onFirstFrame = { [weak self] in
            guard let self, !self.firstFrameSent else { return }
            self.firstFrameSent = true
            self.onFirstFrame?()
        }
        v.onHostEvent = { [weak self] type, ev in
            guard let self else { return }
            switch type {
            case "exit": self.close()
            case "restart": self.restart()
            default:
                if let cb = self.onHostEvent { return cb(type, ev) }
                if type == "hostCall" {
                    v.hostReply((ev["id"] as? NSNumber)?.intValue ?? 0, ok: false, result: ["errMsg": "\(ev["api"] ?? ""):fail 宿主没有实现"])
                }
            }
        }
        view.insertSubview(v, at: 0)
        gameView = v
        applyOrientation()
    }

    /// wx.restartMiniProgram：旧实例同步销毁，新视图从 game.js 重新跑
    public func restart() {
        gameView.close()
        gameView.removeFromSuperview()
        newGame()
    }

    /// 关掉本页（销毁实例）
    public func close() {
        gameView.close()
        onClose?()
        if let nav = navigationController, nav.viewControllers.first !== self {
            nav.popViewController(animated: true)
        } else {
            presentingViewController?.dismiss(animated: true)
        }
    }

    open override var supportedInterfaceOrientations: UIInterfaceOrientationMask { gameView?.orientationMask ?? .portrait }
    open override var preferredInterfaceOrientationForPresentation: UIInterfaceOrientation {
        let m = supportedInterfaceOrientations
        return m.contains(.portrait) ? .portrait : m.contains(.landscapeRight) ? .landscapeRight : .landscapeLeft
    }
    open override var prefersStatusBarHidden: Bool { true }
    open override var prefersHomeIndicatorAutoHidden: Bool { true }
    /// 屏幕边缘的系统手势（下拉通知中心、上滑回桌面）要划两次：游戏贴边的按钮不被抢
    open override var preferredScreenEdgesDeferringSystemGestures: UIRectEdge { .all }

    /// 按游戏要的方向转屏（iOS 16 起要显式请求；之前靠 attemptRotationToDeviceOrientation）
    private func applyOrientation() {
        let mask = gameView?.orientationMask ?? .portrait
        if #available(iOS 16.0, *) {
            setNeedsUpdateOfSupportedInterfaceOrientations()
            view.window?.windowScene?.requestGeometryUpdate(.iOS(interfaceOrientations: mask)) { e in
                NSLog("[mini-game] 转屏没成功：%@", e.localizedDescription)
            }
        } else {
            let target: UIInterfaceOrientation = mask.contains(.portrait) ? .portrait : mask.contains(.landscapeRight) ? .landscapeRight : .landscapeLeft
            UIDevice.current.setValue(target.rawValue, forKey: "orientation")
            UIViewController.attemptRotationToDeviceOrientation()
        }
    }

    open override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        // 弹出之前 view.window 还是 nil，那时的请求没生效：出现之后再要一次
        applyOrientation()
    }
}
