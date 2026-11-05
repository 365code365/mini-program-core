import CoreLocation
import Foundation
import UIKit

/// `<map show-location>` 的定位蓝点（与 Android `MapLocation.kt` 同一套）：
/// - **不申请权限**：宿主在 Info.plist 写了 `NSLocationWhenInUseUsageDescription` 并且用户已经同意（whenInUse / always），
///   才开始定位；没有就不显示蓝点、不报错（微信 3.13.2 起会自动弹授权，这里留给宿主自己弹）。
/// - 系统定位给的是 WGS-84，地图是 GCJ-02：画之前换算（[MapGeo.wgsToGcj]）。方向用 `course`（移动中才有）。
/// - 主线程；地图拿掉 / 关掉 show-location 时 [stop]。
final class MapLocation: NSObject, CLLocationManagerDelegate {
    private(set) var fix: LatLng?
    private(set) var accuracy: Double = 0
    /// 方向（度），没有为 nil
    private(set) var bearing: Double?
    private var manager: CLLocationManager?
    private let changed: () -> Void

    init(_ changed: @escaping () -> Void) { self.changed = changed }

    static var granted: Bool {
        let s: CLAuthorizationStatus
        if #available(iOS 14.0, *) { s = CLLocationManager().authorizationStatus } else { s = CLLocationManager.authorizationStatus() }
        return s == .authorizedWhenInUse || s == .authorizedAlways
    }

    func start() {
        guard manager == nil, Self.granted, CLLocationManager.locationServicesEnabled() else { return }
        let m = CLLocationManager()
        m.delegate = self
        m.desiredAccuracy = kCLLocationAccuracyBest
        m.distanceFilter = 1
        if let l = m.location { take(l) }
        m.startUpdatingLocation()
        manager = m
    }

    func stop() {
        manager?.stopUpdatingLocation()
        manager?.delegate = nil
        manager = nil
    }

    private func take(_ l: CLLocation) {
        fix = MapGeo.wgsToGcj(l.coordinate.latitude, l.coordinate.longitude)
        accuracy = max(l.horizontalAccuracy, 0)
        bearing = l.course >= 0 ? l.course : nil
        changed()
    }

    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        if let l = locations.last { take(l) }
    }

    func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {}

    /// 没开 show-location 时 `moveToLocation()` 用：系统缓存的最后位置（GCJ-02），没权限 / 没有给 nil
    static func lastKnown() -> LatLng? {
        guard granted, let c = CLLocationManager().location?.coordinate else { return nil }
        return MapGeo.wgsToGcj(c.latitude, c.longitude)
    }
}

/// `MapContext.openMapApp {latitude, longitude, destination, preferApplication}`：
/// 指定了 App 且装了就走它的 URL scheme（坐标按 GCJ-02 交：高德 dev=0、百度 coord_type=gcj02、腾讯原样），
/// 否则 / 打不开时退回苹果地图 `maps://?daddr=`。Android 走 `geo:` Intent（`MapContextCalls.kt`）。
enum MapAppLink {
    static func open(_ a: [String: Any], _ reply: MiniHostReply) {
        guard let lat = MapGeo.num(a["latitude"]), let lng = MapGeo.num(a["longitude"]), LatLng(lat, lng).valid else {
            reply.fail("openMapApp:fail latitude / longitude 不对"); return
        }
        let name = (a["destination"] as? String ?? "").addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? ""
        var urls: [String] = []
        switch a["preferApplication"] as? String ?? "" {
        case "amap": urls.append("iosamap://path?sourceApplication=minirender&dlat=\(lat)&dlon=\(lng)&dname=\(name)&dev=0&t=0")
        case "baidu": urls.append("baidumap://map/direction?destination=latlng:\(lat),\(lng)%7Cname:\(name)&coord_type=gcj02&mode=driving")
        case "tencent": urls.append("qqmap://map/routeplan?type=drive&tocoord=\(lat),\(lng)&to=\(name)&referer=minirender")
        case "google": urls.append("comgooglemaps://?daddr=\(lat),\(lng)")
        default: break
        }
        // 苹果地图的 URL 只认目的地坐标（q 会当成搜索词），destination 名称交不过去
        urls.append("maps://?daddr=\(lat),\(lng)")
        attempt(urls.compactMap(URL.init(string:)), reply)
    }

    private static func attempt(_ urls: [URL], _ reply: MiniHostReply) {
        guard let u = urls.first else { reply.fail("openMapApp:fail 没有可用的地图 App"); return }
        UIApplication.shared.open(u, options: [:]) { ok in
            if ok { reply.success() } else { attempt(Array(urls.dropFirst()), reply) }
        }
    }
}

extension MiniMapView {
    /// show-location 开 / 关：开了才定位（有权限才真的开始，没权限每次属性变化重试一次），关了停掉
    func applyShowLocation(_ on: Bool) {
        let was = showLocation
        showLocation = on
        guard on else { if was { location?.stop() }; return }
        let l = location ?? MapLocation { [weak self] in self?.invalidate() }
        location = l
        l.start()
        if !was { NSLog("🗺 show-location：%@", MapLocation.granted ? "有定位权限，开始定位" : "没有定位权限，不显示蓝点") }
    }

    /// polylinetap 的判定：(x, y) 到折线的屏上距离 ≤ 半个线宽 + 边框（至少 8 逻辑像素）
    func onLine(_ l: MapLine, _ x: CGFloat, _ y: CGFloat) -> Bool {
        guard l.points.count >= 2 else { return false }
        let tol = max(8, l.width / 2 + l.borderWidth)
        for (a, b) in zip(l.points, l.points.dropFirst()) {
            let pa = toScreen(a), pb = toScreen(b)
            let dx = pb.x - pa.x, dy = pb.y - pa.y
            let len2 = dx * dx + dy * dy
            let t = len2 > 0 ? min(max(((x - pa.x) * dx + (y - pa.y) * dy) / len2, 0), 1) : 0
            if hypot(x - (pa.x + dx * t), y - (pa.y + dy * t)) <= tol { return true }
        }
        return false
    }
}
