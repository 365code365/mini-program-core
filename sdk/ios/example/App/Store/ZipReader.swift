import Compression
import Foundation

/// 解商店的小程序包（标准 ZIP）。iOS 没有系统级的解 ZIP 接口，又不想为示例拉第三方库：
/// 读中央目录，逐项按「存储（0）/ 压缩（8，raw DEFLATE）」解出来 —— 商店打包（tools/mini-store/store_pkg.py
/// 用的 Python zipfile）只会产出这两种。Compression 框架的 `COMPRESSION_ZLIB` 就是 raw DEFLATE（不带 zlib 头）。
///
/// 路径穿越：服务器已经挡过一次；这里照样再挡（写到沙盒别处），与 Android 示例一致。
enum ZipReader {
    struct ZipError: LocalizedError {
        let message: String
        var errorDescription: String? { message }
    }

    static func extract(_ zip: Data, to dir: URL) throws {
        let bytes = [UInt8](zip)
        let n = bytes.count
        func u16(_ o: Int) -> Int { Int(bytes[o]) | Int(bytes[o + 1]) << 8 }
        func u32(_ o: Int) -> Int { u16(o) | u16(o + 2) << 16 }

        // 中央目录结尾（EOCD）：从尾部往前找签名（尾部可能还有最多 64KB 的注释）
        var eocd = -1
        var i = n - 22
        while i >= max(0, n - 22 - 65535) {
            if u32(i) == 0x0605_4b50 { eocd = i; break }
            i -= 1
        }
        guard eocd >= 0 else { throw ZipError(message: "不是 ZIP 包") }
        let count = u16(eocd + 10)
        var p = u32(eocd + 16)
        let base = dir.standardizedFileURL.path + "/"
        let fm = FileManager.default

        for _ in 0..<count {
            guard p + 46 <= n, u32(p) == 0x0201_4b50 else { throw ZipError(message: "ZIP 中央目录损坏") }
            let method = u16(p + 10)
            let csize = u32(p + 20)
            let usize = u32(p + 24)
            let nameLen = u16(p + 28)
            let extraLen = u16(p + 30)
            let commentLen = u16(p + 32)
            let localOff = u32(p + 42)
            let name = String(decoding: bytes[(p + 46)..<(p + 46 + nameLen)], as: UTF8.self)
            p += 46 + nameLen + extraLen + commentLen
            if name.hasSuffix("/") { continue }

            let out = dir.appendingPathComponent(name).standardizedFileURL
            guard out.path.hasPrefix(base) else { throw ZipError(message: "包里有非法路径：\(name)") }

            // 本地文件头的 name / extra 长度可能与中央目录里的不同，以本地头为准
            guard localOff + 30 <= n, u32(localOff) == 0x0403_4b50 else { throw ZipError(message: "ZIP 文件头损坏：\(name)") }
            let start = localOff + 30 + u16(localOff + 26) + u16(localOff + 28)
            guard start + csize <= n else { throw ZipError(message: "ZIP 数据不完整：\(name)") }
            let raw = Data(bytes[start..<(start + csize)])

            let content: Data
            switch method {
            case 0: content = raw
            case 8: content = try inflate(raw, size: usize, name: name)
            default: throw ZipError(message: "不支持的压缩方式 \(method)：\(name)")
            }
            try fm.createDirectory(at: out.deletingLastPathComponent(), withIntermediateDirectories: true)
            try content.write(to: out)
        }
    }

    private static func inflate(_ src: Data, size: Int, name: String) throws -> Data {
        if size == 0 { return Data() }
        var dst = Data(count: size)
        let written = dst.withUnsafeMutableBytes { d in
            src.withUnsafeBytes { s in
                compression_decode_buffer(d.bindMemory(to: UInt8.self).baseAddress!, size,
                                          s.bindMemory(to: UInt8.self).baseAddress!, src.count, nil, COMPRESSION_ZLIB)
            }
        }
        guard written == size else { throw ZipError(message: "解压失败：\(name)") }
        return dst
    }
}
