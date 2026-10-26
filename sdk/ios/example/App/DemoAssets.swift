import Foundation

/// 把包里的 `demo/` 拷到 Application Support：与 Android 示例的 DemoAssets 同一个约定 ——
/// 小程序目录交给引擎前放在 App 自己可写的地方（真实业务是「下载 → 校验 → 解包」到这里）。
///
/// 只在 App 换过版本之后重拷（按包里 Info.plist 的修改时间记一个标记）：每次冷启都整包重写的话，
/// 首帧要多等，也白白磨闪存。
enum DemoAssets {
    static func unpack() throws -> URL {
        let fm = FileManager.default
        guard let src = Bundle.main.url(forResource: "demo", withExtension: nil) else {
            throw NSError(domain: "DemoAssets", code: 1, userInfo: [NSLocalizedDescriptionKey: "包里没有 demo/（project.yml 的文件夹引用）"])
        }
        let base = try fm.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
        let dst = base.appendingPathComponent("demo", isDirectory: true)
        let stamp = base.appendingPathComponent("demo.stamp")
        let infoPlist = Bundle.main.bundleURL.appendingPathComponent("Info.plist")
        let version = ((try? fm.attributesOfItem(atPath: infoPlist.path)[.modificationDate]) as? Date)
            .map { String($0.timeIntervalSince1970) } ?? "0"
        if fm.fileExists(atPath: dst.path), (try? String(contentsOf: stamp, encoding: .utf8)) == version {
            return dst
        }
        try? fm.removeItem(at: dst)
        try fm.copyItem(at: src, to: dst)
        try version.write(to: stamp, atomically: true, encoding: .utf8)
        return dst
    }
}
