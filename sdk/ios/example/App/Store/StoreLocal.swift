import Foundation

/// 本机记着的三样（与微信一样只存在手机上，与 Android 示例 StoreLocal.kt 同一套规则）：
/// - 最近使用的小程序（入口页那排、下拉面板，最新在前，最多 [maxRecent] 个，存整条 App：离线也画得出来）；
/// - 我的小程序（收藏，后收藏的在前，最多 [maxFavorites] 个）；
/// - 搜索历史（搜索页没输入时显示，最多 [maxHistory] 条，可清空）。
/// 每改一次发 [changed] 通知（主线程）：在面板 / 胶囊菜单 / 搜索页里收藏、删除，入口页跟着变。
struct StoreLocal {
    static let changed = Notification.Name("StoreLocal.changed")
    static let maxRecent = 8
    static let maxHistory = 12
    static let maxFavorites = 50

    private let prefs = UserDefaults(suiteName: "store-local") ?? .standard

    func recent() -> [MiniStore.App] { apps("recent") }

    func touch(_ app: MiniStore.App) {
        putApps("recent", Array(([app] + recent().filter { $0.id != app.id }).prefix(Self.maxRecent)), notify: false)
        // 收藏里存的是收藏那一刻的版本：打开时顺手换成最新的（名字 / 图标改了，面板上跟着变）
        let fav = favorites()
        if fav.contains(where: { $0.id == app.id }) {
            putApps("favorites", fav.map { $0.id == app.id ? app : $0 }, notify: false)
        }
        notify()
    }

    /// 从「最近使用」里删掉（长按 → 删除）；收藏不受影响
    func removeRecent(_ id: String) { putApps("recent", recent().filter { $0.id != id }) }

    func favorites() -> [MiniStore.App] { apps("favorites") }

    func isFavorite(_ id: String) -> Bool { favorites().contains { $0.id == id } }

    /// 收藏 / 取消收藏；返回之后的状态（true = 已在「我的小程序」里）
    @discardableResult
    func toggleFavorite(_ app: MiniStore.App) -> Bool {
        let fav = favorites()
        let had = fav.contains { $0.id == app.id }
        putApps("favorites", had ? fav.filter { $0.id != app.id } : Array(([app] + fav).prefix(Self.maxFavorites)))
        return !had
    }

    func history() -> [String] { prefs.stringArray(forKey: "history") ?? [] }

    func addHistory(_ q: String) {
        let w = q.trimmingCharacters(in: .whitespaces)
        if w.isEmpty { return }
        prefs.set(Array(([w] + history().filter { $0 != w }).prefix(Self.maxHistory)), forKey: "history")
        notify()
    }

    func clearHistory() {
        prefs.set([String](), forKey: "history")
        notify()
    }

    private func apps(_ key: String) -> [MiniStore.App] {
        guard let s = prefs.string(forKey: key), let d = s.data(using: .utf8),
              let arr = (try? JSONSerialization.jsonObject(with: d)) as? [[String: Any]] else { return [] }
        return arr.compactMap(MiniStore.App.init(json:))
    }

    private func putApps(_ key: String, _ list: [MiniStore.App], notify n: Bool = true) {
        if let d = try? JSONSerialization.data(withJSONObject: list.map(\.json)) {
            prefs.set(String(decoding: d, as: UTF8.self), forKey: key)
        }
        if n { notify() }
    }

    private func notify() {
        DispatchQueue.main.async { NotificationCenter.default.post(name: Self.changed, object: nil) }
    }
}
