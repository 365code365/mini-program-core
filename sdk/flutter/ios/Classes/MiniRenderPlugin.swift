import Flutter
import UIKit

/// Flutter 插件入口：通道 `dev.minirender/engine`（与 Android 的 MiniRenderPlugin.kt 同一套方法）。
///
/// 一个进程同一时刻只有一个小程序实例（引擎的小程序根目录、数据目录、网络回包队列都是
/// 进程级的），所以只持有一个 EngineHost；热重启后 Dart 先调 `init` 把旧实例清掉。
public class MiniRenderPlugin: NSObject, FlutterPlugin {
    private let channel: FlutterMethodChannel
    private let textures: FlutterTextureRegistry
    private var host: EngineHost?

    init(channel: FlutterMethodChannel, textures: FlutterTextureRegistry) {
        self.channel = channel
        self.textures = textures
    }

    public static func register(with registrar: FlutterPluginRegistrar) {
        let channel = FlutterMethodChannel(name: "dev.minirender/engine", binaryMessenger: registrar.messenger())
        let instance = MiniRenderPlugin(channel: channel, textures: registrar.textures())
        registrar.addMethodCallDelegate(instance, channel: channel)
    }

    public func detachFromEngine(for registrar: FlutterPluginRegistrar) {
        host?.dispose()
        host = nil
    }

    public func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
        switch call.method {
        case "init", "dispose":
            host?.dispose()
            host = nil
            result(nil)
        case "paths":
            let fm = FileManager.default
            let files = fm.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            let cache = fm.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            try? fm.createDirectory(at: files, withIntermediateDirectories: true)
            result(["files": files.path, "cache": cache.path, "external": ""])
        case "create":
            create(call, result: result)
        default:
            if let h = host {
                h.handle(call, result: result)
            } else {
                result(call.method == "back" ? false : nil)
            }
        }
    }

    private func create(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
        if host != nil {
            result(FlutterError(code: "ALREADY_RUNNING", message: "同一进程同时只能有一个小程序（先 dispose 上一个）", details: nil))
            return
        }
        let a = call.arguments as? [String: Any] ?? [:]
        guard let appDir = a["appDir"] as? String, !appDir.isEmpty else {
            result(FlutterError(code: "BAD_ARGS", message: "appDir 不能为空", details: nil))
            return
        }
        // 数据目录必须给：storage 与图片磁盘缓存都落在这里，不给会静默失效
        let dataDir = (a["dataDir"] as? String) ?? {
            let d = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
                .appendingPathComponent("mini-render")
            try? FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
            return d.path
        }()
        let channel = self.channel
        let h = EngineHost(textures: textures, onEvents: { channel.invokeMethod("events", arguments: $0) })
        host = h
        h.create(
            appDir: appDir,
            dataDir: dataDir,
            route: a["route"] as? String,
            width: (a["width"] as? NSNumber)?.intValue ?? 1,
            height: (a["height"] as? NSNumber)?.intValue ?? 1,
            dpr: (a["dpr"] as? NSNumber)?.floatValue ?? 1,
            options: Self.withDeviceFields((a["options"] as? String) ?? "{}"),
            result: result,
            onFailed: { [weak self, weak h] in
                if let self, let h, self.host === h { self.host = nil }
            }
        )
    }

    /// 系统信息里宿主没给的设备字段在这里补（与原生 MiniProgramView 同一组）。不补的话
    /// `wx.getSystemInfoSync().system` 落到逻辑层的缺省 `iOS 15.0`，与真机版本对不上。宿主给了的不覆盖
    static func withDeviceFields(_ options: String) -> String {
        guard let data = options.data(using: .utf8),
              var o = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else { return options }
        var si = o["systemInfo"] as? [String: Any] ?? [:]
        if si["system"] == nil { si["system"] = "iOS \(UIDevice.current.systemVersion)" }
        if si["model"] == nil { si["model"] = UIDevice.current.model }
        o["systemInfo"] = si
        guard let out = try? JSONSerialization.data(withJSONObject: o) else { return options }
        return String(data: out, encoding: .utf8) ?? options
    }
}
