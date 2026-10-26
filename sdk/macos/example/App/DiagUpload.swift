import AppKit
import OSLog
import zlib

/// 「上传诊断日志」：本进程的日志（`OSLogStore(scope: .currentProcessIdentifier)`，NSLog 也在里面）+ 机型、系统、
/// 当前小程序、本地装着的版本，gzip 后 POST 到 mini-store 的 `/api/diag`。与 Android / iOS 示例上传的是同一种 JSON，
/// 开发机上 `bash tools/mini-store/diag.sh get latest` 拉回来。
enum DiagUpload {
    private static let maxLogBytes = 3 << 20

    static func ask(_ window: NSWindow, baseUrl: String, app: [String: Any]? = nil, backend: String? = nil) {
        let a = NSAlert()
        a.messageText = "上传诊断日志"
        a.informativeText = "会上传本 App 的运行日志、机型与系统版本，用于排查问题。"
        let note = NSTextField(frame: NSRect(x: 0, y: 0, width: 260, height: 24))
        note.placeholderString = "发生了什么（可不填）"
        a.accessoryView = note
        a.addButton(withTitle: "上传")
        a.addButton(withTitle: "取消")
        a.beginSheetModal(for: window) { resp in
            guard resp == .alertFirstButtonReturn else { return }
            let text = note.stringValue
            DispatchQueue.global(qos: .userInitiated).async {
                let r = Result { try post("\(baseUrl)/api/diag", collect(note: text, app: app, backend: backend)) }
                DispatchQueue.main.async {
                    let done = NSAlert()
                    switch r {
                    case let .success(id):
                        done.messageText = "已上传"
                        done.informativeText = "诊断 id：\(id)\n\n把这个 id 发给开发，或在开发机上跑\nbash tools/mini-store/diag.sh get \(id)"
                    case let .failure(e):
                        done.messageText = "上传失败"
                        done.informativeText = e.localizedDescription
                    }
                    NSLog("[诊断] %@ %@", done.messageText, done.informativeText)
                    done.beginSheetModal(for: window)
                }
            }
        }
    }

    private static func collect(note: String, app: [String: Any]?, backend: String?) -> [String: Any] {
        let info = Bundle.main.infoDictionary ?? [:]
        var size = 0
        sysctlbyname("hw.model", nil, &size, nil, 0)
        var model = [CChar](repeating: 0, count: max(size, 1))
        sysctlbyname("hw.model", &model, &size, nil, 0)
        let os = ProcessInfo.processInfo.operatingSystemVersionString
        return [
            "host": "macos-native",
            "hostVersion": "\(info["CFBundleShortVersionString"] ?? "") (\(info["CFBundleVersion"] ?? ""))",
            "platform": "mac",
            "device": "Apple \(String(cString: model))",
            "system": "macOS \(os)",
            "abi": "arm64",
            "backend": backend ?? NSNull(),
            "app": app ?? NSNull(),
            "store": ["installed": MiniStore().installed()],
            "note": note,
            "clientTime": Int(Date().timeIntervalSince1970 * 1000),
            "logs": logs(),
        ]
    }

    private static func logs() -> String {
        do {
            let store = try OSLogStore(scope: .currentProcessIdentifier)
            let f = DateFormatter()
            f.dateFormat = "MM-dd HH:mm:ss.SSS"
            var text = ""
            for e in try store.getEntries(at: store.position(date: Date().addingTimeInterval(-3600))) {
                guard let m = e as? OSLogEntryLog else { continue }
                text += "\(f.string(from: m.date)) \(m.subsystem.isEmpty ? m.process : m.subsystem) \(m.composedMessage)\n"
            }
            return text.utf8.count > maxLogBytes ? "…（前面截掉）\n" + String(text.suffix(maxLogBytes)) : text
        } catch {
            return "（读日志失败：\(error.localizedDescription)）"
        }
    }

    private static func post(_ url: String, _ body: [String: Any]) throws -> String {
        guard let u = URL(string: url) else { throw MiniStore.StoreError(message: "地址不对：\(url)") }
        var req = URLRequest(url: u, timeoutInterval: 30)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.setValue("gzip", forHTTPHeaderField: "Content-Encoding")
        req.httpBody = try gzip(JSONSerialization.data(withJSONObject: body))
        let done = DispatchSemaphore(value: 0)
        var out: (Data?, URLResponse?, Error?) = (nil, nil, nil)
        URLSession.shared.dataTask(with: req) { d, r, e in out = (d, r, e); done.signal() }.resume()
        done.wait()
        if let e = out.2 { throw e }
        let code = (out.1 as? HTTPURLResponse)?.statusCode ?? 0
        let j = out.0.map(MiniStore.object) ?? [:]
        guard (200..<300).contains(code) else { throw MiniStore.StoreError(message: (j["error"] as? String) ?? "HTTP \(code)") }
        guard let id = j["id"] as? String, !id.isEmpty else { throw MiniStore.StoreError(message: "服务器没回诊断 id") }
        return id
    }

    private static func gzip(_ data: Data) throws -> Data {
        var s = z_stream()
        guard deflateInit2_(&s, Z_DEFAULT_COMPRESSION, Z_DEFLATED, 15 + 16, 8, Z_DEFAULT_STRATEGY,
                            ZLIB_VERSION, Int32(MemoryLayout<z_stream>.size)) == Z_OK else { throw MiniStore.StoreError(message: "压缩失败") }
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
}
