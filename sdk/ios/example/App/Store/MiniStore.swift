import CryptoKit
import Foundation

/// 小程序商店客户端：从 mini-store 服务（tools/mini-store/server.py）拉列表、下载包、解到本地目录。
/// 与 Android 示例 MiniStore.kt、Flutter 示例 lib/store/mini_store.dart 同一套规则：
///
/// - **每次打开前都问一次服务器最新版**（[prepare]，`GET /api/apps/<AppID>`）：sha256 与本地一致就直接打开，
///   不一致就先下新包。列表里的版本信息可能是几分钟前拉的，不能拿来判断；
/// - 解到 `Application Support/mini-apps/<id>/`：目录名就是 id —— 引擎的 storage 按目录名分文件，各小程序不串数据；
///   旁边 `<id>.json` 记着本地这一版（AppID、版本、sha256）；
/// - 服务器连不上：本地有就打开本地那版（离线可用），没有才报错；服务器明确说没有（404，已下架）就不打开；
/// - 先解到临时目录再整个换上去，解一半断掉不会留下半个小程序。
///
/// 全部是阻塞调用，放后台队列跑。
final class MiniStore {
    /// 测试服务器（明文 HTTP，Info.plist 的 NSAllowsArbitraryLoads 放行）。启动参数 `-store http://…` 可以换
    static let defaultURL = "http://64.83.22.217:9630"

    struct App: Equatable {
        var id: String
        var appId: String
        var name: String
        var description: String
        var version: Int
        var sha256: String
        var size: Int64
        var packageUrl: String
        /// 商店展示与搜索用（服务器 store.meta.json / 管理页填的）；老服务器没有这些字段
        var category = ""
        var tags: [String] = []
        var developer = ""
        var iconUrl = ""
        var downloads = 0

        /// 深链 / 跳转用的 key：有 AppID 用 AppID
        var key: String { appId.isEmpty ? id : appId }

        var json: [String: Any] {
            ["id": id, "appId": appId, "name": name, "description": description, "version": version, "sha256": sha256,
             "size": size, "packageUrl": packageUrl, "category": category, "tags": tags, "developer": developer,
             "iconUrl": iconUrl, "downloads": downloads]
        }

        init?(json j: [String: Any]) {
            guard let id = j["id"] as? String else { return nil }
            self.id = id
            appId = j["appId"] as? String ?? ""
            name = (j["name"] as? String).flatMap { $0.isEmpty ? nil : $0 } ?? id
            description = j["description"] as? String ?? ""
            version = (j["version"] as? NSNumber)?.intValue ?? 0
            sha256 = j["sha256"] as? String ?? ""
            size = (j["size"] as? NSNumber)?.int64Value ?? 0
            packageUrl = (j["packageUrl"] as? String).flatMap { $0.isEmpty ? nil : $0 } ?? "/api/apps/\(id)/package"
            category = j["category"] as? String ?? ""
            tags = (j["tags"] as? [Any])?.map { "\($0)" } ?? []
            developer = j["developer"] as? String ?? ""
            iconUrl = j["iconUrl"] as? String ?? ""
            downloads = (j["downloads"] as? NSNumber)?.intValue ?? 0
        }

        static func == (a: App, b: App) -> Bool { a.id == b.id && a.version == b.version && a.sha256 == b.sha256 }
    }

    /// [offline] = 服务器连不上，用的是上次缓存的列表
    struct Listing {
        let apps: [App]
        let offline: Bool
        let error: String?
    }

    /// [stale] 不为 nil：没拿到最新版，打开的是本地旧版（值是原因）；[updated] = 这次下了新包
    struct Prepared {
        let dir: URL
        let app: App
        let updated: Bool
        let stale: Error?
    }

    /// 服务器明确回了 404：没有这个小程序（AppID 不对 / 已下架）
    struct NotFound: LocalizedError {
        let message: String
        var errorDescription: String? { message }
    }

    struct StoreError: LocalizedError {
        let message: String
        var errorDescription: String? { message }
    }

    let baseUrl: String
    private let root: URL

    init(baseUrl: String = MiniStore.configuredURL) {
        self.baseUrl = baseUrl
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        root = support.appendingPathComponent("mini-apps", isDirectory: true)
        try? FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    }

    /// 启动参数 `-store <url>`（Xcode scheme 或 `simctl launch … -store http://…`）换商店地址
    static var configuredURL: String {
        UserDefaults.standard.string(forKey: "store").flatMap { $0.isEmpty ? nil : $0 } ?? defaultURL
    }

    // MARK: - 列表与搜索

    func list() -> Listing {
        let cache = root.appendingPathComponent("index.json")
        do {
            let body = try get("\(baseUrl)/api/apps")
            let apps = try Self.parse(body)
            try? body.write(to: cache)
            return Listing(apps: apps, offline: false, error: nil)
        } catch {
            let apps = (try? Data(contentsOf: cache)).flatMap { try? Self.parse($0) } ?? []
            return Listing(apps: apps, offline: true, error: error.localizedDescription)
        }
    }

    /// 搜索（服务器 `/api/apps?q=&category=`）：关键词每段都要命中，按相关度 + 热度排。返回 (结果, 总数)
    func search(_ q: String, category: String = "") throws -> ([App], Int) {
        let body = try get("\(baseUrl)/api/apps?q=\(Self.enc(q))&category=\(Self.enc(category))&limit=100", timeout: 6)
        let total = (Self.object(body)["total"] as? NSNumber)?.intValue ?? 0
        return (try Self.parse(body), total)
    }

    /// 热搜词（服务器按搜索次数排，冷启动时用热门小程序补齐）
    func hotKeywords() throws -> [String] {
        Self.strings(Self.object(try get("\(baseUrl)/api/search/hot", timeout: 6)), "keywords")
    }

    /// 输入联想
    func suggest(_ q: String) throws -> [String] {
        Self.strings(Self.object(try get("\(baseUrl)/api/search/suggest?q=\(Self.enc(q))", timeout: 4)), "words")
    }

    /// 分类与数量（服务器的固定顺序）
    func categories() throws -> [(String, Int)] {
        let a = Self.object(try get("\(baseUrl)/api/categories", timeout: 6))["categories"] as? [[String: Any]] ?? []
        return a.compactMap { o in (o["name"] as? String).map { ($0, (o["count"] as? NSNumber)?.intValue ?? 0) } }
    }

    /// 商店图标的字节（`mini-apps/icons/<id>-v<版本>.png` 缓存，换版本换文件）；没有图标返回 nil
    func iconData(_ app: App) -> Data? {
        if app.iconUrl.isEmpty { return nil }
        let dir = root.appendingPathComponent("icons", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let f = dir.appendingPathComponent("\(app.id)-v\(app.version).png")
        if let d = try? Data(contentsOf: f) { return d }
        guard let url = URL(string: app.iconUrl, relativeTo: URL(string: baseUrl)),
              let bytes = try? get(url.absoluteString, timeout: 6) else { return nil }
        for old in (try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? [] where old.hasPrefix("\(app.id)-v") {
            try? FileManager.default.removeItem(at: dir.appendingPathComponent(old))
        }
        try? bytes.write(to: f)
        return bytes
    }

    // MARK: - 打开

    /// 服务器上的最新版（key = AppID 或 id）
    func latest(_ key: String) throws -> App {
        guard Self.matches(key, Self.appIdPattern) || Self.matches(key, Self.idPattern) else {
            throw NotFound(message: "非法的 AppID：\(key)")
        }
        guard let app = App(json: Self.object(try get("\(baseUrl)/api/apps/\(key)", timeout: 6))) else {
            throw StoreError(message: "服务器回的不是小程序信息")
        }
        guard Self.matches(app.id, Self.idPattern) else { throw StoreError(message: "服务器给的 id 不合法") }
        return app
    }

    /// 本地装着的那一版（key = AppID 或 id）；没装返回 nil
    func local(_ key: String) -> App? {
        let fm = FileManager.default
        let names = (try? fm.contentsOfDirectory(atPath: root.path)) ?? []
        for n in names where n.hasSuffix(".json") && n != "index.json" {
            guard let d = try? Data(contentsOf: root.appendingPathComponent(n)), let app = App(json: Self.object(d)) else { continue }
            if (app.id == key || app.appId == key), fm.fileExists(atPath: root.appendingPathComponent("\(app.id)/app.json").path) {
                return app
            }
        }
        return nil
    }

    /// 打开前的准备：问服务器最新版 → 本地不是这一版就下载解包 → 返回目录。
    /// [status] 给 loading 页的阶段文字，[progress] 给（已收, 总字节）
    func prepare(_ key: String, status: (String) -> Void = { _ in }, progress: @escaping (Int64, Int64) -> Void = { _, _ in }) throws -> Prepared {
        status("检查更新…")
        let app: App
        do {
            app = try latest(key)
        } catch let e as NotFound {
            throw e // 服务器明确说没有：不拿本地旧版糊弄
        } catch {
            guard let have = local(key) else { throw error }
            return Prepared(dir: root.appendingPathComponent(have.id), app: have, updated: false, stale: error)
        }
        let installed = local(app.id)
        if let installed, installed.sha256 == app.sha256 {
            return Prepared(dir: root.appendingPathComponent(app.id), app: app, updated: false, stale: nil)
        }
        status(installed == nil ? "下载中…" : "发现新版本 v\(app.version)，更新中…")
        do {
            guard let url = URL(string: app.packageUrl, relativeTo: URL(string: baseUrl)) else { throw StoreError(message: "包地址不对") }
            let bytes = try get(url.absoluteString, sizeHint: app.size, progress: progress)
            if !app.sha256.isEmpty, Self.sha256(bytes) != app.sha256 { throw StoreError(message: "包校验失败（sha256 与服务器给的不一致）") }
            status("解包中…")
            try unpack(bytes, app)
            return Prepared(dir: root.appendingPathComponent(app.id), app: app, updated: true, stale: nil)
        } catch {
            guard let installed else { throw error }
            return Prepared(dir: root.appendingPathComponent(installed.id), app: installed, updated: false, stale: error)
        }
    }

    /// 本地装着的全部（诊断日志里带上：版本对不上是常见问题来源）
    func installed() -> [[String: Any]] {
        let names = (try? FileManager.default.contentsOfDirectory(atPath: root.path)) ?? []
        return names.filter { $0.hasSuffix(".json") && $0 != "index.json" }
            .compactMap { (try? Data(contentsOf: root.appendingPathComponent($0))).map(Self.object) }
    }

    // MARK: - HTTP

    private func get(_ url: String, sizeHint: Int64 = 0, timeout: TimeInterval = 8,
                     progress: ((Int64, Int64) -> Void)? = nil) throws -> Data {
        guard let u = URL(string: url) else { throw StoreError(message: "地址不对：\(url)") }
        var req = URLRequest(url: u, timeoutInterval: timeout)
        req.cachePolicy = .reloadIgnoringLocalCacheData
        let loader = Download(progress: progress, sizeHint: sizeHint)
        let (data, resp) = try loader.run(req)
        let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
        if code == 404 {
            let msg = Self.object(data)["error"] as? String
            throw NotFound(message: (msg?.isEmpty == false ? msg : nil) ?? "没有这个小程序")
        }
        if code != 200 { throw StoreError(message: "HTTP \(code)") }
        return data
    }

    private func unpack(_ bytes: Data, _ app: App) throws {
        let fm = FileManager.default
        let tmp = root.appendingPathComponent(".\(app.id).tmp", isDirectory: true)
        try? fm.removeItem(at: tmp)
        try fm.createDirectory(at: tmp, withIntermediateDirectories: true)
        try ZipReader.extract(bytes, to: tmp)
        guard fm.fileExists(atPath: tmp.appendingPathComponent("app.json").path) else { throw StoreError(message: "包里没有 app.json") }
        let dir = root.appendingPathComponent(app.id, isDirectory: true)
        try? fm.removeItem(at: dir)
        do { try fm.moveItem(at: tmp, to: dir) } catch { throw StoreError(message: "换上新版失败") }
        let meta = try JSONSerialization.data(withJSONObject: app.json)
        try meta.write(to: root.appendingPathComponent("\(app.id).json"))
    }

    // MARK: - 小工具

    /// 与服务器同一条规则：id 要当目录名用
    static let idPattern = "^[a-z0-9][a-z0-9._-]{0,63}$"
    /// 微信 AppID 的形状（服务器分配的也是这个形状）
    static let appIdPattern = "^wx[0-9a-f]{16}$"

    static func matches(_ s: String, _ pattern: String) -> Bool {
        s.range(of: pattern, options: .regularExpression) != nil
    }

    static func object(_ d: Data) -> [String: Any] {
        (try? JSONSerialization.jsonObject(with: d)) as? [String: Any] ?? [:]
    }

    private static func parse(_ body: Data) throws -> [App] {
        guard let arr = object(body)["apps"] as? [[String: Any]] else { throw StoreError(message: "列表格式不对") }
        return arr.filter { matches($0["id"] as? String ?? "", idPattern) }.compactMap(App.init(json:))
    }

    private static func strings(_ o: [String: Any], _ key: String) -> [String] {
        (o[key] as? [Any] ?? []).map { "\($0)" }.filter { !$0.isEmpty }
    }

    private static func enc(_ s: String) -> String {
        var allowed = CharacterSet.urlQueryAllowed
        allowed.remove(charactersIn: "&=+?#")
        return s.addingPercentEncoding(withAllowedCharacters: allowed) ?? s
    }

    static func sha256(_ d: Data) -> String {
        SHA256.hash(data: d).map { String(format: "%02x", $0) }.joined()
    }
}

/// 一次同步下载（后台队列上调），边收边报进度。URLSession 的 dataTask 拿不到逐块进度，所以走 delegate
private final class Download: NSObject, URLSessionDataDelegate {
    private let progress: ((Int64, Int64) -> Void)?
    private let sizeHint: Int64
    private var data = Data()
    private var response: URLResponse?
    private var error: Error?
    private var total: Int64 = 0
    private let done = DispatchSemaphore(value: 0)

    init(progress: ((Int64, Int64) -> Void)?, sizeHint: Int64) {
        self.progress = progress
        self.sizeHint = sizeHint
    }

    func run(_ req: URLRequest) throws -> (Data, URLResponse?) {
        let cfg = URLSessionConfiguration.ephemeral
        // 单次 20 秒没动静就算断了（整包不设上限：几 MB 的包在慢网上也要能下完）
        cfg.timeoutIntervalForRequest = max(req.timeoutInterval, 20)
        let session = URLSession(configuration: cfg, delegate: self, delegateQueue: nil)
        session.dataTask(with: req).resume()
        done.wait()
        session.finishTasksAndInvalidate()
        if let error { throw error }
        return (data, response)
    }

    func urlSession(_ s: URLSession, dataTask: URLSessionDataTask, didReceive r: URLResponse,
                    completionHandler: @escaping (URLSession.ResponseDisposition) -> Void) {
        response = r
        total = r.expectedContentLength > 0 ? r.expectedContentLength : sizeHint
        completionHandler(.allow)
    }

    func urlSession(_ s: URLSession, dataTask: URLSessionDataTask, didReceive d: Data) {
        data.append(d)
        progress?(Int64(data.count), total)
    }

    func urlSession(_ s: URLSession, task: URLSessionTask, didCompleteWithError e: Error?) {
        error = e
        done.signal()
    }
}
