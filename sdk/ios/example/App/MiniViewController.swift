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
        // 屏幕左缘侧滑 = 小程序返回；页面栈见底了才收起本页（与 Android 的返回键同一条规则）
        let edge = UIScreenEdgePanGestureRecognizer(target: self, action: #selector(edgeBack(_:)))
        edge.edges = .left
        view.addGestureRecognizer(edge)
        if let link {
            openStore(link, hint: hint)
        } else {
            openDemo()
        }
    }

    override var prefersHomeIndicatorAutoHidden: Bool { false }

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
        v.hostApis = DemoHostApis(presenter: self) { [weak self] args in self?.navigateToMiniProgram(args) }.apis
        // 胶囊「•••」：微信式底部菜单（转发、收藏 / 重新进入、上传诊断日志、关于）
        v.onAbout = { [weak self] about in self?.showCapsuleMenu(about) }
        // 胶囊「⊙」：收起本页（从下往下退，与打开成对）
        v.onClose = { [weak self] in self?.close() }
        v.onEvent = { [weak self] ev in
            guard let self else { return }
            NSLog("[事件] %@", self.summarize(ev))
            // 第一页出来了才收起 loading（冷启动加载字体那几百毫秒也盖在底下，不露白屏）
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
