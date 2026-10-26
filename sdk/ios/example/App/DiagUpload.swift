import OSLog
import UIKit
import zlib

/// 「上传诊断日志」：把**本进程**的日志（引擎 `mini-render`、宿主事件 `[事件]`、系统报错都在里面）
/// 连同机型、系统、当前小程序、本地装着的版本打成一份 JSON，gzip 后 POST 到 mini-store 的 `/api/diag`。
/// 开发机上 `bash tools/mini-store/diag.sh get latest` 拉回来分析。与 Android 示例 DiagUpload.kt 上传的是同一种 JSON。
///
/// 日志来源：iOS 15 起 `OSLogStore(scope: .currentProcessIdentifier)` 读得到本进程写过的日志（NSLog 也在里面），
/// 不要权限；iOS 13/14 没有这个接口，只传设备信息与说明。
enum DiagUpload {
    private static let maxLogBytes = 3 << 20

    /// 问一句要不要附一句说明，然后后台收集 + 上传，结果弹框给出诊断 id
    static func ask(_ vc: UIViewController, baseUrl: String, app: [String: Any]? = nil, backend: String? = nil) {
        let a = UIAlertController(title: "上传诊断日志", message: "会上传本 App 的运行日志、机型与系统版本，用于排查问题。", preferredStyle: .alert)
        a.addTextField { $0.placeholder = "发生了什么（可不填）" }
        a.addAction(UIAlertAction(title: "取消", style: .cancel))
        a.addAction(UIAlertAction(title: "上传", style: .default) { [weak vc, weak a] _ in
            guard let vc else { return }
            upload(vc, baseUrl: baseUrl, note: a?.textFields?.first?.text ?? "", app: app, backend: backend)
        })
        topmost(vc).present(a, animated: true)
    }

    private static func upload(_ vc: UIViewController, baseUrl: String, note: String, app: [String: Any]?, backend: String?) {
        StoreSheets.toast("正在收集日志…")
        DispatchQueue.global(qos: .userInitiated).async {
            let result = Result { try post("\(baseUrl)/api/diag", collect(note: note, app: app, backend: backend)) }
            DispatchQueue.main.async { [weak vc] in
                guard let vc else { return }
                let (title, msg): (String, String)
                switch result {
                case let .success(id): (title, msg) = ("已上传", "诊断 id：\(id)\n\n把这个 id 发给开发，或在开发机上跑\nbash tools/mini-store/diag.sh get \(id)")
                case let .failure(e): (title, msg) = ("上传失败", e.localizedDescription)
                }
                NSLog("[诊断] %@ %@", title, msg)
                let a = UIAlertController(title: title, message: msg, preferredStyle: .alert)
                a.addAction(UIAlertAction(title: "好", style: .default))
                topmost(vc).present(a, animated: true)
            }
        }
    }

    private static func collect(note: String, app: [String: Any]?, backend: String?) -> [String: Any] {
        let info = Bundle.main.infoDictionary ?? [:]
        var machine = utsname()
        uname(&machine)
        let model = withUnsafeBytes(of: &machine.machine) { String(decoding: $0.prefix { $0 != 0 }, as: UTF8.self) }
        let d = UIDevice.current
        return [
            "host": "ios-native",
            "hostVersion": "\(info["CFBundleShortVersionString"] ?? "") (\(info["CFBundleVersion"] ?? ""))",
            "platform": "ios",
            "device": "Apple \(model)",
            "system": "\(d.systemName) \(d.systemVersion)",
            "abi": "arm64",
            "backend": backend ?? NSNull(),
            "app": app ?? NSNull(),
            "store": ["installed": MiniStore().installed()],
            "note": note,
            "clientTime": Int(Date().timeIntervalSince1970 * 1000),
            "logs": logs(),
        ]
    }

    /// 本进程的日志（最近一小时）；太长只留最后 [maxLogBytes]
    private static func logs() -> String {
        guard #available(iOS 15.0, *) else { return "（iOS 15 以下读不到本进程日志）" }
        do {
            let store = try OSLogStore(scope: .currentProcessIdentifier)
            let since = store.position(date: Date().addingTimeInterval(-3600))
            let f = DateFormatter()
            f.dateFormat = "MM-dd HH:mm:ss.SSS"
            var text = ""
            for e in try store.getEntries(at: since) {
                guard let m = e as? OSLogEntryLog else { continue }
                text += "\(f.string(from: m.date)) \(m.subsystem.isEmpty ? m.process : m.subsystem) \(m.composedMessage)\n"
            }
            if text.utf8.count > maxLogBytes {
                let tail = String(text.suffix(maxLogBytes))
                return "…（前面截掉 \(text.count - tail.count) 字符）\n" + tail
            }
            return text
        } catch {
            return "（读日志失败：\(error.localizedDescription)）"
        }
    }

    private static func post(_ url: String, _ body: [String: Any]) throws -> String {
        guard let u = URL(string: url) else { throw MiniStore.StoreError(message: "地址不对：\(url)") }
        let raw = try gzip(JSONSerialization.data(withJSONObject: body))
        var req = URLRequest(url: u, timeoutInterval: 30)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.setValue("gzip", forHTTPHeaderField: "Content-Encoding")
        req.httpBody = raw
        let done = DispatchSemaphore(value: 0)
        var out: (Data?, URLResponse?, Error?) = (nil, nil, nil)
        URLSession.shared.dataTask(with: req) { d, r, e in
            out = (d, r, e)
            done.signal()
        }.resume()
        done.wait()
        if let e = out.2 { throw e }
        let code = (out.1 as? HTTPURLResponse)?.statusCode ?? 0
        let j = out.0.map(MiniStore.object) ?? [:]
        guard (200..<300).contains(code) else {
            throw MiniStore.StoreError(message: (j["error"] as? String).flatMap { $0.isEmpty ? nil : $0 } ?? "HTTP \(code)")
        }
        guard let id = j["id"] as? String, !id.isEmpty else { throw MiniStore.StoreError(message: "服务器没回诊断 id") }
        return id
    }

    /// gzip 格式（zlib 的 deflateInit2 windowBits = 15 + 16）：服务器按 Content-Encoding: gzip 解
    private static func gzip(_ data: Data) throws -> Data {
        var s = z_stream()
        guard deflateInit2_(&s, Z_DEFAULT_COMPRESSION, Z_DEFLATED, 15 + 16, 8, Z_DEFAULT_STRATEGY,
                            ZLIB_VERSION, Int32(MemoryLayout<z_stream>.size)) == Z_OK else {
            throw MiniStore.StoreError(message: "压缩失败")
        }
        defer { deflateEnd(&s) }
        var out = Data(count: Int(deflateBound(&s, uLong(data.count))))
        let n: Int = data.withUnsafeBytes { src in
            out.withUnsafeMutableBytes { dst in
                s.next_in = UnsafeMutablePointer(mutating: src.bindMemory(to: Bytef.self).baseAddress)
                s.avail_in = uInt(data.count)
                s.next_out = dst.bindMemory(to: Bytef.self).baseAddress
                s.avail_out = uInt(dst.count)
                return deflate(&s, Z_FINISH) == Z_STREAM_END ? Int(s.total_out) : -1
            }
        }
        guard n >= 0 else { throw MiniStore.StoreError(message: "压缩失败") }
        return out.prefix(n)
    }

    private static func topmost(_ vc: UIViewController) -> UIViewController {
        var t = vc
        while let p = t.presentedViewController { t = p }
        return t
    }
}
