import AppKit

/// 「灵犀 for Mac」：PC 微信式的宿主示例。主窗口三栏（[MainWindowController]），小程序在独立窗口里开
/// （[MiniWindowController]）。外部链接 `minirender://open?appid=…` 由系统交给这里（Info.plist 的 CFBundleURLTypes）。
///
/// 验收 / 脚本用的启动参数：`-open <AppID 或 id> [-route pages/x/x]` 直接开商店小程序，`-demo 1` 直接开内置示例。
@main
final class AppDelegate: NSObject, NSApplicationDelegate {
    private var main: MainWindowController!

    static func main() {
        let app = NSApplication.shared
        let d = AppDelegate()
        app.delegate = d
        app.setActivationPolicy(.regular)
        app.run()
    }

    func applicationDidFinishLaunching(_ n: Notification) {
        buildMenu()
        main = MainWindowController()
        main.showWindow(nil)
        NSApp.activate(ignoringOtherApps: true)
        NSAppleEventManager.shared().setEventHandler(self, andSelector: #selector(handleURL(_:_:)),
                                                     forEventClass: AEEventClass(kInternetEventClass), andEventID: AEEventID(kAEGetURL))
        let d = UserDefaults.standard
        // `-tab discover|hub|games|favorites|contacts`：起来就切到那一页（截图验收用）
        switch d.string(forKey: "tab") {
        case "discover": main.select(.discover)
        case "hub": main.select(.discover); main.showHub()
        case "games": main.select(.discover); main.showHub(gamesOnly: true)
        case "favorites": main.select(.favorites)
        case "contacts": main.select(.contacts)
        default: break
        }
        // `-shot <目录>`：8 秒后把主窗口与小程序窗口各存一张 PNG（不要屏幕录制权限：窗口用 cacheDisplay、
        // 小程序画面用引擎自己的快照 —— Metal 层 cacheDisplay 截不到）
        if let dir = d.string(forKey: "shot"), !dir.isEmpty {
            DispatchQueue.main.asyncAfter(deadline: .now() + (d.double(forKey: "shotDelay") > 0 ? d.double(forKey: "shotDelay") : 8)) { [weak self] in
                self?.saveShots(to: URL(fileURLWithPath: dir))
            }
        }
        if d.bool(forKey: "demo") {
            main.openDemo()
        } else if let key = d.string(forKey: "open"), !key.isEmpty {
            var c = URLComponents()
            c.queryItems = [URLQueryItem(name: "appid", value: key), URLQueryItem(name: "path", value: d.string(forKey: "route") ?? "")]
            if var l = StoreLink.parse("minirender://open?" + (c.percentEncodedQuery ?? "")) {
                l.scene = 1001
                MiniWindowController.open(link: l, store: main.store)
            }
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }

    private func saveShots(to dir: URL) {
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        if let v = main.window?.contentView, let rep = v.bitmapImageRepForCachingDisplay(in: v.bounds) {
            v.cacheDisplay(in: v.bounds, to: rep)
            try? rep.representation(using: .png, properties: [:])?.write(to: dir.appendingPathComponent("main.png"))
        }
        MiniWindowController.shared?.snapshot { img in
            guard let img, let tiff = img.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff) else { return }
            try? rep.representation(using: .png, properties: [:])?.write(to: dir.appendingPathComponent("mini.png"))
        }
        NSLog("[截图] 已存到 %@", dir.path)
    }

    /// Dock 图标点一下：主窗口回来
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        main.showWindow(nil)
        return true
    }

    @objc private func handleURL(_ event: NSAppleEventDescriptor, _ reply: NSAppleEventDescriptor) {
        guard let s = event.paramDescriptor(forKeyword: keyDirectObject)?.stringValue else { return }
        let link = StoreLink.parse(s)
        NSLog("[商店] 外部链接 %@ → %@", s, link.map { "\($0.key) \($0.route ?? "首页")" } ?? "认不出")
        if let link { MiniWindowController.open(link: link, store: main.store) }
    }

    private func buildMenu() {
        let bar = NSMenu()
        let appItem = NSMenuItem()
        bar.addItem(appItem)
        let appMenu = NSMenu()
        appMenu.addItem(withTitle: "关于灵犀", action: #selector(NSApplication.orderFrontStandardAboutPanel(_:)), keyEquivalent: "")
        appMenu.addItem(.separator())
        appMenu.addItem(withTitle: "隐藏灵犀", action: #selector(NSApplication.hide(_:)), keyEquivalent: "h")
        appMenu.addItem(withTitle: "退出灵犀", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        appItem.submenu = appMenu
        // 编辑菜单：⌘C / ⌘V / ⌘A / ⌘Z 要靠它分发到输入框（小程序的输入框接在一块隐藏的 NSTextView 上）
        let editItem = NSMenuItem()
        bar.addItem(editItem)
        let edit = NSMenu(title: "编辑")
        edit.addItem(withTitle: "撤销", action: Selector(("undo:")), keyEquivalent: "z")
        edit.addItem(.separator())
        edit.addItem(withTitle: "剪切", action: #selector(NSText.cut(_:)), keyEquivalent: "x")
        edit.addItem(withTitle: "复制", action: #selector(NSText.copy(_:)), keyEquivalent: "c")
        edit.addItem(withTitle: "粘贴", action: #selector(NSText.paste(_:)), keyEquivalent: "v")
        edit.addItem(withTitle: "全选", action: #selector(NSText.selectAll(_:)), keyEquivalent: "a")
        editItem.submenu = edit
        let winItem = NSMenuItem()
        bar.addItem(winItem)
        let win = NSMenu(title: "窗口")
        win.addItem(withTitle: "最小化", action: #selector(NSWindow.performMiniaturize(_:)), keyEquivalent: "m")
        win.addItem(withTitle: "关闭", action: #selector(NSWindow.performClose(_:)), keyEquivalent: "w")
        winItem.submenu = win
        NSApp.mainMenu = bar
        NSApp.windowsMenu = win
    }
}
