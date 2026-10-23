import Foundation

/// 小程序调了一个**宿主实现**的 `wx.*`（[MiniProgramView.hostApis] 里登记的名字）。与 Android SDK 的
/// `MiniHostCall` 同一个形状：参数已由引擎按官方默认值补齐；交给小程序的文件写进 [tempDir]，回包里写
/// `"file://" + 绝对路径`（引擎换成 `wxfile://tmp/…`）；小程序交过来的文件真实路径查 [files]。
public struct MiniHostCall {
    public let id: Int
    public let api: String
    public let args: [String: Any]
    /// 宿主可以直接写的临时目录（引擎只换名字、不拷贝）
    public let tempDir: String
    /// `args` 里出现的 `wxfile://…` → 真实路径
    public let files: [String: String]

    init(json o: [String: Any]) {
        id = (o["id"] as? NSNumber)?.intValue ?? 0
        api = o["api"] as? String ?? ""
        args = o["args"] as? [String: Any] ?? [:]
        tempDir = o["tempDir"] as? String ?? ""
        files = o["files"] as? [String: String] ?? [:]
    }
}

/// hostCall 的回包。**只认第一次**，哪条线程都可以调。成功给 `res`（`errMsg` 引擎补 `<api>:ok`），
/// 失败给完整的 `errMsg`（`<api>:fail 原因`；用户取消照微信写 `<api>:fail cancel`）
public final class MiniHostReply {
    private let send: (Bool, [String: Any]) -> Void
    private let lock = NSLock()
    private var done = false

    init(send: @escaping (Bool, [String: Any]) -> Void) {
        self.send = send
    }

    public func success(_ result: [String: Any] = [:]) {
        finish(true, result)
    }

    /// `extra` 合进回包（如 `startSoterAuthentication` 的 `errCode`）
    public func fail(_ errMsg: String, extra: [String: Any] = [:]) {
        var r = extra
        r["errMsg"] = errMsg
        finish(false, r)
    }

    private func finish(_ ok: Bool, _ res: [String: Any]) {
        let first = lock.locked { () -> Bool in
            if done { return false }
            done = true
            return true
        }
        if first { send(ok, res) }
    }
}

/// 宿主实现的一个 `wx.*` API（主线程上调；耗时的活自己挪到后台，做完再回）
public typealias MiniHostApi = (MiniHostCall, MiniHostReply) -> Void
