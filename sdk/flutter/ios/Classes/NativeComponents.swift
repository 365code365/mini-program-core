import Flutter
import UIKit
#if canImport(MiniRenderFFI)
import MiniRenderFFI
#endif

/// 插件自带的原生组件（iOS）：与原生 SDK 的 `MiniProgramView` 同一份实现（`Native/` 里是指向 sdk/ios 的软链，
/// 与 Android 插件直接编原生 SDK 的 Kotlin 源码同一个思路）——
/// - 同层渲染：`<map>`（[MiniMapView]）、`<video>` / `<live-player>`（[VideoHost]，系统播放器解码、帧交给引擎）、
///   `<camera>` / `<live-pusher>` / `<voip-room>`（[MiniCameraView] / [MiniLivePusherView]）；
/// - 盖在画面上：`<web-view>`（[MiniWebViews]，WKWebView 放在一层只接 web-view 那几块触摸的容器里，
///   容器的位置由 Dart 报来的小程序画面在屏上的矩形定：`viewRect`）。
///
/// 引擎事件先过这里（主线程）：归这几样的在这里处理并从交给 Dart 的列表里拿掉（与 Android `NativeComponents.kt` 同一张表）。
final class NativeComponents {
    private let run: (@escaping (OpaquePointer) -> Void) -> Void
    private let map: Bool
    private let webView: Bool
    private let video: Bool
    private let cameras: [String]
    private let sameLayer: SameLayerViews
    private lazy var videoHost = VideoHost(run: run)
    private var webViews: MiniWebViews?
    private var container: PassThroughView?
    private var lastPage: [String: Any]?

    init(run: @escaping (@escaping (OpaquePointer) -> Void) -> Void, map: Bool, webView: Bool, video: Bool, cameras: [String]) {
        self.run = run
        self.map = map
        self.webView = webView
        self.video = video
        self.cameras = cameras
        sameLayer = SameLayerViews(run: run, scale: { UIScreen.main.scale })
        if map { sameLayer.factories["map"] = { MiniMapView() } }
        for t in cameras {
            switch t {
            case "camera": sameLayer.factories[t] = { MiniCameraView(tag: "camera") }
            case "voip-room": sameLayer.factories[t] = { MiniCameraView(tag: "voip-room") }
            case "live-pusher": sameLayer.factories[t] = { MiniLivePusherView() }
            default: break
            }
        }
    }

    var active: Bool { map || webView || video || !cameras.isEmpty }

    /// 建引擎的选项里补上自带组件那几项
    func options(_ json: String) -> String {
        guard active, let d = json.data(using: .utf8), var o = (try? JSONSerialization.jsonObject(with: d)) as? [String: Any] else { return json }
        func add(_ k: String, _ v: [String]) {
            var a = o[k] as? [String] ?? []
            for x in v where !a.contains(x) { a.append(x) }
            o[k] = a
        }
        if video { add("sameLayer", ["video", "live-player"]) }
        let tags = sameLayer.tags
        if !tags.isEmpty { add("sameLayer", tags) }
        let gestures = sameLayer.gestureTags
        if !gestures.isEmpty { add("sameLayerGestures", gestures) }
        let apis = sameLayer.contextApis
        if !apis.isEmpty { add("hostApis", apis) }
        if webView { add("nativeComponents", ["web-view"]) }
        guard let out = try? JSONSerialization.data(withJSONObject: o) else { return json }
        return String(data: out, encoding: .utf8) ?? json
    }

    func setAppDir(_ dir: String) { sameLayer.appDir = URL(fileURLWithPath: dir) }

    /// Dart 报来的小程序画面在屏上的位置（逻辑像素 = UIKit 的点）：web-view 的容器跟着它
    func setRect(_ r: CGRect) {
        guard webView else { return }
        let c = ensureContainer()
        c?.frame = r
    }

    private func ensureContainer() -> PassThroughView? {
        if let c = container { return c }
        guard let root = UIApplication.shared.connectedScenes.compactMap({ ($0 as? UIWindowScene)?.windows.first { $0.isKeyWindow } }).first?.rootViewController?.view
            ?? UIApplication.shared.windows.first?.rootViewController?.view
        else { return nil }
        let c = PassThroughView(frame: root.bounds)
        c.backgroundColor = .clear
        root.addSubview(c)
        container = c
        return c
    }

    private func web() -> MiniWebViews? {
        guard webView else { return nil }
        if let w = webViews { return w }
        guard let c = ensureContainer() else { return nil }
        let w = MiniWebViews(parent: c, run: run)
        webViews = w
        if let p = lastPage { w.onPage(p) }
        return w
    }

    /// 主线程：处理归这里的事件，返回剩下交给 Dart 的
    func filter(_ list: [String]) -> [String] {
        guard active else { return list }
        var out: [String] = []
        for raw in list {
            guard let d = raw.data(using: .utf8), let ev = (try? JSONSerialization.jsonObject(with: d)) as? [String: Any],
                  let type = ev["type"] as? String else { out.append(raw); continue }
            switch type {
            case "page":
                sameLayer.route = ev["route"] as? String ?? ""
                lastPage = ev
                webViews?.onPage(ev)
                out.append(raw)
            case "sameLayerViews":
                sameLayer.onViews(ev)
                out.append(strip(ev) { sameLayer.factories[$0["tag"] as? String ?? ""] != nil })
            case "nativeViewTap", "nativeViewTouch":
                if sameLayer.factories[ev["tag"] as? String ?? ""] != nil {
                    if type == "nativeViewTap" { sameLayer.onTap(ev) } else { sameLayer.onTouch(ev) }
                } else { out.append(raw) }
            case "nativeViews":
                if webView {
                    if raw.contains("\"tag\":\"web-view\"") || webViews != nil { web()?.onViews(ev) }
                    out.append(strip(ev) { ($0["tag"] as? String) == "web-view" })
                } else { out.append(raw) }
            case "videos" where video: videoHost.sync(ev)
            case "videoCommand" where video: videoHost.command(ev)
            case "hostCall":
                let api = ev["api"] as? String ?? ""
                let mine = (api.hasPrefix("MapContext.") && map) ||
                    ((api.hasPrefix("CameraContext.") || api.hasPrefix("LivePusherContext.")) && !cameras.isEmpty)
                if mine { contextCall(MiniHostCall(json: ev)) } else { out.append(raw) }
            default: out.append(raw)
            }
        }
        return out
    }

    private func strip(_ ev: [String: Any], _ drop: ([String: Any]) -> Bool) -> String {
        var e = ev
        let views = (ev["views"] as? [[String: Any]] ?? []).filter { !drop($0) }
        e["views"] = views
        guard let d = try? JSONSerialization.data(withJSONObject: e) else { return "{}" }
        return String(data: d, encoding: .utf8) ?? "{}"
    }

    private func contextCall(_ call: MiniHostCall) {
        let id = UInt32(max(0, call.id))
        let reply = MiniHostReply { [run] ok, res in
            let json = (try? JSONSerialization.data(withJSONObject: res)).flatMap { String(data: $0, encoding: .utf8) } ?? "{}"
            run { _ = mr_app_host_reply($0, id, ok ? 1 : 0, json) }
        }
        if !sameLayer.contextCall(call, reply) { reply.fail("\(call.api):fail 宿主没有实现") }
    }

    /// Dart 的 lifecycle：0 回前台 / 1 进后台
    func lifecycle(_ what: Int) {
        switch what {
        case 0: sameLayer.resume()
        case 1:
            if video { videoHost.pauseAll() }
            sameLayer.pause()
        default: break
        }
    }

    func goBack() -> Bool { webViews?.goBack() ?? false }

    func dispose() {
        webViews = nil
        container?.removeFromSuperview()
        container = nil
    }
}

/// 只接子视图（web-view）那几块触摸的容器：别的地方点下去穿到 Flutter 的视图上
final class PassThroughView: UIView {
    override func hitTest(_ point: CGPoint, with event: UIEvent?) -> UIView? {
        let v = super.hitTest(point, with: event)
        return v === self ? nil : v
    }
}

extension NSLock {
    /// `withLock` 要 iOS 16：部署目标是 13（与 sdk/ios EngineSession.swift 里那份相同；插件不带 EngineSession）
    func locked<T>(_ body: () -> T) -> T {
        lock()
        defer { unlock() }
        return body()
    }
}
