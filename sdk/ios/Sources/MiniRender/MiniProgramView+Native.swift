import UIKit

/// [MiniProgramView] 的原生组件那一片（与 Android `MiniProgramView.kt` 里同名的几段对照）：
/// - 同层渲染：`<map>`（[MiniMapView]，或宿主在 `sameLayerViews` 里登记的）、`<video>` / `<live-player>`（[VideoHost]）；
/// - 盖在画面上：`<web-view>`（[MiniWebViews]）。
extension MiniProgramView {
    func makeSameLayer() -> SameLayerViews {
        let s = SameLayerViews(run: { [weak self] b in self?.onEngine(b) }, scale: { [weak self] in self?.window?.screen.scale ?? UIScreen.main.scale })
        s.factories["map"] = { MiniMapView() }
        // 相机 / 推流 / voip-room 自己那一格（AVCaptureSession → 同层渲染）。宿主 Info.plist 要有 NSCameraUsageDescription / NSMicrophoneUsageDescription
        s.factories["camera"] = { MiniCameraView(tag: "camera") }
        s.factories["live-pusher"] = { MiniLivePusherView() }
        s.factories["voip-room"] = { MiniCameraView(tag: "voip-room") }
        return s
    }

    /// 建引擎的选项里原生组件那几项
    func nativeOptions(_ options: inout [String: Any]) {
        // 视频 / 直播（VideoHost 解码）、登记了的组件（地图）
        var same = sameLayerVideo ? ["video", "live-player"] : []
        same += sameLayer.tags.filter { $0 != "video" }
        if !same.isEmpty { options["sameLayer"] = same }
        // 自己处理手势的（地图）：按在框里的整串触摸交给它（nativeViewTouch）
        let gestures = sameLayer.gestureTags
        if !gestures.isEmpty { options["sameLayerGestures"] = gestures }
        // web-view 盖在画面上，引擎报位置（nativeViews）
        if webView { options["nativeComponents"] = ["web-view"] }
    }

    /// 主线程：原生组件相关的引擎事件
    func nativeEvent(_ ev: [String: Any]) {
        switch ev["type"] as? String {
        case "page":
            sameLayer.route = ev["route"] as? String ?? ""
            if webView { webViews.onPage(ev) }
        case "nativeViews": if webView { webViews.onViews(ev) }
        case "sameLayerViews": sameLayer.onViews(ev)
        case "nativeViewTap": sameLayer.onTap(ev)
        case "nativeViewTouch": sameLayer.onTouch(ev)
        case "videos": videoHost.sync(ev)
        case "videoCommand": videoHost.command(ev)
        default: break
        }
    }
}
