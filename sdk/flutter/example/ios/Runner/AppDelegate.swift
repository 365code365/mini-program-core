import Flutter
import UIKit

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
    GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
    if let registrar = engineBridge.pluginRegistry.registrar(forPlugin: "MiniLinks") {
      MiniLinks.register(with: registrar)
    }
  }
}

/// 外部 App / H5 拉起小程序的链接（`minirender://open?appid=…`，Info.plist 的 CFBundleURLTypes 登记了 scheme）
/// 交给 Dart（lib/store/store_links.dart）：冷启动的那条用 `initial` 取，运行中来的推 `link`。
///
/// 走插件的 scene 生命周期回调（`addSceneDelegate`）而不是覆写 FlutterSceneDelegate：
/// 后者的 `scene(_:willConnectTo:)` / `openURLContexts` 没在头文件里声明，Swift 里同名实现会
/// 把 Flutter 自己的那份顶掉
final class MiniLinks: NSObject, FlutterPlugin, FlutterSceneLifeCycleDelegate {
  private let channel: FlutterMethodChannel
  private var initial: String?

  init(channel: FlutterMethodChannel) {
    self.channel = channel
  }

  static func register(with registrar: FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(name: "mini_render_example/links", binaryMessenger: registrar.messenger())
    let instance = MiniLinks(channel: channel)
    registrar.addMethodCallDelegate(instance, channel: channel)
    registrar.addApplicationDelegate(instance)
    registrar.addSceneDelegate(instance)
  }

  func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    if call.method == "initial" {
      result(initial)
      initial = nil
    } else {
      result(FlutterMethodNotImplemented)
    }
  }

  private func accept(_ url: URL) -> Bool {
    guard url.scheme == "minirender" else { return false }
    // Dart 侧还没起来时先存着（冷启动），起来后它会来取
    initial = url.absoluteString
    channel.invokeMethod("link", arguments: url.absoluteString)
    return true
  }

  // 有 scene 的 App（本示例）：冷启动的链接在 connectionOptions 里，运行中的走 openURLContexts
  func scene(_ scene: UIScene, willConnectTo session: UISceneSession,
             options connectionOptions: UIScene.ConnectionOptions?) -> Bool {
    guard let url = connectionOptions?.urlContexts.first?.url else { return false }
    return accept(url)
  }

  func scene(_ scene: UIScene, openURLContexts URLContexts: Set<UIOpenURLContext>) -> Bool {
    guard let url = URLContexts.first?.url else { return false }
    return accept(url)
  }

  // 没有 scene 的老路径
  func application(_ application: UIApplication, open url: URL,
                   options: [UIApplication.OpenURLOptionsKey: Any] = [:]) -> Bool {
    accept(url)
  }
}
