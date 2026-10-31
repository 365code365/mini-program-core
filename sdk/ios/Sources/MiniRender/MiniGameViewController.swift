import UIKit
#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

/// 小游戏宿主视图的公共接口：QuickJS 模式的 [MiniGameView] 与高性能模式的 [MiniWebGameView] 都实现它，
/// 宿主（支付、广告、登录这些 hostCall 的处理）不用管当前是哪种模式
public protocol MiniGameHosting: UIView {
    var onHostEvent: ((_ type: String, _ event: [String: Any]) -> Void)? { get set }
    var onFirstFrame: (() -> Void)? { get set }
    var onOrientationChange: ((UIInterfaceOrientationMask) -> Void)? { get set }
    var orientationMask: UIInterfaceOrientationMask { get }
    func hostReply(_ id: Int, ok: Bool, result: [String: Any])
    func dispatchEvent(_ name: String, json: String)
    func eval(_ js: String, _ done: @escaping (String?) -> Void)
    func close()
}

extension MiniGameView: MiniGameHosting {}
extension MiniWebGameView: MiniGameHosting {}

/// 小游戏的运行模式（对标微信 iOS 的普通模式 / 高性能模式）
public enum MiniGameMode: String {
    /// 按 game.json：`iOSHighPerformance`（或 `iOSHighPerformance+`）为 true 走高性能模式，否则普通模式。
    /// 启动参数 `-gameMode web | native` 可以强制（对照用）
    case auto
    /// 普通模式：JS 在 QuickJS 里（解释执行），WebGL 经 ANGLE、2D 画布 CPU 光栅（[MiniGameView]）
    case standard
    /// 高性能模式：JS 在 WKWebView 里（JIT），WebGL / 2D 是 WebKit 的 GPU 实现（[MiniWebGameView]）。
    /// 包里用了开放数据域 / 引擎插件时回落普通模式（日志一行）
    case highPerformance
    /// 高性能+：JS 在 WKWebView 里（JIT），画布 / WebGL 命令每帧交回 App、经 ANGLE 直接上屏（[WebGamePlusRenderer]）。
    /// 与微信的「高性能+」同一个思路：渲染挪回宿主进程，不经 WebKit 的 GPU 进程与合成器
    case highPerformancePlus

    /// 这个包实际走哪种
    public static func resolve(_ mode: MiniGameMode, dir: String) -> MiniGameMode {
        var want = mode
        // 启动参数 -gameMode web | native 优先（对照用），其次宿主给的，最后看 game.json
        switch UserDefaults.standard.string(forKey: "gameMode") {
        case "web": want = .highPerformance
        case "plus": want = .highPerformancePlus
        case "native": want = .standard
        default:
            if want == .auto {
                switch mr_wg_wanted(dir) {
                case 2: want = .highPerformancePlus
                case 1: want = .highPerformance
                default: want = .standard
                }
            }
        }
        guard want == .highPerformance || want == .highPerformancePlus else { return .standard }
        switch mr_wg_support(dir) {
        case 0: return want
        case 1: NSLog("[mini-game] 用了开放数据域，高性能模式不支持，走普通模式"); return .standard
        case 2: NSLog("[mini-game] 用了引擎插件，高性能模式不支持，走普通模式"); return .standard
        default: return .standard
        }
    }
}

/// 全屏跑一个小游戏的控制器（与 Android 示例的 GameActivity 对应）：藏状态栏与 home 横条、按游戏要的方向转屏、
/// `wx.exitMiniProgram` 关掉本页、`wx.restartMiniProgram` 换一个新实例（从 game.js 重新跑）。
///
/// 用 `present(_, animated:)` 弹出（`modalPresentationStyle` 已是 `.fullScreen`）：方向由**被弹出的**控制器决定，
/// 放进 UINavigationController 的栈里时导航控制器不问子控制器。转屏要 Info.plist 的 UISupportedInterfaceOrientations 里有横屏。
open class MiniGameViewController: UIViewController {
    public let gameDir: String
    public let dataDir: String?
    public let options: [String: Any]
    /// 实际的运行模式（[MiniGameMode.resolve] 之后的）
    public let mode: MiniGameMode
    public private(set) var gameView: MiniGameHosting!
    /// 引擎事件（exit / restart 之外的，含 hostCall）：宿主据此接支付、广告、登录 …（用 [gameView] 的 hostReply 回）
    public var onHostEvent: ((_ type: String, _ event: [String: Any]) -> Void)?
    /// 第一帧上屏（只回调一次；restart 之后的新实例不再回调）
    public var onFirstFrame: (() -> Void)?
    /// 本页关掉了（exit / 宿主调 [close]）
    public var onClose: (() -> Void)?
    private var firstFrameSent = false

    public init(gameDir: String, dataDir: String? = nil, options: [String: Any] = [:], mode: MiniGameMode = .auto) {
        self.gameDir = gameDir
        self.dataDir = dataDir
        self.options = options
        self.mode = MiniGameMode.resolve(mode, dir: gameDir)
        NSLog("[mini-game] 运行模式：%@", self.mode == .highPerformance ? "高性能（WKWebView）" : self.mode == .highPerformancePlus ? "高性能+（WKWebView + ANGLE）" : "普通（QuickJS）")
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
        let v: MiniGameHosting = mode == .standard
            ? MiniGameView(gameDir: gameDir, dataDir: dataDir, options: options)
            : MiniWebGameView(gameDir: gameDir, dataDir: dataDir, options: options, plus: mode == .highPerformancePlus)
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
            // 胶囊「⊙」= 退出（与微信一致）；「•••」交给宿主，宿主没接就弹一个最小的菜单（重新进入 / 退出）
            case "menuButton" where ev["action"] as? String == "close": self.close()
            case "menuButton" where self.onHostEvent == nil: self.showDefaultMenu()
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

    private func showDefaultMenu() {
        let a = UIAlertController(title: nil, message: nil, preferredStyle: .actionSheet)
        a.addAction(UIAlertAction(title: "重新进入小游戏", style: .default) { [weak self] _ in self?.restart() })
        a.addAction(UIAlertAction(title: "退出", style: .destructive) { [weak self] _ in self?.close() })
        a.addAction(UIAlertAction(title: "取消", style: .cancel))
        a.popoverPresentationController?.sourceView = view
        present(a, animated: true)
    }

    /// wx.restartMiniProgram：旧实例同步销毁，新视图从 game.js 重新跑
    public func restart() {
        gameView.close()
        gameView.removeFromSuperview()
        newGame()
    }

    /// 关掉本页（销毁实例）。多次调用只关一次
    ///
    /// 顺序：先收起本页，收起完了再转回竖屏、再调 [onClose]。从前先调 onClose：宿主在里面收起它自己那一页
    /// （iOS 示例的 MiniViewController 就是），而那一刻它还弹着本页 —— UIKit 把「收起自己」当成「收起它弹出的那一页」，
    /// 结果只收起了游戏，宿主那一页留在屏上：白屏（萌宠小岛点「⊙」）。
    /// 转屏同理：本页还在最上面时它要横屏，退出前转不回去；收起之后按底下那一页要的方向转（iOS 16 起要显式请求，
    /// 底下那页支持横屏的话系统不会自己转回竖屏 —— 玩完横屏游戏回来还是横的）
    public func close() {
        guard !closing else { return }
        closing = true
        gameView.close()
        let done = onClose
        if let nav = navigationController, nav.viewControllers.first !== self {
            nav.popViewController(animated: true)
            Self.restoreOrientation(for: nav)
            done?()
        } else if let p = presentingViewController {
            p.dismiss(animated: true) {
                Self.restoreOrientation(for: p)
                done?()
            }
        } else {
            done?()
        }
    }

    private var closing = false

    /// 游戏退出后转回 `vc` 要的方向：它支持竖屏就转竖屏（与微信一致，玩完横屏游戏回到竖着的页面）
    public static func restoreOrientation(for vc: UIViewController) {
        let mask = vc.supportedInterfaceOrientations
        let want: UIInterfaceOrientationMask = mask.contains(.portrait) ? .portrait : mask
        if #available(iOS 16.0, *) {
            vc.setNeedsUpdateOfSupportedInterfaceOrientations()
            guard let scene = vc.view.window?.windowScene else { return }
            scene.requestGeometryUpdate(.iOS(interfaceOrientations: want)) { e in
                NSLog("[mini-game] 转回竖屏没成功：%@", e.localizedDescription)
            }
        } else if want.contains(.portrait) {
            UIDevice.current.setValue(UIInterfaceOrientation.portrait.rawValue, forKey: "orientation")
            UIViewController.attemptRotationToDeviceOrientation()
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
