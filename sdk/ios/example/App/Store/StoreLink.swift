import Foundation

/// 一次「打开小程序」：外部 App / H5 的链接、商店列表的点击、`wx.navigateToMiniProgram` 都归到这里。
/// 与 Android 示例 StoreLink.kt、Flutter 示例 lib/store/store_links.dart 同一套规则。
///
/// 链接的形状仿微信的 `weixin://dl/business/?appid=&path=&query=`：
///     minirender://open?appid=wx3b7d9f1a2c4e6b80&path=pages/detail/detail&query=id%3D3
/// 也认 mini-store 的 H5 落地页地址（`http(s)://<商店>/open?appid=…`、`/a/<AppID>?path=…`）。
struct StoreLink {
    /// AppID（或 id）
    var key: String
    /// 页面路径（不带前导 `/`）；空 = 首页
    var path = ""
    var query: [(String, String)] = []
    /// 微信的场景值：1001 小程序列表、1065 URL Scheme（H5 / 外部 App）、1037 小程序打开小程序
    var scene = 1065
    /// 从别的小程序跳来时：`{appId, extraData}`
    var referrer: [String: Any]?

    /// 引擎的起始页（`pages/x/x?a=1`）；nil = 首页
    var route: String? {
        if path.isEmpty { return nil }
        if query.isEmpty { return path }
        var c = URLComponents()
        c.queryItems = query.map { URLQueryItem(name: $0.0, value: $0.1) }
        return path + "?" + (c.percentEncodedQuery ?? "")
    }

    /// 交给引擎的 launchOptions（`wx.getLaunchOptionsSync()` / `App.onLaunch(options)`）
    var launchOptions: [String: Any] {
        var o: [String: Any] = ["path": path, "query": Dictionary(query, uniquingKeysWith: { _, b in b }), "scene": scene]
        if let referrer { o["referrerInfo"] = referrer }
        return o
    }

    /// `minirender://open?...` / `http(s)://…/open?appid=…` / `…/a/<AppID>`；认不出返回 nil
    static func parse(_ url: String) -> StoreLink? {
        guard let c = URLComponents(string: url.trimmingCharacters(in: .whitespaces)) else { return nil }
        let items = c.queryItems ?? []
        func param(_ n: String) -> String? { items.first { $0.name == n }?.value }
        let segs = c.path.split(separator: "/").map(String.init)
        let key: String?
        switch c.scheme {
        case "minirender": key = c.host == "open" ? (param("appid") ?? param("id")) : nil
        case "http", "https":
            if segs == ["open"] { key = param("appid") } else if segs.count == 2, segs[0] == "a" { key = segs[1] } else { key = nil }
        default: key = nil
        }
        guard let key, !key.isEmpty else { return nil }
        var path = String((param("path") ?? "").drop(while: { $0 == "/" }))
        var query: [(String, String)] = []
        // path 里可以直接带 ?a=1（微信也允许），query 参数另给的再合进去
        if let qi = path.firstIndex(of: "?") {
            query += split(String(path[path.index(after: qi)...]))
            path = String(path[..<qi])
        }
        if let q = param("query"), !q.isEmpty { query += split(q) }
        return StoreLink(key: key, path: path, query: query)
    }

    private static func split(_ q: String) -> [(String, String)] {
        (URLComponents(string: "x://x?" + q)?.queryItems ?? []).map { ($0.name, $0.value ?? "") }
    }
}
