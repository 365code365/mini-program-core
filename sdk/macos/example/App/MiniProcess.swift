import AppKit

/// 一个小程序一个进程（PC 微信也是这样：每个小程序窗口是一个独立的 WeChatAppEx 进程）。
///
/// 为什么不在主进程里开多个窗口：引擎的小程序根目录、数据目录、图片缓存的键、tabBar 高度都是**进程级**的，
/// 同一进程里两个引擎会互相串页面栈与 storage。分进程之后各管各的，一个小程序崩了也不带走主窗口与别的小程序。
///
/// 子进程就是这个 App 自己，带 `-miniHost 1` 起来：不建主窗口、不进 Dock，只开一个小程序窗口，关了就退出。
/// 本机记录（最近使用、收藏）在同一个 UserDefaults 域里，两边共用；主窗口回到前台时重读一次。
enum MiniProcess {
    /// 已经开着的小程序（key = AppID / id，内置示例是 "demo"）→ 子进程
    private static var running: [String: Process] = [:]

    /// 打开一个小程序：已经开着就把它的窗口叫到前面，没开就起一个子进程
    static func open(_ link: StoreLink?) {
        let key = link?.key ?? "demo"
        if let p = running[key], p.isRunning {
            NSRunningApplication(processIdentifier: p.processIdentifier)?.activate()
            return
        }
        guard let exe = Bundle.main.executablePath else { return }
        var args = ["-miniHost", "1", "-cascade", "\(running.values.filter(\.isRunning).count)"]
        if let link {
            let j: [String: Any] = ["key": link.key, "path": link.path, "query": Dictionary(link.query, uniquingKeysWith: { _, b in b }),
                                    "scene": link.scene, "referrer": link.referrer ?? NSNull()]
            // base64：`-link {…}` 会被 UserDefaults 的参数域当成 plist 字典解析，string(forKey:) 拿到 nil
            let s = (try? JSONSerialization.data(withJSONObject: j))?.base64EncodedString() ?? ""
            args += ["-link", s]
        } else {
            args += ["-demo", "1"]
        }
        // 商店地址、诊断开关跟着主进程走
        args += ["-store", MiniStore.configuredURL]
        let p = Process()
        p.executableURL = URL(fileURLWithPath: exe)
        p.arguments = args
        p.terminationHandler = { _ in DispatchQueue.main.async { if running[key]?.isRunning == false { running[key] = nil } } }
        do {
            try p.run()
            running[key] = p
            NSLog("[多开] 起子进程 %d 打开 %@（同时开着 %d 个）", p.processIdentifier, key, running.values.filter(\.isRunning).count)
        } catch {
            NSLog("[多开] 起子进程失败：%@", error.localizedDescription)
        }
    }

    /// 子进程收到的 `-link`
    static func link(fromArgument s: String) -> StoreLink? {
        guard let d = Data(base64Encoded: s), let j = (try? JSONSerialization.jsonObject(with: d)) as? [String: Any],
              let key = j["key"] as? String else { return nil }
        let q = (j["query"] as? [String: Any] ?? [:]).map { ($0.key, "\($0.value)") }
        return StoreLink(key: key, path: j["path"] as? String ?? "", query: q, scene: (j["scene"] as? NSNumber)?.intValue ?? 1001,
                         referrer: j["referrer"] as? [String: Any])
    }

    /// 主进程退出时把子进程一起关掉（PC 微信退出时小程序窗口也一起关）
    static func terminateAll() {
        running.values.filter(\.isRunning).forEach { $0.terminate() }
    }
}
