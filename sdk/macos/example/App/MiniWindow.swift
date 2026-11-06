import AppKit
import MiniRender

/// 小程序窗口（PC 微信：小程序在一个手机大小的独立窗口里开，红绿灯压在小程序自己的导航栏那一行上）。
///
/// 与 Android 示例 MainActivity / iOS 示例 MiniViewController 一样的流程：盖 loading → 后台问商店最新版、
/// 不是最新就下载解包 → 打开；内置示例直接打开包里的 demo。胶囊「•••」弹 PC 微信式菜单
/// （转发 = 复制链接、收藏、重新进入、上传诊断日志、关于），「⊙」关窗口。
///
/// 一个进程一个小程序窗口（多开靠多进程，见 MiniProcess.swift）。窗口拉宽到 600pt 以上自动分栏
/// （PC 微信的 frameset：上一页在左栏、当前页在右栏），窄回去退回单栏。
///
/// 验收参数（子进程）：`-miniWidth <pt>` 起来 4 秒后把窗口拉到这个宽、`-miniNav <url>` 6 秒后跳一页、
/// `-miniTap x,y[;x,y…]` 7.5 秒起每 1.5 秒点一下、`-miniShot <目录>` 9 秒后把两栏拼成一张 `split.png`（不要屏幕录制权限）。
final class MiniWindowController: NSWindowController, NSWindowDelegate {
    static private(set) var shared: MiniWindowController?

    /// 打开（或在已开着的窗口里换成）一个小程序。`link` 为 nil = 内置示例
    static func open(link: StoreLink?, hint: MiniStore.App? = nil, store: MiniStore) {
        let c = shared ?? MiniWindowController(store: store)
        shared = c
        c.showWindow(nil)
        c.window?.makeKeyAndOrderFront(nil)
        c.load(link: link, hint: hint)
    }

    let store: MiniStore
    var mini: MiniProgramView?
    /// 小游戏包（game.json）开的是这个，不是 MiniProgramView（GameWindow.swift）
    var game: MiniGameView?
    private(set) var link: StoreLink?
    /// 小程序打开小程序的来路（A → B → C 时是 [A, B]）：胶囊「⊙」/ navigateBackMiniProgram / exitMiniProgram 先回上一个，
    /// 最后才关窗口。单实例：回去是按来路的链接重新打开（scene 1038，referrerInfo 带回 extraData）。
    /// shot 是离开它那一刻的画面：回去时不盖 loading、不问服务器，先盖着它，第一页出来再淡掉
    private var backStack: [(link: StoreLink, shot: NSImage?)] = []
    var currentApp: MiniStore.App?
    private var currentRoute: String?
    /// 当前视图的宿主 API 实现（见 newView：不留着就被释放）
    private var hostApiImpl: DemoHostApis?
    private var generation = 0
    private var cover: NSView?
    private var coverStatus: NSTextField?
    /// 红绿灯那一行的高度：交给引擎当状态栏
    let titleBarH: CGFloat = 28

    private init(store: MiniStore) {
        self.store = store
        let w = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 390, height: 760),
                         styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
                         backing: .buffered, defer: false)
        w.titlebarAppearsTransparent = true
        w.titleVisibility = .hidden
        w.isMovableByWindowBackground = false
        w.minSize = NSSize(width: 320, height: 520)
        w.contentView = FlippedView()
        w.contentView?.wantsLayer = true
        w.contentView?.layer?.backgroundColor = NSColor.white.cgColor
        super.init(window: w)
        w.delegate = self
        // 居中打开；同时开着几个时按「第几个开的」往右下错开 30pt，不完全叠在一起。
        // 从前贴着屏幕右边开（maxX - 440），拉宽到分栏时窗口有一半出了屏幕
        w.isRestorable = false
        w.center()
        let n = CGFloat(UserDefaults.standard.integer(forKey: "cascade") % 8)
        w.setFrameOrigin(NSPoint(x: w.frame.minX + n * 30, y: w.frame.minY - n * 30))
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    func windowWillClose(_ notification: Notification) {
        game?.close()
        game = nil
        mini?.close()
        mini?.removeFromSuperview()
        mini = nil
        Self.shared = nil
        // 子进程（一个小程序一个进程，MiniProcess.swift）：窗口关了进程就退
        if UserDefaults.standard.bool(forKey: "miniHost") { NSApp.terminate(nil) }
    }

    // MARK: - 打开

    private func newView() -> MiniProgramView {
        closeGame()
        mini?.close()
        mini?.removeFromSuperview()
        let diag = UserDefaults.standard.bool(forKey: "diag")
        diag ? setenv("MINI_DAMAGE_LOG", "1", 1) : unsetenv("MINI_DAMAGE_LOG")
        diag ? setenv("MINI_FPS", "1", 1) : unsetenv("MINI_FPS")
        let v = MiniProgramView(frame: window!.contentView!.bounds)
        v.autoresizingMask = [.width, .height]
        v.topInset = titleBarH
        // 实例要留住：apis 里的闭包是 [weak self]，只取 .apis 的话它当场就释放了（iOS 示例同一个坑）
        let hosts = DemoHostApis(window: window) { [weak self] args in self?.navigateToMiniProgram(args) }
        hosts.onNavigateBackMiniProgram = { [weak self] extra in self?.backToReferrer(extra) ?? false }
        hosts.canNavigateBack = { [weak self] in !(self?.backStack.isEmpty ?? true) }
        hostApiImpl = hosts
        v.hostApis = hosts.apis
        v.onAbout = { [weak self] about in self?.showMenu(about) }
        v.onClose = { [weak self] in self?.leave() }
        v.onEvent = { [weak self] ev in
            guard let self else { return }
            NSLog("[事件] %@", Self.summarize(ev))
            // wx.exitMiniProgram：与胶囊「⊙」同一条路
            if ev["type"] as? String == "exitMiniProgram" { DispatchQueue.main.async { self.leave() } }
            if ev["type"] as? String == "page" {
                self.currentRoute = ev["route"] as? String
                let title = (ev["window"] as? [String: Any])?["navigationBarTitleText"] as? String
                self.window?.title = title ?? self.currentApp?.name ?? "小程序"
                self.hideCover()
            }
        }
        window!.contentView!.addSubview(v, positioned: .below, relativeTo: nil)
        mini = v
        scheduleAcceptance()
        return v
    }

    /// `-miniWidth` / `-miniNav` / `-miniShot`（见类注释）
    private func scheduleAcceptance() {
        let d = UserDefaults.standard
        let width = d.double(forKey: "miniWidth")
        if width > 0 {
            DispatchQueue.main.asyncAfter(deadline: .now() + 4) { [weak self] in
                guard let w = self?.window else { return }
                var f = w.frame
                f.origin.x -= (width - f.width) / 2 // 向两边拉开，窗口仍在原来的中线上
                f.size.width = width
                w.setFrame(w.constrainFrameRect(f, to: w.screen), display: true)
                NSLog("[分栏] 窗口拉到 %.0fpt", width)
            }
        }
        if let url = d.string(forKey: "miniNav"), !url.isEmpty {
            DispatchQueue.main.asyncAfter(deadline: .now() + 6) { [weak self] in self?.mini?.navigate(url) }
        }
        // `-miniTap x,y`（视图坐标，pt）：7.5 秒时在那里点一下（不要辅助功能权限：事件直接交给视图）
        // 多个点用分号隔开（`200,670;200,760`），每隔 1.5 秒点下一个
        for (i, one) in (d.string(forKey: "miniTap") ?? "").split(separator: ";").enumerated() {
            let xy = one.split(separator: ",").compactMap { Double($0) }
            guard xy.count == 2 else { continue }
            DispatchQueue.main.asyncAfter(deadline: .now() + 7.5 + Double(i) * 1.5) { [weak self] in self?.click(CGPoint(x: xy[0], y: xy[1])) }
        }
        if let dir = d.string(forKey: "miniShot"), !dir.isEmpty {
            DispatchQueue.main.asyncAfter(deadline: .now() + 9) { [weak self] in self?.saveSplitShot(to: URL(fileURLWithPath: dir)) }
        }
    }

    private func click(_ p: CGPoint) {
        guard let mini, let w = window else { return }
        let inWindow = mini.convert(p, to: nil)
        let make = { (type: NSEvent.EventType) in
            NSEvent.mouseEvent(with: type, location: inWindow, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                               windowNumber: w.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: 1)
        }
        if let e = make(.leftMouseDown) { mini.mouseDown(with: e) }
        // 真手指按下到抬起约 90ms（用户上传的诊断里每次点击两组日志隔 ~90ms）：中间那几帧也要上屏
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.09) { if let e = make(.leftMouseUp) { mini.mouseUp(with: e) } }
        NSLog("[分栏] 点了 (%.0f, %.0f)", p.x, p.y)
    }

    /// 两栏拼一张：活页面（引擎快照）摆在 liveX，另一栏（引擎给的图）摆在另一边
    private func saveSplitShot(to dir: URL) {
        guard let mini else { return }
        let layout = mini.splitLayout
        let side = mini.snapshotSide()
        let size = mini.bounds.size
        mini.snapshot { live in
            guard let live else { return }
            let img = NSImage(size: size)
            img.lockFocusFlipped(true)
            NSColor.white.setFill()
            NSRect(origin: .zero, size: size).fill()
            let paneW = layout.on ? layout.pane : size.width
            live.draw(in: NSRect(x: layout.liveX, y: 0, width: paneW, height: size.height))
            if let side {
                side.draw(in: NSRect(x: layout.liveX > 0 ? 0 : layout.pane, y: 0, width: layout.pane, height: size.height))
            }
            img.unlockFocus()
            try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            if let tiff = img.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff) {
                try? rep.representation(using: .png, properties: [:])?.write(to: dir.appendingPathComponent("split.png"))
            }
            NSLog("[分栏] 截图 on=%d liveX=%.0f pane=%.0f side=%d → %@", layout.on ? 1 : 0, layout.liveX, layout.pane, side == nil ? 0 : 1, dir.path)
        }
    }

    /// [resume]：回到上一个小程序（[backToReferrer]）—— 用本地那一版不问服务器，盖 [shot] 而不是 loading
    func load(link l: StoreLink?, hint: MiniStore.App?, resume: Bool = false, shot: NSImage? = nil) {
        generation += 1
        let gen = generation
        link = l
        currentApp = nil
        let v = newView()
        guard let l else {
            window?.title = "内置示例"
            if let dir = try? DemoAssets.unpack() { v.open(appDir: dir) }
            return
        }
        let known = hint ?? store.local(l.key)
        if resume {
            showResumeCover(shot)
        } else {
            showCover(id: known?.id ?? l.key, name: known?.name ?? "小程序")
        }
        let store = self.store
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            var stage = ""
            let onStatus: (String) -> Void = { s in
                stage = s
                DispatchQueue.main.async { if gen == self?.generation { self?.coverStatus?.stringValue = s } }
            }
            let onProgress: (Int64, Int64) -> Void = { got, total in
                let mb = total > 0 ? String(format: "%.1f / %.1f MB", Double(got) / 1_048_576, Double(total) / 1_048_576) : ""
                DispatchQueue.main.async { if gen == self?.generation { self?.coverStatus?.stringValue = "\(stage)  \(mb)" } }
            }
            let r = Result {
                resume ? try store.prepareLocal(l.key, status: onStatus, progress: onProgress)
                    : try store.prepare(l.key, status: onStatus, progress: onProgress)
            }
            DispatchQueue.main.async {
                guard let self, gen == self.generation else { return }
                switch r {
                case let .success(p):
                    NSLog("[商店] 打开 %@ %@ v%d → %@ 起始页 %@", p.app.id, p.app.appId, p.app.version, p.dir.path, l.route ?? "首页")
                    self.currentApp = p.app
                    StoreLocal().touch(p.app)
                    // 小游戏包：换成 MiniGameView（GameWindow.swift），loading 层盖到第一帧
                    if MiniGame.packageKind(p.dir.path) == 2 { return self.openGame(p, link: l) }
                    v.launchOptions = l.launchOptions
                    v.open(appDir: p.dir, route: l.route)
                case let .failure(e):
                    // 回来时打不开（本地包被清了又连不上服务器）：换回普通 loading 层，把原因写上
                    if resume { self.showCover(id: known?.id ?? l.key, name: known?.name ?? "小程序") }
                    self.coverStatus?.textColor = NSColor(argb: 0xFFE64340)
                    self.coverStatus?.stringValue = "打不开：\(e.localizedDescription)"
                }
            }
        }
    }

    /// loading 层：图标 + 名字 + 进度（与手机端示例同一个样子）
    private func showCover(id: String, name: String) {
        cover?.removeFromSuperview()
        let c = FlippedView(frame: window!.contentView!.bounds)
        c.autoresizingMask = [.width, .height]
        c.wantsLayer = true
        c.layer?.backgroundColor = NSColor.white.cgColor
        let icon = IconView(id: id, name: name, size: 64)
        let title = label(name, 17)
        let status = label("检查更新…", 12, NSColor(argb: 0x61000000))
        let col = NSStackView(views: [icon, title, status])
        col.orientation = .vertical
        col.spacing = 14
        c.addSubview(col)
        col.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([col.centerXAnchor.constraint(equalTo: c.centerXAnchor),
                                     col.centerYAnchor.constraint(equalTo: c.centerYAnchor, constant: -60)])
        window!.contentView!.addSubview(c)
        cover = c
        coverStatus = status
    }

    /// 回到上一个小程序时盖的那层：离开它时的画面铺满内容区（= MiniProgramView 的位置；没截到就是白底）。
    /// 没有图标、名字、进度 —— 返回不该看起来像「又打开了一次」
    private func showResumeCover(_ shot: NSImage?) {
        cover?.removeFromSuperview()
        let c = FlippedView(frame: window!.contentView!.bounds)
        c.autoresizingMask = [.width, .height]
        c.wantsLayer = true
        c.layer?.backgroundColor = NSColor.white.cgColor
        if let shot {
            // 截图是引擎像素（视图尺寸 × backingScale）：拉到视图尺寸画，正好 1:1
            let iv = NSImageView(frame: c.bounds)
            iv.autoresizingMask = [.width, .height]
            iv.imageScaling = .scaleAxesIndependently
            iv.image = shot
            c.addSubview(iv)
        }
        window!.contentView!.addSubview(c)
        cover = c
        coverStatus = nil
    }

    func hideCover() {
        guard let c = cover else { return }
        cover = nil
        NSAnimationContext.runAnimationGroup({ ctx in
            ctx.duration = 0.2
            c.animator().alphaValue = 0
        }, completionHandler: { c.removeFromSuperview() })
    }

    func relaunch() { load(link: link, hint: currentApp) }

    /// 引擎当前这一帧（截图验收；画面在 CAMetalLayer 上，窗口截图截不到）
    func snapshot(_ done: @escaping (NSImage?) -> Void) {
        if let game { return game.snapshot(done) }
        guard let mini else { return done(nil) }
        mini.snapshot(done)
    }

    /// `wx.navigateToMiniProgram`（用户已点允许）：在这个窗口里换过去，scene 1037 + referrerInfo
    private func navigateToMiniProgram(_ args: [String: Any]) {
        var c = URLComponents()
        c.queryItems = [URLQueryItem(name: "appid", value: args["appId"] as? String ?? ""),
                        URLQueryItem(name: "path", value: args["path"] as? String ?? "")]
        guard var l = StoreLink.parse("minirender://open?" + (c.percentEncodedQuery ?? "")) else { return }
        var referrer: [String: Any] = ["appId": currentApp?.appId ?? ""]
        if let extra = args["extraData"], !(extra is NSNull) { referrer["extraData"] = extra }
        l.scene = 1037
        l.referrer = referrer
        // 记下来路：B 里返回要回到这一个（A），连同 A 此刻的画面（要在关掉 A 之前截）
        let from = link
        let gen = generation
        snapshot { [weak self] shot in
            // 截图的这几毫秒里又换了别的：这次跳转作废
            guard let self, gen == self.generation else { return }
            if let from { self.backStack.append((from, shot)) }
            self.load(link: l, hint: nil)
        }
    }

    /// 回到打开当前小程序的那个小程序（scene 1038，referrerInfo 带 [extra]）。没有来路返回 false。
    /// 不走 loading：用本地那一版（不问服务器），先盖离开它时截的画面，它的第一页出来再淡掉
    @discardableResult
    private func backToReferrer(_ extra: Any?) -> Bool {
        guard let entry = backStack.popLast() else { return false }
        var a = entry.link
        var referrer: [String: Any] = ["appId": currentApp?.appId ?? ""]
        if let extra { referrer["extraData"] = extra }
        a.scene = 1038
        a.referrer = referrer
        load(link: a, hint: nil, resume: true, shot: entry.shot)
        return true
    }

    /// 离开当前小程序：有来路回上一个，否则关窗口
    private func leave() {
        if !backToReferrer(nil) { close() }
    }

    // MARK: - 胶囊菜单

    private func showMenu(_ about: MiniProgramAbout) {
        let m = NSMenu()
        // 灰掉的项（内置示例没有商店链接：转发、收藏）按 isEnabled 来，不让 AppKit 按 target 自动判断
        m.autoenablesItems = false
        let app = currentApp
        let local = StoreLocal()
        let fav = app.map { local.isFavorite($0.id) } ?? false
        m.addItem(withTitle: app?.name ?? (about.name.isEmpty ? "内置示例" : about.name), action: nil, keyEquivalent: "").isEnabled = false
        m.addItem(.separator())
        m.addItem(MenuAction("转发给朋友（复制链接）", enabled: app != nil) { [weak self] in
            guard let app else { return }
            var c = URLComponents()
            c.queryItems = [URLQueryItem(name: "appid", value: app.key)]
            if let r = self?.currentRoute, !r.isEmpty { c.queryItems?.append(URLQueryItem(name: "path", value: r)) }
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString("minirender://open?" + (c.percentEncodedQuery ?? ""), forType: .string)
        })
        m.addItem(MenuAction(fav ? "取消收藏" : "添加到「我的小程序」", enabled: app != nil) {
            if let app { local.toggleFavorite(app) }
        })
        m.addItem(.separator())
        m.addItem(MenuAction("重新进入小程序") { [weak self] in self?.relaunch() })
        m.addItem(MenuAction("上传诊断日志") { [weak self] in self?.uploadDiag() })
        m.addItem(MenuAction("关于\(about.name)") { [weak self] in
            let a = NSAlert()
            a.messageText = about.dialogTitle
            a.informativeText = about.dialogMessage
            if let w = self?.window { a.beginSheetModal(for: w) }
        })
        // 菜单出在胶囊下面（右上角）
        guard let v = window?.contentView else { return }
        m.popUp(positioning: nil, at: NSPoint(x: v.bounds.width - 96, y: titleBarH + 40), in: v)
    }

    private func uploadDiag() {
        guard let mini, let w = window else { return }
        let backend = "\(mini.activeBackend == .gpu ? "gpu" : "cpu")：\(mini.backendReason)"
        var app: [String: Any] = currentApp?.json ?? ["id": "demo"]
        app["route"] = currentRoute ?? NSNull()
        DiagUpload.ask(w, baseUrl: store.baseUrl, app: app, backend: backend)
    }

    private static func summarize(_ e: [String: Any]) -> String {
        let type = e["type"] as? String ?? "?"
        if type == "page" {
            return "page \(e["route"] as? String ?? "") depth=\((e["depth"] as? NSNumber)?.intValue ?? 0)"
        }
        var copy = e
        copy.removeValue(forKey: "type")
        let json = (try? JSONSerialization.data(withJSONObject: copy)).map { String(decoding: $0, as: UTF8.self) } ?? ""
        return "\(type) \(json)"
    }
}

/// 闭包版的菜单项
final class MenuAction: NSMenuItem {
    private let block: () -> Void

    init(_ title: String, enabled: Bool = true, _ block: @escaping () -> Void) {
        self.block = block
        super.init(title: title, action: #selector(fire), keyEquivalent: "")
        target = self
        isEnabled = enabled
    }

    required init(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    @objc private func fire() { block() }
}
