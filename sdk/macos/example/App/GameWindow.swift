import AppKit
import MiniRender

/// 小游戏窗口（MiniWindowController 打开的包是 game.json 时走这里；与 Android 的 GameActivity、iOS 的 openGame 同一套）：
/// 窗口按 game.json 的方向开（横屏游戏开成横的窗口，桌面不转屏）、红绿灯那一行报成安全区内缩、
/// 右上角原生胶囊「⊙」关窗口、「•••」弹 PC 微信式菜单；示例宿主接支付 / 激励视频 / 分享 / 登录（演示实现）。
extension MiniWindowController {
    func openGame(_ p: MiniStore.Prepared, link l: StoreLink) {
        mini?.close()
        mini?.removeFromSuperview()
        mini = nil
        closeGame()
        // 已经解好的包：后台编字节码（与马上建实例并行也安全）
        MiniGame.precompile(dir: p.dir.path)
        let g = MiniGameView(gameDir: p.dir.path, options: [
            // 存档按 appId 分：用商店的真 AppID
            "appId": p.app.key,
            "launchOptions": l.launchOptions,
            "adaptiveFps": true,
            "hostApis": GameHostApis.apis,
        ])
        fitWindow(orientation: g.gameOrientation)
        guard let content = window?.contentView else { return }
        g.frame = content.bounds
        g.autoresizingMask = [.width, .height]
        g.topInset = titleBarH
        let hosts = GameHostApis(window: window) { [weak g] in g }
        g.onFirstFrame = { [weak self] in self?.hideCover() }
        g.onOrientationChange = { [weak self] o in self?.fitWindow(orientation: o) }
        g.onHostEvent = { [weak self, weak g] type, ev in
            guard let self, let g else { return }
            switch type {
            case "exit": self.close()
            case "restart": self.relaunch()
            case "menuButton": ev["action"] as? String == "close" ? self.close() : self.showGameMenu(p.app)
            case "hostCall":
                if !hosts.handle(ev) {
                    g.hostReply((ev["id"] as? NSNumber)?.intValue ?? 0, ok: false, result: ["errMsg": "\(ev["api"] ?? ""):fail 示例宿主未实现"])
                }
            default: NSLog("[小游戏] %@ %@", p.app.id, type)
            }
        }
        content.addSubview(g, positioned: .below, relativeTo: nil)
        game = g
        window?.title = p.app.name
        window?.makeFirstResponder(g)
        // 兜底：游戏一直不出帧（脚本报错、卡在加载）也别让 loading 永远盖着
        DispatchQueue.main.asyncAfter(deadline: .now() + 10) { [weak self, weak g] in
            if g != nil, self?.game === g { self?.hideCover() }
        }
    }

    func closeGame() {
        game?.close()
        game?.removeFromSuperview()
        game = nil
    }

    /// 横屏游戏开成横的窗口（手机横屏的比例，844×390 + 红绿灯那一行），竖屏的回到手机大小；窗口中心不动
    func fitWindow(orientation o: String) {
        guard let w = window else { return }
        let landscape = o.hasPrefix("landscape")
        let size = landscape ? NSSize(width: 844, height: 390 + titleBarH) : NSSize(width: 390, height: 760)
        w.minSize = landscape ? NSSize(width: 520, height: 260) : NSSize(width: 320, height: 520)
        var f = w.frameRect(forContentRect: NSRect(origin: .zero, size: size))
        f.origin = NSPoint(x: w.frame.midX - f.width / 2, y: w.frame.midY - f.height / 2)
        w.setFrame(w.constrainFrameRect(f, to: w.screen), display: true, animate: false)
    }

    /// 胶囊「•••」：PC 微信式菜单（转发 = 复制链接、收藏、重新进入、关于），出在胶囊下面
    func showGameMenu(_ app: MiniStore.App) {
        let m = NSMenu()
        m.autoenablesItems = false
        let local = StoreLocal()
        let fav = local.isFavorite(app.id)
        m.addItem(withTitle: app.name, action: nil, keyEquivalent: "").isEnabled = false
        m.addItem(.separator())
        m.addItem(MenuAction("转发给朋友（复制链接）") {
            var c = URLComponents()
            c.queryItems = [URLQueryItem(name: "appid", value: app.key)]
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString("minirender://open?" + (c.percentEncodedQuery ?? ""), forType: .string)
        })
        m.addItem(MenuAction(fav ? "取消收藏" : "添加到「我的小程序」") { local.toggleFavorite(app) })
        m.addItem(.separator())
        m.addItem(MenuAction("重新进入小游戏") { [weak self] in self?.relaunch() })
        m.addItem(MenuAction("关于\(app.name)") { [weak self] in
            let a = NSAlert()
            a.messageText = app.name
            a.informativeText = "小游戏 · \(app.appId)\nv\(app.version)，由 mini-render 引擎运行"
            if let w = self?.window { a.beginSheetModal(for: w) }
        })
        guard let v = window?.contentView else { return }
        m.popUp(positioning: nil, at: NSPoint(x: v.bounds.width - 96, y: titleBarH + 40), in: v)
    }
}

/// 示例 App 给小游戏接的「宿主能力」（与 Android 的 GameHostApis.kt、iOS 的 GameHostApis.swift 同一组、同样的演示实现）：
/// 支付弹确认框（不扣钱）、激励视频是一个「看完了吗」的对话框、分享复制链接、登录给一个假 code。
final class GameHostApis {
    static let apis = ["requestMidasPayment", "checkIsSupportMidasPayment", "shareAppMessage", "login",
                       "RewardedVideoAd.create", "RewardedVideoAd.load", "RewardedVideoAd.show", "RewardedVideoAd.destroy"]
    private weak var window: NSWindow?
    private let view: () -> MiniGameView?

    init(window: NSWindow?, view: @escaping () -> MiniGameView?) {
        self.window = window
        self.view = view
    }

    /// 不认识的返回 false
    func handle(_ call: [String: Any]) -> Bool {
        let id = (call["id"] as? NSNumber)?.intValue ?? 0
        let args = call["args"] as? [String: Any] ?? [:]
        let ctx = args["contextId"].map { "\($0)" } ?? ""
        switch call["api"] as? String ?? "" {
        case "checkIsSupportMidasPayment": ok(id, ["isSupport": true])
        case "login": ok(id, ["code": "demo_game_" + String(Int(Date().timeIntervalSince1970 * 1000), radix: 36)])
        case "shareAppMessage": ok(id)
        case "RewardedVideoAd.create", "RewardedVideoAd.load":
            ok(id)
            adEvent(ctx, "load")
        case "RewardedVideoAd.destroy": ok(id)
        case "RewardedVideoAd.show":
            ok(id)
            ask("（演示）激励视频广告", "示例 App 没有接广告 SDK。当作看完了吗？", yes: "看完了，领奖励", no: "跳过") { [weak self] done in
                self?.adEvent(ctx, "close", ["isEnded": done])
            }
        case "requestMidasPayment":
            let yuan = Double((args["buyQuantity"] as? NSNumber)?.intValue ?? 0) / 10
            ask("模拟支付", String(format: "示例 App 没有接入米大师。\n模拟支付 ¥%.2f？（不会真的扣款）", yuan), yes: "支付", no: "取消") { [weak self] paid in
                if paid { self?.ok(id, ["errMsg": "requestMidasPayment:ok"]) } else {
                    self?.view()?.hostReply(id, ok: false, result: ["errMsg": "requestMidasPayment:fail cancel", "errCode": 1])
                }
            }
        default: return false
        }
        return true
    }

    private func ok(_ id: Int, _ res: [String: Any] = [:]) { view()?.hostReply(id, ok: true, result: res) }

    private func adEvent(_ ctx: String, _ event: String, _ data: [String: Any] = [:]) {
        let o: [String: Any] = ["context": "RewardedVideoAd", "contextId": ctx, "event": event, "data": data]
        let json = (try? JSONSerialization.data(withJSONObject: o)).map { String(decoding: $0, as: UTF8.self) } ?? "{}"
        view()?.dispatchEvent("contextEvent", json: json)
    }

    private func ask(_ title: String, _ text: String, yes: String, no: String, _ done: @escaping (Bool) -> Void) {
        let a = NSAlert()
        a.messageText = title
        a.informativeText = text
        a.addButton(withTitle: yes)
        a.addButton(withTitle: no)
        guard let w = window else { return done(a.runModal() == .alertFirstButtonReturn) }
        a.beginSheetModal(for: w) { done($0 == .alertFirstButtonReturn) }
    }
}
