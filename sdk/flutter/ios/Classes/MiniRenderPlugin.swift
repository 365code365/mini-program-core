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
    /// 插件自带的原生组件（地图 / web-view / 同层视频 / 相机，NativeComponents.swift）
    private var native: NativeComponents?
    /// 插件自带的宿主能力：选图 / 选视频 / 选文件 / 存相册 / 打开文档（MediaPicker.swift）
    private let picker = MediaPicker()

    init(channel: FlutterMethodChannel, textures: FlutterTextureRegistry) {
        self.channel = channel
        self.textures = textures
    }

    public static func register(with registrar: FlutterPluginRegistrar) {
        let channel = FlutterMethodChannel(name: "dev.minirender/engine", binaryMessenger: registrar.messenger())
        let instance = MiniRenderPlugin(channel: channel, textures: registrar.textures())
        registrar.addMethodCallDelegate(instance, channel: channel)
        // 小游戏：平台视图（每个视图一个实例，不受「同一时刻一个小程序」的限制）
        registrar.register(GameViewFactory(messenger: registrar.messenger()), withId: "dev.minirender/game-view")
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
            native?.dispose()
            native = nil
            result(nil)
        case "viewRect":
            // 小程序画面在屏上的位置（Dart 布局完报来）：盖 web-view 的容器跟着它
            let a = call.arguments as? [String: Any] ?? [:]
            func d(_ k: String) -> CGFloat { CGFloat((a[k] as? NSNumber)?.doubleValue ?? 0) }
            native?.setRect(CGRect(x: d("x"), y: d("y"), width: d("width"), height: d("height")))
            result(nil)
        case "lifecycle":
            native?.lifecycle((call.arguments as? [String: Any])?["what"] as? Int ?? -1)
            host?.handle(call, result: result) ?? result(nil)
        case "back":
            // web-view 里的网页先后退
            if native?.goBack() == true { return result(true) }
            host?.handle(call, result: result) ?? result(false)
        case "paths":
            let fm = FileManager.default
            let files = fm.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            let cache = fm.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            try? fm.createDirectory(at: files, withIntermediateDirectories: true)
            result(["files": files.path, "cache": cache.path, "external": ""])
        case "create":
            create(call, result: result)
        // 小游戏下载 / 解包完：后台编字节码（dataDir 空 = MiniGame.defaultDataDir，与视图的默认一致）
        case "precompileGame":
            let a = call.arguments as? [String: Any] ?? [:]
            let data = (a["dataDir"] as? String).flatMap { $0.isEmpty ? nil : $0 }
            MiniGame.precompile(dir: a["dir"] as? String ?? "", dataDir: data)
            result(nil)
        // 不碰引擎实例：文件直接写进 hostCall 给的 tempDir
        case "pickMedia", "pickFiles", "saveToAlbum", "openDocument":
            picker.handle(call, result: result)
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
        var nc: NativeComponents?
        let h = EngineHost(textures: textures, onEvents: { list in
            // 归自带原生组件的事件在这里处理掉，剩下的交给 Dart（主线程）
            let rest = nc?.filter(list) ?? list
            if !rest.isEmpty { channel.invokeMethod("events", arguments: rest) }
        })
        let n = NativeComponents(
            run: { [weak h] b in h?.run(b) },
            map: a["builtinMap"] as? Bool ?? false,
            webView: a["builtinWebView"] as? Bool ?? false,
            video: a["sameLayerVideo"] as? Bool ?? false,
            cameras: a["builtinCamera"] as? [String] ?? [])
        n.setAppDir(appDir)
        nc = n
        native = n
        host = h
        h.create(
            appDir: appDir,
            dataDir: dataDir,
            route: a["route"] as? String,
            width: (a["width"] as? NSNumber)?.intValue ?? 1,
            height: (a["height"] as? NSNumber)?.intValue ?? 1,
            dpr: (a["dpr"] as? NSNumber)?.floatValue ?? 1,
            options: n.options(Self.withDeviceFields((a["options"] as? String) ?? "{}")),
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
