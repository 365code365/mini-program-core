import UIKit

/// 入口：[StoreViewController]（Android 示例的 StoreActivity）。
/// 外部 App / H5 拉起小程序：`minirender://open?appid=…`（project.yml 的 CFBundleURLTypes）—— 与 Android 的
/// LinkActivity 一样解析后交给小程序页；已经开着一个小程序时就地换掉（单实例）。
///
/// 验收脚本用的启动参数（`simctl launch … -autoOpen 1 [-route pages/x/x]`）：直接打开内置示例，不经过入口页。
final class SceneDelegate: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?

    func scene(_ scene: UIScene, willConnectTo session: UISceneSession, options connectionOptions: UIScene.ConnectionOptions) {
        guard let windowScene = scene as? UIWindowScene else { return }
        let window = UIWindow(windowScene: windowScene)
        let root = UINavigationController(rootViewController: StoreViewController())
        root.setNavigationBarHidden(true, animated: false)
        window.rootViewController = root
        window.makeKeyAndVisible()
        self.window = window
        if UserDefaults.standard.bool(forKey: "autoOpen") {
            StoreWidgets.present(MiniViewController(route: UserDefaults.standard.string(forKey: "route")), from: root)
        } else if let key = UserDefaults.standard.string(forKey: "open"), !key.isEmpty {
            // `-open <AppID 或 id>`：直接走商店那条路（问最新版 → 下载 → 打开），与 Android 的 `am start … --es` 对应。
            // 用链接不行：Flutter 示例「灵犀」也认 minirender://，两个都装着时系统会弹框问开哪个
            // `-route pages/x/x?a=1` 一起给就是深链到那一页
            var link = StoreLink.parse("minirender://open?" + (URLComponents(string: "x://x")!.with {
                $0.queryItems = [URLQueryItem(name: "appid", value: key),
                                 URLQueryItem(name: "path", value: UserDefaults.standard.string(forKey: "route") ?? "")]
            }.percentEncodedQuery ?? "")) ?? StoreLink(key: key)
            link.scene = 1001
            StoreWidgets.present(MiniViewController(link: link), from: root)
        }
        if let url = connectionOptions.urlContexts.first?.url { open(url) }
    }

    func scene(_ scene: UIScene, openURLContexts contexts: Set<UIOpenURLContext>) {
        contexts.first.map { open($0.url) }
    }

    private func open(_ url: URL) {
        let link = StoreLink.parse(url.absoluteString)
        NSLog("[商店] 外部链接 %@ → %@", url.absoluteString, link.map { "\($0.key) \($0.route ?? "首页")" } ?? "认不出")
        guard let link, let root = window?.rootViewController else { return }
        var top = root
        while let p = top.presentedViewController { top = p }
        if let mini = top as? MiniViewController {
            mini.handle(link)
        } else {
            StoreWidgets.present(MiniViewController(link: link), from: root)
        }
    }
}

extension URLComponents {
    /// 就地改一份再返回（构造带参数的链接用）
    func with(_ edit: (inout URLComponents) -> Void) -> URLComponents {
        var c = self
        edit(&c)
        return c
    }
}
