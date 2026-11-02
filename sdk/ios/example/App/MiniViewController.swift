import MiniRender
import UIKit

/// 原生集成的宿主页：整个控制器只有一个 [MiniProgramView]，照 doc/原生App集成指南.md「极简集成」写。
/// 与 Android 示例 MainActivity.kt + MenuSheet.kt 一一对应。
///
/// 不带参数打开的是包里的内置示例；从入口页点开的带商店信息，外部 App / H5 的链接（`minirender://open?…`）
/// 带 [StoreLink] —— 这两种都是「问服务器最新版 → 不是最新就下载 → 打开」（[openStore]）。
/// 已经开着时再来一条链接 / 小程序里 `wx.navigateToMiniProgram`：就地换成另一个小程序（单实例，先关后开）。
///
/// 每条引擎事件都 `NSLog` 一行 `[事件] …`：ios-debug.sh 的 native.log 里靠 `[事件] page <路由>` 判断进没进对页面。
final class MiniViewController: UIViewController {
    /// 入口页「诊断日志」开关（[StoreViewController.diagRow]）
    static let prefDiag = "diag"

    private let store: MiniStore
    private var mini: MiniProgramView!
    private var link: StoreLink?
    private let hint: MiniStore.App?
    private let demoRoute: String?
    /// 当前打开着的商店小程序（胶囊菜单的转发 / 收藏要用）；内置示例为 nil
    private var currentStoreApp: MiniStore.App?
    /// 当前页面路由（page 事件带的），转发链接带上它
    private var currentRoute: String?
    /// 当前打开着的小程序（诊断日志里带上）；内置示例是 {id: demo}
    private var currentApp: [String: Any] = ["id": "demo"]
    /// 每开一次商店小程序 +1：后台回来时对不上就丢掉（等下载的时候用户又点了别的链接）
    private var generation = 0
    private var opened = false
    /// 商店点开时的 loading 层（第一条 page 事件到了收起）
    private var cover: LoadingCover?

    /// 内置示例（[route]：起始页深链）
    init(route: String? = nil) {
        store = MiniStore()
        link = nil
        hint = nil
        demoRoute = route
        super.init(nibName: nil, bundle: nil)
    }

    /// 商店小程序：[hint] 是列表里的信息（先用它画 loading），[link] 带场景值 / 起始页
    init(link: StoreLink, hint: MiniStore.App? = nil, store: MiniStore = MiniStore()) {
        self.store = store
        self.link = link
        self.hint = hint
        demoRoute = nil
        super.init(nibName: nil, bundle: nil)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .white
        mini = newMiniView()
        view.addSubview(mini)
        // 屏幕左缘侧滑：二级页及以上交给引擎（跟手推出、不够阈值回弹，与微信 iOS 一样，host/edge_back.rs）；
        // 只有栈底那一页由这里接（收起本页，与 Android 的返回键同一条规则）。
        // 从前这个识别器每一页都接：它一认出手势，UIKit 就把交给引擎视图的触摸 cancel 掉，引擎的侧滑刚推出几个像素
        // 就收回去，之后页面不跟手、半路松手不回弹，划过 60pt 直接 goBack 跳转（「侧滑返回回弹没了」）
        let edge = UIScreenEdgePanGestureRecognizer(target: self, action: #selector(edgeBack(_:)))
        edge.edges = .left
        edge.delegate = self
        view.addGestureRecognizer(edge)
        if let link {
            openStore(link, hint: hint)
        } else {
            openDemo()
        }
    }

    /// 视频全屏期间收起状态栏与底部横条（与微信一致；引擎发 `videoFullscreen`）。全屏控件已经按安全区排开
    /// （引擎 video/fullscreen_chrome.rs），收起来画面更干净；横屏是引擎把那一层转过来画、界面还是竖的，
    /// 不收的话状态栏会竖着压在画面左边
    private var videoFullscreen = false {
        didSet {
            guard videoFullscreen != oldValue else { return }
            setNeedsStatusBarAppearanceUpdate()
            setNeedsUpdateOfHomeIndicatorAutoHidden()
        }
    }
    override var prefersStatusBarHidden: Bool { videoFullscreen }
    override var prefersHomeIndicatorAutoHidden: Bool { videoFullscreen }
    /// 小程序页只竖屏（横屏只给小游戏，由 MiniGameViewController 要）：不锁的话玩完横屏游戏回来，
    /// 系统看本页也支持横屏就不转回去（iOS 16 起只有当前方向不被支持时才自动转）
    override var supportedInterfaceOrientations: UIInterfaceOrientationMask { .portrait }

    /// 外部链接来了（SceneDelegate 收到 `minirender://`）：就地换小程序
    func handle(_ l: StoreLink) { openStore(l) }

    // MARK: - 视图与打开

    /// 一个配好宿主能力的视图（换小程序时要新建：一个视图只开一个小程序）
    private func newMiniView() -> MiniProgramView {
        // 入口页上的「诊断日志」开关：帧耗时归因、整帧原因、每秒帧率（引擎建实例时读一次进程环境）
        if UserDefaults.standard.bool(forKey: Self.prefDiag) {
            setenv("MINI_DAMAGE_LOG", "1", 1)
            setenv("MINI_FPS", "1", 1)
        } else {
            unsetenv("MINI_DAMAGE_LOG")
            unsetenv("MINI_FPS")
        }
        let v = MiniProgramView(frame: view.bounds)
        v.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        // 登录、支付、定位、扫码……要 App 自己接的 wx.*
        // 实例要留住：apis 里的闭包都是 [weak self]，只取 .apis 的话它当场就释放了，
        // 之后每个要弹框的接口都回「fail host gone」（跳转中心点了没反应就是这个）
        let hosts = DemoHostApis(presenter: self) { [weak self] args in self?.navigateToMiniProgram(args) }
        hostApiImpl = hosts
        v.hostApis = hosts.apis
        // 胶囊「•••」：微信式底部菜单（转发、收藏 / 重新进入、上传诊断日志、关于）
        v.onAbout = { [weak self] about in self?.showCapsuleMenu(about) }
        // 胶囊「⊙」：收起本页（从下往下退，与打开成对）
        v.onClose = { [weak self] in self?.close() }
        v.onEvent = { [weak self] ev in
            guard let self else { return }
            NSLog("[事件] %@", self.summarize(ev))
            // 第一页出来了才收起 loading（冷启动加载字体那几百毫秒也盖在底下，不露白屏）
            if ev["type"] as? String == "page" { self.pageDepth = (ev["depth"] as? NSNumber)?.intValue ?? self.pageDepth }
            if ev["type"] as? String == "videoFullscreen" { self.videoFullscreen = ev["fullScreen"] as? Bool ?? false }
            if ev["type"] as? String == "page", let c = self.cover {
                self.cover = nil
                c.hide()
            }
        }
        return v
    }

    private func openDemo() {
        do {
            mini.open(appDir: try DemoAssets.unpack(), route: demoRoute ?? UserDefaults.standard.string(forKey: "route"))
            opened = true
        } catch {
            NSLog("[示例] 内置示例打不开：%@", error.localizedDescription)
        }
    }

    /// 换一个新视图（旧的同步关掉：引擎的小程序根目录是进程级的，新引擎建之前旧的必须已经销毁）
    private func replaceView() {
        opened = false
        currentStoreApp = nil
        mini.close()
        mini.removeFromSuperview()
        mini = newMiniView()
        view.insertSubview(mini, at: 0)
    }

    /// 打开商店里的小程序（[l] 的 key 是 AppID 或 id）：盖 loading → 后台问服务器最新版、
    /// 不是最新就下载解包 → 回主线程 open。已经开着一个时先关掉它，再在新视图上开
    private func openStore(_ l: StoreLink, hint: MiniStore.App? = nil) {
        generation += 1
        let gen = generation
        link = l
        // 游戏页弹着时来了新链接：先同步关掉游戏（不连带收起本页）
        if let g = game {
            game = nil
            g.gameView.close()
            g.dismiss(animated: false) { [weak self] in
                if let self { MiniGameViewController.restoreOrientation(for: self) }
            }
        }
        if opened { replaceView() }
        cover?.removeFromSuperview()
        // 深链只带 AppID：装过的话先用本地记录的名字 / 图标画 loading（读一个小 JSON，不等网络）
        let known = hint ?? store.local(l.key)
        let c = LoadingCover(id: known?.id ?? l.key, name: known?.name ?? "小程序") { [weak self] in self?.close() }
        c.frame = view.bounds
        c.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        view.addSubview(c)
        cover = c
        let store = self.store
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            var stage = ""
            let r = Result {
                try store.prepare(l.key, status: { s in
                    stage = s
                    DispatchQueue.main.async { if gen == self?.generation { c.progress(s) } }
                }, progress: { got, total in
                    let mb = total > 0 ? String(format: "%.1f / %.1f MB", Double(got) / 1_048_576, Double(total) / 1_048_576)
                        : String(format: "%.1f MB", Double(got) / 1_048_576)
                    DispatchQueue.main.async { if gen == self?.generation { c.progress("\(stage)  \(mb)") } }
                })
            }
            DispatchQueue.main.async {
                guard let self, gen == self.generation else { return }
                switch r {
                case let .success(p):
                    let why = p.stale.map { "连不上服务器，用本地版（\($0.localizedDescription)）" } ?? (p.updated ? "已更新" : "已是最新")
                    NSLog("[商店] 打开 %@ %@ v%d %@ → %@ 起始页 %@", p.app.id, p.app.appId, p.app.version, why, p.dir.path, l.route ?? "首页")
                    self.currentStoreApp = p.app
                    var cur = p.app.json
                    cur["route"] = l.route ?? NSNull()
                    cur["stale"] = p.stale?.localizedDescription ?? NSNull()
                    self.currentApp = cur
                    c.setIdentity(id: p.app.id, name: p.app.name)
                    // 小游戏包（game.json）：开全屏的游戏页，loading 层跟过去盖到第一帧
                    if MiniGame.packageKind(p.dir.path) == 2 { return self.openGame(p, link: l, cover: c) }
                    self.mini.launchOptions = l.launchOptions
                    self.mini.open(appDir: p.dir, route: l.route)
                    self.opened = true
                case let .failure(e):
                    NSLog("[商店] %@ 打不开：%@", l.key, e.localizedDescription)
                    c.error("打不开：\(e.localizedDescription)") { [weak self] in self?.openStore(l, hint: hint) }
                }
            }
        }
    }

    /// 小游戏：全屏弹出 [MiniGameViewController]（转屏要由被弹出的控制器决定，放在导航栈里转不了）。
    /// 与 Android 示例的 GameActivity 一样：示例宿主接支付 / 激励视频 / 分享 / 登录，loading 盖到第一帧（10s 兜底），
    /// 游戏退出（wx.exitMiniProgram）连本页一起收起
    private func openGame(_ p: MiniStore.Prepared, link l: StoreLink, cover c: LoadingCover) {
        // 已经解好的包：后台编字节码（与马上建实例并行也安全，实例会等正在编的那段）
        MiniGame.precompile(dir: p.dir.path)
        let vc = MiniGameViewController(gameDir: p.dir.path, options: [
            // 存档按 appId 分：用商店的真 AppID，换了解包目录也不丢档
            "appId": p.app.key,
            "launchOptions": l.launchOptions,
            // 跑不满 60 时稳在 30（帧间隔均匀，比 40fps 上下跳更顺）
            "adaptiveFps": UserDefaults.standard.string(forKey: "adaptiveFps") != "0",   // `-adaptiveFps 0` 关（对照）
            "hostApis": GameHostApis.apis,
        ])
        let hosts = GameHostApis(presenter: vc) { [weak vc] () -> MiniGameHosting? in vc.flatMap { $0.gameView } }
        vc.onHostEvent = { [weak self, weak vc] type, ev in
            NSLog("[小游戏] %@ %@", p.app.id, type)
            // 胶囊「•••」（「⊙」控制器自己处理成退出）：与小程序同一个微信式底部菜单
            if type == "menuButton", let vc { self?.showGameMenu(vc, p.app); return }
            guard type == "hostCall", let v: MiniGameHosting = vc.flatMap({ $0.gameView }), !hosts.handle(ev) else { return }
            v.hostReply((ev["id"] as? NSNumber)?.intValue ?? 0, ok: false, result: ["errMsg": "\(ev["api"] ?? ""):fail 示例宿主未实现"])
        }
        vc.loadViewIfNeeded()
        cover = nil
        c.removeFromSuperview()
        let gc = LoadingCover(id: p.app.id, name: p.app.name) { [weak vc] in vc?.close() }
        gc.frame = vc.view.bounds
        gc.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        vc.view.addSubview(gc)
        var covered = true
        let reveal = { if covered { covered = false; gc.hide() } }
        // 验收：`-gameReopen <秒>` 第一帧之后这么久关掉游戏（同「⊙」），再在同一进程里打开同一个（查「关了还在响」「再进黑屏」）
        let reopen = UserDefaults.standard.double(forKey: "gameReopen")
        vc.onFirstFrame = { [weak self, weak vc] in
            reveal()
            guard reopen > 0, !Self.reopened else { return }
            Self.reopened = true
            DispatchQueue.main.asyncAfter(deadline: .now() + reopen) {
                guard vc != nil else { return }
                NSLog("[验收] 关掉游戏再进")
                self?.openStore(l) // 先同步关掉弹着的游戏（与外部链接换小程序同一条路），再开新的
            }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 10, execute: reveal)
        vc.onClose = { [weak self] in
            guard let self, self.game === vc else { return }
            self.game = nil
            self.close()
        }
        game = vc
        opened = true
        present(vc, animated: false)
    }

    /// 小游戏胶囊「•••」：顶上游戏名，转发 / 收藏、重新进入 / 关于（与 Android 示例 GameActivity.showGameMenu 同一份内容）
    private func showGameMenu(_ vc: MiniGameViewController, _ app: MiniStore.App) {
        let local = StoreLocal()
        let fav = local.isFavorite(app.id)
        let about = {
            let a = UIAlertController(title: app.name, message: "小游戏 · \(app.appId)\nv\(app.version)，由 mini-render 引擎运行", preferredStyle: .alert)
            a.addAction(UIAlertAction(title: "确定", style: .cancel))
            vc.present(a, animated: true)
        }
        StoreSheets.capsuleMenu(
            vc, icon: IconView(store: store, app: app, size: 30), name: app.name, subtitle: "小游戏 · v\(app.version)",
            onHeader: about,
            top: [
                StoreSheets.Item(glyph: "↗", label: "转发给朋友") { StoreSheets.copyLink(app, route: nil) },
                StoreSheets.Item(glyph: fav ? "★" : "☆", label: fav ? "取消收藏" : "添加到\n我的小程序", active: fav) {
                    let now = local.toggleFavorite(app)
                    StoreSheets.toast(now ? "已添加到「我的小程序」，入口页下拉可见" : "已从「我的小程序」移除")
                },
            ],
            bottom: [
                StoreSheets.Item(glyph: "↻", label: "重新进入\n小游戏") { [weak vc] in vc?.restart() },
                StoreSheets.Item(glyph: "ⓘ", label: "关于", onTap: about),
            ])
    }

    /// 当前视图的宿主 API 实现（见 newMiniView：不留着就被释放）
    private var hostApiImpl: DemoHostApis?

    /// `-gameReopen` 只做一次
    private static var reopened = false

    /// 弹着的游戏页（外部链接换小程序时先关掉它）
    private var game: MiniGameViewController?

    /// 胶囊菜单「重新进入小程序」：商店的走 [openStore]（先关后开）；内置示例同样换一个新视图再开
    private func relaunch() {
        if let link { return openStore(link, hint: currentStoreApp) }
        replaceView()
        openDemo()
    }

    /// 小程序里的 `wx.navigateToMiniProgram`（用户已点允许）：按 AppID 换过去，scene 1037 + referrerInfo。
    /// 与微信的差别：微信把目标小程序叠在上面、`navigateBackMiniProgram` 能回来；这里是**替换**（单实例）
    private func navigateToMiniProgram(_ args: [String: Any]) {
        var c = URLComponents()
        c.queryItems = [URLQueryItem(name: "appid", value: args["appId"] as? String ?? ""),
                        URLQueryItem(name: "path", value: args["path"] as? String ?? "")]
        guard var l = StoreLink.parse("minirender://open?" + (c.percentEncodedQuery ?? "")) else { return }
        var referrer: [String: Any] = ["appId": currentStoreApp?.appId ?? ""]
        if let extra = args["extraData"], !(extra is NSNull) { referrer["extraData"] = extra }
        l.scene = 1037
        l.referrer = referrer
        openStore(l)
    }

    private func uploadDiag() {
        let backend = "\(mini.activeBackend == .gpu ? "gpu" : "cpu")：\(mini.backendReason)"
        DiagUpload.ask(self, baseUrl: store.baseUrl, app: currentApp, backend: backend)
    }

    private func close() {
        mini.close()
        if let nav = navigationController, nav.viewControllers.first !== self {
            nav.popViewController(animated: true)
        } else {
            dismiss(animated: true)
        }
    }

    @objc private func edgeBack(_ g: UIScreenEdgePanGestureRecognizer) {
        guard g.state == .ended, g.translation(in: view).x > 60 else { return }
        if !mini.goBack() { close() }
    }

    /// 当前页面栈深度（引擎的 page 事件带 depth）：栈底（1）之上的侧滑归引擎
    private var pageDepth = 1

    // MARK: - 胶囊菜单（MenuSheet.kt）

    /// 胶囊「•••」的底部菜单。SDK 只留口子，展示什么由宿主定；示例照微信：
    /// 顶上小程序名（点了看关于）、转发 / 收藏、重新进入 / 上传诊断日志 / 关于。内置示例没有商店链接：转发、收藏灰掉
    private func showCapsuleMenu(_ about: MiniProgramAbout) {
        let app = currentStoreApp
        let local = StoreLocal()
        let fav = app.map { local.isFavorite($0.id) } ?? false
        let name = app?.name ?? (about.name.isEmpty ? "内置示例" : about.name)
        let icon: UIView = app.map { IconView(store: store, app: $0, size: 30) } ?? IconView(id: "demo", name: name, size: 30)
        StoreSheets.capsuleMenu(
            self, icon: icon, name: name,
            subtitle: app.map { "v\($0.version)" } ?? "本地小程序",
            onHeader: { [weak self] in self?.showAboutDialog(about) },
            top: [
                StoreSheets.Item(glyph: "↗", label: "转发给朋友", enabled: app != nil) { [weak self] in
                    if let app { StoreSheets.copyLink(app, route: self?.currentRoute) }
                },
                StoreSheets.Item(glyph: fav ? "★" : "☆", label: fav ? "取消收藏" : "添加到\n我的小程序", enabled: app != nil, active: fav) {
                    guard let app else { return }
                    let now = local.toggleFavorite(app)
                    StoreSheets.toast(now ? "已添加到「我的小程序」，入口页下拉可见" : "已从「我的小程序」移除")
                },
            ],
            bottom: [
                StoreSheets.Item(glyph: "↻", label: "重新进入\n小程序") { [weak self] in self?.relaunch() },
                StoreSheets.Item(glyph: "⚑", label: "上传\n诊断日志") { [weak self] in self?.uploadDiag() },
                StoreSheets.Item(glyph: "ⓘ", label: "关于") { [weak self] in self?.showAboutDialog(about) },
            ])
    }

    private func showAboutDialog(_ a: MiniProgramAbout) {
        let alert = UIAlertController(title: a.dialogTitle, message: a.dialogMessage, preferredStyle: .alert)
        alert.addAction(UIAlertAction(title: "上传诊断日志", style: .default) { [weak self] _ in self?.uploadDiag() })
        alert.addAction(UIAlertAction(title: "确定", style: .cancel))
        present(alert, animated: true)
    }

    // MARK: - 日志

    private func summarize(_ e: [String: Any]) -> String {
        let type = e["type"] as? String ?? "?"
        if type == "page" {
            let title = (e["window"] as? [String: Any])?["navigationBarTitleText"] as? String ?? ""
            currentRoute = (e["route"] as? String).flatMap { $0.isEmpty ? nil : $0 }
            return "page \(e["route"] as? String ?? "") depth=\((e["depth"] as? NSNumber)?.intValue ?? 0) title=\(title)"
        }
        var copy = e
        copy.removeValue(forKey: "type")
        let json = (try? JSONSerialization.data(withJSONObject: copy)).map { String(decoding: $0, as: UTF8.self) } ?? "\(copy)"
        return "\(type) \(json)"
    }
}

extension MiniViewController: UIGestureRecognizerDelegate {
    /// 左缘侧滑只在栈底那一页由宿主接（收起小程序）；更深的页交给引擎自己的侧滑返回（见 viewDidLoad）
    func gestureRecognizerShouldBegin(_ g: UIGestureRecognizer) -> Bool {
        guard g is UIScreenEdgePanGestureRecognizer else { return true }
        return pageDepth <= 1
    }
}
