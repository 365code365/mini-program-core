import Foundation

/// `<live-pusher>` 的推流（与 Android `RtmpPublisher.kt` 同一套协议与状态码）：FLV tag 经 RTMP 发出去。
/// 简单握手 → connect → releaseStream / FCPublish / createStream → publish(live) → onMetaData → 两个序列头 → 音视频 tag；
/// 每条消息 fmt 0 块头，块大小 4096；收方向回 ping、认命令回包，其余丢掉。断线 1102 重连最多 3 次，都不行 -1307。
final class RtmpPublisher {
    var onState: ((Int, String) -> Void)?
    var onNetStatus: (([String: Any]) -> Void)?
    var onCongested: (() -> Void)?
    var videoWidth = 0, videoHeight = 0, fps = 15, gop = 2

    private let url: String
    private struct Packet { let video: Bool; let key: Bool; let ts: UInt32; let body: [UInt8] }
    private var queue: [Packet] = []
    private let cond = NSCondition()
    private var running = true
    private var publishing = false
    private var waitKey = true
    private(set) var videoHeader: [UInt8]?
    private(set) var audioHeader: [UInt8]?
    private var thread: Thread?
    private var input: InputStream?
    private var output: OutputStream?
    private let writeLock = NSLock()
    private var inChunk = 128
    private var streamId = 1
    private var chunks: [Int: (ts: UInt32, len: Int, type: Int, buf: [UInt8])] = [:]
    private var vBytes = 0, aBytes = 0, vFrames = 0
    private var statAt = Date()
    private var firstVideo = true

    static let messages: [Int: String] = [
        1001: "已经连接推流服务器", 1002: "已经与服务器握手完毕,开始推流", 1009: "推流：发送视频首帧", 1102: "网络断连, 已启动自动重连",
        3001: "RTMP -DNS解析失败", 3002: "RTMP服务器连接失败", 3003: "RTMP服务器握手失败",
        3004: "RTMP服务器主动断开，请检查推流地址的合法性或防盗链有效期", 3005: "RTMP 读/写失败",
        -1307: "网络断连，且经多次重连抢救无效，更多重试请自行重启推流",
    ]

    init(url: String) { self.url = url }

    func start() {
        let t = Thread { [weak self] in self?.run() }
        t.name = "mini-rtmp"
        thread = t
        t.start()
    }

    func stop() {
        cond.lock()
        running = false
        queue.removeAll()
        cond.broadcast()
        cond.unlock()
        DispatchQueue.global().asyncAfter(deadline: .now() + 1) { [weak self] in
            self?.input?.close()
            self?.output?.close()
        }
    }

    // MARK: 编码器那一侧

    func setVideoHeader(sps: [UInt8], pps: [UInt8]) {
        guard sps.count >= 4 else { return }
        var b: [UInt8] = [0x17, 0, 0, 0, 0, 1, sps[1], sps[2], sps[3], 0xFF, 0xE1]
        b += [UInt8(sps.count >> 8), UInt8(sps.count & 0xFF)] + sps
        b += [1, UInt8(pps.count >> 8), UInt8(pps.count & 0xFF)] + pps
        videoHeader = b
    }

    func setAudioHeader(asc: [UInt8]) { audioHeader = [0xAF, 0] + asc }

    /// 一帧画面（AVCC：每个 NAL 前 4 字节长度），[ts] 毫秒
    func video(avcc: [UInt8], key: Bool, ts: UInt32) {
        guard publishing else { return }
        if waitKey && !key { return }
        waitKey = false
        push(Packet(video: true, key: key, ts: ts, body: [key ? 0x17 : 0x27, 1, 0, 0, 0] + avcc))
    }

    func audio(aac: [UInt8], ts: UInt32) {
        guard publishing else { return }
        push(Packet(video: false, key: false, ts: ts, body: [0xAF, 1] + aac))
    }

    private func push(_ p: Packet) {
        cond.lock()
        queue.append(p)
        if queue.count > 240 {
            queue.removeAll { $0.video }
            waitKey = true
            DispatchQueue.global().async { [weak self] in self?.onCongested?() }
        }
        cond.signal()
        cond.unlock()
    }

    // MARK: 发送线程

    private struct Fail: Error { let code: Int }

    private func run() {
        var attempt = 0
        while running {
            var code = 0
            do { try session(attempt) } catch let f as Fail { code = f.code } catch { code = 3005 }
            publishing = false
            input?.close()
            output?.close()
            if !running { return }
            if code != 0 && attempt == 0 { onState?(code, Self.messages[code] ?? "推流失败") }
            attempt += 1
            if attempt > 3 {
                onState?(-1307, Self.messages[-1307]!)
                running = false
                return
            }
            onState?(1102, Self.messages[1102]!)
            Thread.sleep(forTimeInterval: Double(attempt))
        }
    }

    private func session(_ attempt: Int) throws {
        guard let u = URLComponents(string: url), u.scheme?.lowercased() == "rtmp", let host = u.host else { throw Fail(code: 3004) }
        let port = u.port ?? 1935
        let parts = u.percentEncodedPath.trimmingCharacters(in: CharacterSet(charactersIn: "/")).split(separator: "/", maxSplits: 1).map(String.init)
        guard parts.count == 2 else { throw Fail(code: 3004) }
        let app = parts[0]
        let stream = parts[1] + (u.percentEncodedQuery.map { "?" + $0 } ?? "")
        let tcUrl = "rtmp://\(host)\(u.port != nil ? ":\(port)" : "")/\(app)"
        var i: InputStream?, o: OutputStream?
        Stream.getStreamsToHost(withName: host, port: port, inputStream: &i, outputStream: &o)
        guard let inp = i, let out = o else { throw Fail(code: 3001) }
        inp.open()
        out.open()
        input = inp
        output = out
        // 握手
        var c1 = [UInt8](repeating: 0, count: 1536)
        for k in 8..<1536 { c1[k] = UInt8.random(in: 0...255) }
        try write([3] + c1)
        guard let s0 = try? read(1), s0[0] == 3 else { throw Fail(code: attempt == 0 ? 3002 : 3003) }
        guard let s1 = try? read(1536), (try? read(1536)) != nil else { throw Fail(code: 3003) }
        try write(s1)
        if attempt == 0 { onState?(1001, Self.messages[1001]!) }
        inChunk = 128
        chunks = [:]
        try send(csid: 2, type: 1, stream: 0, ts: 0, [0, 0, 0x10, 0])
        try command(3, 0, "connect", 1, Amf.Obj([("app", app), ("type", "nonprivate"), ("flashVer", "FMLE/3.0 (compatible; FMSc/1.0)"), ("swfUrl", tcUrl), ("tcUrl", tcUrl)]))
        _ = try expectResult(1)
        try command(3, 0, "releaseStream", 2, nil, stream)
        try command(3, 0, "FCPublish", 3, nil, stream)
        try command(3, 0, "createStream", 4, nil)
        let created = try expectResult(4)
        streamId = Int((created.count > 3 ? created[3] as? Double : nil) ?? 1)
        try command(8, streamId, "publish", 5, nil, stream, "live")
        while true {
            let r = try readCommand()
            guard let name = r.first as? String else { continue }
            if name == "onStatus" {
                let code = (r.count > 3 ? r[3] as? [String: Any] : nil)?["code"] as? String ?? ""
                if code == "NetStream.Publish.Start" { break }
                throw Fail(code: 3004)
            }
            if name == "_error" { throw Fail(code: 3004) }
        }
        try sendMeta()
        if let v = videoHeader { try send(csid: 6, type: 9, stream: streamId, ts: 0, v) }
        if let a = audioHeader { try send(csid: 4, type: 8, stream: streamId, ts: 0, a) }
        waitKey = true
        publishing = true
        onCongested?()
        onState?(1002, Self.messages[1002]!)
        statAt = Date()
        while running {
            cond.lock()
            if queue.isEmpty { _ = cond.wait(until: Date().addingTimeInterval(0.2)) }
            let batch = queue
            queue.removeAll()
            cond.unlock()
            for p in batch {
                try send(csid: p.video ? 6 : 4, type: p.video ? 9 : 8, stream: streamId, ts: p.ts, p.body)
                if p.video {
                    vBytes += p.body.count
                    vFrames += 1
                    if firstVideo { firstVideo = false; onState?(1009, Self.messages[1009]!) }
                } else {
                    aBytes += p.body.count
                }
            }
            if inp.streamStatus == .error || inp.streamStatus == .atEnd { throw Fail(code: 3004) }
            stats()
        }
        try? command(3, 0, "FCUnpublish", 6, nil, stream)
        try? command(3, 0, "deleteStream", 7, nil, Double(streamId))
    }

    private func stats() {
        let dt = Date().timeIntervalSince(statAt)
        guard dt >= 1 else { return }
        cond.lock()
        let q = queue.count, vq = queue.filter { $0.video }.count
        cond.unlock()
        let vk = Double(vBytes * 8) / dt / 1000, ak = Double(aBytes * 8) / dt / 1000
        onNetStatus?([
            "videoBitrate": Int(vk), "audioBitrate": Int(ak), "videoFPS": Int((Double(vFrames) / dt).rounded()), "videoGOP": gop,
            "netSpeed": Int(vk + ak), "netJitter": 0, "netQualityLevel": q < 10 ? 1 : q < 40 ? 2 : q < 120 ? 3 : 5,
            "videoWidth": videoWidth, "videoHeight": videoHeight, "videoCache": vq, "audioCache": q - vq,
        ])
        statAt = Date()
        vBytes = 0
        aBytes = 0
        vFrames = 0
    }

    // MARK: 块

    private func write(_ b: [UInt8]) throws {
        guard let out = output else { throw Fail(code: 3005) }
        var off = 0
        while off < b.count {
            let n = b[off...].withUnsafeBufferPointer { out.write($0.baseAddress!, maxLength: $0.count) }
            if n <= 0 { throw Fail(code: 3005) }
            off += n
        }
    }

    private func read(_ n: Int) throws -> [UInt8] {
        guard let inp = input else { throw Fail(code: 3005) }
        var b = [UInt8](repeating: 0, count: n)
        var off = 0
        while off < n {
            let k = b.withUnsafeMutableBufferPointer { inp.read($0.baseAddress! + off, maxLength: n - off) }
            if k <= 0 { throw Fail(code: 3005) }
            off += k
        }
        return b
    }

    private func send(csid: Int, type: Int, stream: Int, ts: UInt32, _ payload: [UInt8]) throws {
        writeLock.lock()
        defer { writeLock.unlock() }
        let ext = ts >= 0xFFFFFF
        let t3 = ext ? 0xFFFFFF : ts
        var h: [UInt8] = [UInt8(csid & 0x3F), UInt8((t3 >> 16) & 0xFF), UInt8((t3 >> 8) & 0xFF), UInt8(t3 & 0xFF),
                          UInt8((payload.count >> 16) & 0xFF), UInt8((payload.count >> 8) & 0xFF), UInt8(payload.count & 0xFF), UInt8(type),
                          UInt8(stream & 0xFF), UInt8((stream >> 8) & 0xFF), UInt8((stream >> 16) & 0xFF), UInt8((stream >> 24) & 0xFF)]
        let extB: [UInt8] = [UInt8(ts >> 24), UInt8((ts >> 16) & 0xFF), UInt8((ts >> 8) & 0xFF), UInt8(ts & 0xFF)]
        if ext { h += extB }
        var out = h
        var off = 0
        while off < payload.count {
            if off > 0 {
                out.append(0xC0 | UInt8(csid & 0x3F))
                if ext { out += extB }
            }
            let n = min(4096, payload.count - off)
            out += payload[off..<(off + n)]
            off += n
        }
        try write(out)
    }

    private func command(_ csid: Int, _ stream: Int, _ name: String, _ txn: Double, _ obj: Amf.Obj?, _ args: Any...) throws {
        var b = Amf.encode(name) + Amf.encode(txn) + (obj.map { Amf.encode($0) } ?? [5])
        for a in args { b += Amf.encode(a) }
        try send(csid: csid, type: 20, stream: stream, ts: 0, b)
    }

    private func sendMeta() throws {
        let b = Amf.encode("@setDataFrame") + Amf.encode("onMetaData") + Amf.ecma([
            ("width", Double(videoWidth)), ("height", Double(videoHeight)), ("videocodecid", 7.0), ("audiocodecid", 10.0),
            ("framerate", Double(fps)), ("audiochannels", 1.0), ("stereo", false),
        ])
        try send(csid: 5, type: 18, stream: streamId, ts: 0, b)
    }

    private func readMessage() throws -> (type: Int, payload: [UInt8]) {
        while true {
            let b0 = Int(try read(1)[0])
            let fmt = b0 >> 6
            var csid = b0 & 0x3F
            if csid == 0 { csid = 64 + Int(try read(1)[0]) } else if csid == 1 { let x = try read(2); csid = 64 + Int(x[0]) + Int(x[1]) * 256 }
            var st = chunks[csid] ?? (0, 0, 0, [])
            if fmt <= 2 { let t = try read(3); st.ts = UInt32(t[0]) << 16 | UInt32(t[1]) << 8 | UInt32(t[2]) }
            if fmt <= 1 { let l = try read(4); st.len = Int(l[0]) << 16 | Int(l[1]) << 8 | Int(l[2]); st.type = Int(l[3]) }
            if fmt == 0 { _ = try read(4) }
            if st.ts == 0xFFFFFF { _ = try read(4) }
            let n = min(inChunk, st.len - st.buf.count)
            st.buf += try read(n)
            if st.buf.count < st.len { chunks[csid] = st; continue }
            let m = (st.type, st.buf)
            st.buf = []
            chunks[csid] = st
            if m.0 == 1, m.1.count >= 4 { inChunk = Int(m.1[0] & 0x7F) << 24 | Int(m.1[1]) << 16 | Int(m.1[2]) << 8 | Int(m.1[3]) }
            if m.0 == 4, m.1.count >= 6, m.1[0] == 0, m.1[1] == 6 { try send(csid: 2, type: 4, stream: 0, ts: 0, [0, 7] + m.1[2..<6]) }
            return m
        }
    }

    private func readCommand() throws -> [Any?] {
        while true {
            let m = try readMessage()
            if m.type == 20 { return Amf.decodeAll(m.payload) }
        }
    }

    private func expectResult(_ txn: Double) throws -> [Any?] {
        while true {
            let r = try readCommand()
            guard r.count > 1, (r[1] as? Double) == txn else { continue }
            if (r[0] as? String) == "_result" { return r }
            throw Fail(code: 3004)
        }
    }
}

/// AMF0：推流端用得到的那几种值
enum Amf {
    struct Obj { let kv: [(String, Any)]; init(_ kv: [(String, Any)]) { self.kv = kv } }

    static func str(_ s: String) -> [UInt8] {
        let b = Array(s.utf8)
        return [UInt8(b.count >> 8), UInt8(b.count & 0xFF)] + b
    }

    static func encode(_ v: Any?) -> [UInt8] {
        switch v {
        case nil: return [5]
        case let d as Double: return [0] + withUnsafeBytes(of: d.bitPattern.bigEndian, Array.init)
        case let i as Int: return encode(Double(i))
        case let b as Bool: return [1, b ? 1 : 0]
        case let s as String: return [2] + str(s)
        case let o as Obj: return [3] + o.kv.flatMap { str($0.0) + encode($0.1) } + [0, 0, 9]
        default: return encode("\(v!)")
        }
    }

    static func ecma(_ kv: [(String, Any)]) -> [UInt8] {
        let n = UInt32(kv.count)
        return [8, UInt8(n >> 24), UInt8((n >> 16) & 0xFF), UInt8((n >> 8) & 0xFF), UInt8(n & 0xFF)] + kv.flatMap { str($0.0) + encode($0.1) } + [0, 0, 9]
    }

    static func decodeAll(_ p: [UInt8]) -> [Any?] {
        var i = 0
        var out: [Any?] = []
        // 不能写 `while let v = try? decode(…)`：null 解出来是 nil，会把循环提前停在那里
        while i < p.count {
            do { out.append(try decode(p, &i)) } catch { break }
        }
        return out
    }

    private struct Bad: Error {}

    private static func u16(_ p: [UInt8], _ i: inout Int) throws -> Int {
        guard i + 2 <= p.count else { throw Bad() }
        defer { i += 2 }
        return Int(p[i]) << 8 | Int(p[i + 1])
    }

    private static func s(_ p: [UInt8], _ i: inout Int) throws -> String {
        let n = try u16(p, &i)
        guard i + n <= p.count else { throw Bad() }
        defer { i += n }
        return String(decoding: p[i..<(i + n)], as: UTF8.self)
    }

    private static func decode(_ p: [UInt8], _ i: inout Int) throws -> Any? {
        guard i < p.count else { throw Bad() }
        let t = p[i]
        i += 1
        switch t {
        case 0:
            guard i + 8 <= p.count else { throw Bad() }
            let bits = p[i..<(i + 8)].reduce(UInt64(0)) { $0 << 8 | UInt64($1) }
            i += 8
            return Double(bitPattern: bits)
        case 1: i += 1; return p[i - 1] != 0
        case 2: return try s(p, &i)
        case 3, 8:
            if t == 8 { i += 4 }
            var m: [String: Any] = [:]
            while true {
                let k = try s(p, &i)
                if k.isEmpty, i < p.count, p[i] == 9 { i += 1; break }
                m[k] = try decode(p, &i) ?? NSNull()
            }
            return m
        case 5, 6: return nil
        default: throw Bad()
        }
    }
}
