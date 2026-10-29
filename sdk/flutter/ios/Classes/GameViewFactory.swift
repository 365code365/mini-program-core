import Flutter
import UIKit

/// 小游戏的平台视图 `dev.minirender/game-view`（与 Android 的 GameView.kt 对应）：里面就是 SDK 的 [MiniGameView]
/// （Classes/Game/ 下是指向 sdk/ios/Sources/MiniRender 的软链，两边同一份源码）。
///
/// 每个视图一条通道 `dev.minirender/game_<viewId>`：
/// - 原生 → Dart：`event {type, json}`、`firstFrame`、`orientation {value}`（游戏要转屏，Dart 用 SystemChrome 转：
///   Flutter 的控制器不认原生改的 supportedInterfaceOrientations）；
/// - Dart → 原生：`hostReply {id, ok, json}`、`dispatchEvent {name, json}`、`eval {js}`。
final class GameViewFactory: NSObject, FlutterPlatformViewFactory {
    private let messenger: FlutterBinaryMessenger

    init(messenger: FlutterBinaryMessenger) {
        self.messenger = messenger
    }

    func createArgsCodec() -> FlutterMessageCodec & NSObjectProtocol { FlutterStandardMessageCodec.sharedInstance() }

    func create(withFrame frame: CGRect, viewIdentifier viewId: Int64, arguments args: Any?) -> FlutterPlatformView {
        let a = args as? [String: Any] ?? [:]
        let dir = a["dir"] as? String ?? ""
        let dataDir = (a["dataDir"] as? String).flatMap { $0.isEmpty ? nil : $0 }
        let options = (a["options"] as? String).flatMap { try? JSONSerialization.jsonObject(with: Data($0.utf8)) } as? [String: Any] ?? [:]
        return GamePlatformView(frame: frame, id: viewId, dir: dir, dataDir: dataDir, options: options, messenger: messenger)
    }
}

final class GamePlatformView: NSObject, FlutterPlatformView {
    private let game: MiniGameView
    private let channel: FlutterMethodChannel

    init(frame: CGRect, id: Int64, dir: String, dataDir: String?, options: [String: Any], messenger: FlutterBinaryMessenger) {
        game = MiniGameView(gameDir: dir, dataDir: dataDir, options: options)
        game.frame = frame
        channel = FlutterMethodChannel(name: "dev.minirender/game_\(id)", binaryMessenger: messenger)
        super.init()
        let ch = channel
        game.onHostEvent = { type, ev in
            let json = (try? JSONSerialization.data(withJSONObject: ev)).map { String(decoding: $0, as: UTF8.self) } ?? "{}"
            ch.invokeMethod("event", arguments: ["type": type, "json": json])
        }
        game.onFirstFrame = { ch.invokeMethod("firstFrame", arguments: nil) }
        game.onOrientationChange = { m in ch.invokeMethod("orientation", arguments: ["value": Self.name(m)]) }
        channel.setMethodCallHandler { [weak self] call, result in self?.handle(call, result) }
    }

    func view() -> UIView { game }

    deinit {
        channel.setMethodCallHandler(nil)
        game.close()
    }

    private static func name(_ m: UIInterfaceOrientationMask) -> String {
        if m == .landscapeLeft { return "landscapeLeft" }
        if m == .landscapeRight { return "landscapeRight" }
        return m == .landscape ? "landscape" : "portrait"
    }

    private func handle(_ call: FlutterMethodCall, _ result: @escaping FlutterResult) {
        let a = call.arguments as? [String: Any] ?? [:]
        switch call.method {
        case "hostReply":
            let json = a["json"] as? String ?? "{}"
            let obj = (try? JSONSerialization.jsonObject(with: Data(json.utf8))) as? [String: Any] ?? [:]
            game.hostReply((a["id"] as? NSNumber)?.intValue ?? 0, ok: a["ok"] as? Bool ?? false, result: obj)
            result(nil)
        case "dispatchEvent":
            game.dispatchEvent(a["name"] as? String ?? "", json: a["json"] as? String ?? "{}")
            result(nil)
        case "eval":
            game.eval(a["js"] as? String ?? "") { result($0) }
        // 建视图时 Dart 还拿不到方向（初始方向在原生读 game.json）：Dart 建好通道就来问一次
        case "orientation":
            result(Self.name(game.orientationMask))
        default:
            result(FlutterMethodNotImplemented)
        }
    }
}
