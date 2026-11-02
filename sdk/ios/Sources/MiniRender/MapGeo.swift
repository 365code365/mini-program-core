import CoreGraphics
import Foundation
import UIKit

/// [MiniMapView] 的数据与投影：Web 墨卡托（256 逻辑像素一张瓦片），属性按官方字段解析。
/// 与 Android `MapGeo.kt` 同一套，改一边对照另一边。坐标原样用（微信 map 与默认的高德瓦片都是 GCJ-02）。
enum MapGeo {
    static let tile = 256.0

    static func x(_ lng: Double, _ z: Double) -> Double { (lng + 180) / 360 * tile * pow(2, z) }

    static func y(_ lat: Double, _ z: Double) -> Double {
        let r = min(max(lat, -85.05112878), 85.05112878) * .pi / 180
        return (1 - log(tan(r) + 1 / cos(r)) / .pi) / 2 * tile * pow(2, z)
    }

    static func lng(_ x: Double, _ z: Double) -> Double { x / (tile * pow(2, z)) * 360 - 180 }

    static func lat(_ y: Double, _ z: Double) -> Double {
        let n = Double.pi - 2 * .pi * y / (tile * pow(2, z))
        return atan(0.5 * (exp(n) - exp(-n))) * 180 / .pi
    }

    static func metersPerPx(_ lat: Double, _ z: Double) -> Double { 156543.03392 * cos(lat * .pi / 180) / pow(2, z) }

    /// `#RRGGBB` / `#RRGGBBAA`（透明度在后）/ `#RGB`
    static func color(_ s: Any?, _ def: UIColor) -> UIColor {
        guard var t = (s as? String)?.trimmingCharacters(in: .whitespaces), !t.isEmpty else { return def }
        if t.hasPrefix("#") { t.removeFirst() }
        guard let v = UInt64(t, radix: 16) else { return def }
        func c(_ x: UInt64) -> CGFloat { CGFloat(x) / 255 }
        switch t.count {
        case 3: return UIColor(red: c((v >> 8 & 0xF) * 17), green: c((v >> 4 & 0xF) * 17), blue: c((v & 0xF) * 17), alpha: 1)
        case 6: return UIColor(red: c(v >> 16 & 0xFF), green: c(v >> 8 & 0xFF), blue: c(v & 0xFF), alpha: 1)
        case 8: return UIColor(red: c(v >> 24 & 0xFF), green: c(v >> 16 & 0xFF), blue: c(v >> 8 & 0xFF), alpha: c(v & 0xFF))
        default: return def
        }
    }

    /// 属性里的 JSON 数组（引擎给原生组件的数组属性是标准 JSON）
    static func array(_ attrs: [String: Any], _ k: String) -> [[String: Any]] {
        guard let raw = (attrs[k] as? String)?.trimmingCharacters(in: .whitespaces), raw.hasPrefix("["),
              let d = raw.data(using: .utf8), let a = try? JSONSerialization.jsonObject(with: d) as? [Any] else { return [] }
        return a.compactMap { $0 as? [String: Any] }
    }

    static func points(_ a: Any?) -> [LatLng] {
        (a as? [Any] ?? []).compactMap { o in
            guard let o = o as? [String: Any] else { return nil }
            let p = LatLng(num(o["latitude"]) ?? .nan, num(o["longitude"]) ?? .nan)
            return p.valid ? p : nil
        }
    }

    static func flag(_ attrs: [String: Any], _ k: String, _ def: Bool) -> Bool {
        guard let v = attrs[k] else { return def }
        return !["false", "0", "{{false}}"].contains("\(v)".trimmingCharacters(in: .whitespaces))
    }

    static func num(_ v: Any?) -> Double? {
        if let n = v as? NSNumber { return n.doubleValue }
        if let s = v as? String { return Double(s.trimmingCharacters(in: .whitespaces)) }
        return nil
    }
}

struct LatLng {
    var lat: Double
    var lng: Double
    init(_ lat: Double, _ lng: Double) { self.lat = lat; self.lng = lng }
    var valid: Bool { !lat.isNaN && !lng.isNaN && (-90...90).contains(lat) && (-180...180).contains(lng) }
    var json: [String: Any] { ["latitude": lat, "longitude": lng] }
}

/// 气泡（callout）或标签（label）
struct MapBubble {
    let content: String
    let color: UIColor
    let bg: UIColor
    let fontSize: CGFloat
    let padding: CGFloat
    let radius: CGFloat
    let borderWidth: CGFloat
    let borderColor: UIColor
    let always: Bool
    let anchorX: CGFloat
    let anchorY: CGFloat

    init(_ o: [String: Any]) {
        content = "\(o["content"] ?? "")"
        color = MapGeo.color(o["color"], .black)
        bg = MapGeo.color(o["bgColor"], .white)
        fontSize = CGFloat(MapGeo.num(o["fontSize"]) ?? 14)
        padding = CGFloat(MapGeo.num(o["padding"]) ?? 0)
        radius = CGFloat(MapGeo.num(o["borderRadius"]) ?? 0)
        borderWidth = CGFloat(MapGeo.num(o["borderWidth"]) ?? 0)
        borderColor = MapGeo.color(o["borderColor"], .clear)
        always = ((o["display"] as? String) ?? "BYCLICK").uppercased() == "ALWAYS"
        anchorX = CGFloat(MapGeo.num(o["anchorX"]) ?? 0)
        anchorY = CGFloat(MapGeo.num(o["anchorY"]) ?? 0)
    }
}

final class MapMarker {
    let id: Int64
    var pos: LatLng
    let iconPath: String
    let width: CGFloat?
    let height: CGFloat?
    var rotate: CGFloat
    let alpha: CGFloat
    let anchorX: CGFloat
    let anchorY: CGFloat
    let callout: MapBubble?
    let label: MapBubble?
    let zIndex: Int

    init?(_ o: [String: Any]) {
        id = Int64(MapGeo.num(o["id"]) ?? 0)
        pos = LatLng(MapGeo.num(o["latitude"]) ?? .nan, MapGeo.num(o["longitude"]) ?? .nan)
        guard pos.valid else { return nil }
        iconPath = o["iconPath"] as? String ?? ""
        width = MapGeo.num(o["width"]).map { CGFloat($0) }
        height = MapGeo.num(o["height"]).map { CGFloat($0) }
        rotate = CGFloat(MapGeo.num(o["rotate"]) ?? 0)
        alpha = CGFloat(min(max(MapGeo.num(o["alpha"]) ?? 1, 0), 1))
        let a = o["anchor"] as? [String: Any]
        anchorX = CGFloat(MapGeo.num(a?["x"]) ?? 0.5)
        anchorY = CGFloat(MapGeo.num(a?["y"]) ?? 1)
        callout = (o["callout"] as? [String: Any]).map(MapBubble.init)
        label = (o["label"] as? [String: Any]).map(MapBubble.init)
        zIndex = Int(MapGeo.num(o["zIndex"]) ?? 0)
    }
}

struct MapLine {
    let points: [LatLng]
    let color: UIColor
    let width: CGFloat
    let dotted: Bool
    let arrow: Bool
    let borderColor: UIColor
    let borderWidth: CGFloat
    init(_ o: [String: Any]) {
        points = MapGeo.points(o["points"])
        color = MapGeo.color(o["color"], .black)
        width = CGFloat(MapGeo.num(o["width"]) ?? 2)
        dotted = (o["dottedLine"] as? Bool) ?? false
        arrow = (o["arrowLine"] as? Bool) ?? false
        borderColor = MapGeo.color(o["borderColor"], .clear)
        borderWidth = CGFloat(MapGeo.num(o["borderWidth"]) ?? 0)
    }
}

struct MapCircle {
    let center: LatLng
    let radius: Double
    let color: UIColor
    let fill: UIColor
    let stroke: CGFloat
    init(_ o: [String: Any]) {
        center = LatLng(MapGeo.num(o["latitude"]) ?? .nan, MapGeo.num(o["longitude"]) ?? .nan)
        radius = MapGeo.num(o["radius"]) ?? 0
        color = MapGeo.color(o["color"], .clear)
        fill = MapGeo.color(o["fillColor"], .clear)
        stroke = CGFloat(MapGeo.num(o["strokeWidth"]) ?? 0)
    }
}

struct MapPolygon {
    let points: [LatLng]
    let stroke: CGFloat
    let strokeColor: UIColor
    let fill: UIColor
    init(_ o: [String: Any]) {
        points = MapGeo.points(o["points"])
        stroke = CGFloat(MapGeo.num(o["strokeWidth"]) ?? 0)
        strokeColor = MapGeo.color(o["strokeColor"], .clear)
        fill = MapGeo.color(o["fillColor"], .clear)
    }
}

/// 上一帧画出来的一个可点的东西
struct MapHit {
    enum Kind { case marker, callout, label }
    let kind: Kind
    let id: Int64
    let rect: CGRect
}

/// 瓦片源。默认高德公开瓦片（GCJ-02）；换源改 [MiniMapView.tileSource]
public struct MiniMapTileSource {
    public let road: String
    public let satellite: String
    public let maxZoom: Int
    public let attribution: String

    public init(road: String, satellite: String, maxZoom: Int = 18, attribution: String = "") {
        self.road = road
        self.satellite = satellite
        self.maxZoom = maxZoom
        self.attribution = attribution
    }

    func url(_ x: Int, _ y: Int, _ z: Int, _ sat: Bool) -> URL? {
        let s = ((x + y) % 4 + 4) % 4 + 1
        let t = (sat ? satellite : road).replacingOccurrences(of: "{s}", with: "\(s)")
            .replacingOccurrences(of: "{x}", with: "\(x)").replacingOccurrences(of: "{y}", with: "\(y)")
            .replacingOccurrences(of: "{z}", with: "\(z)")
        return URL(string: t)
    }

    public static let autonavi = MiniMapTileSource(
        road: "https://webrd0{s}.is.autonavi.com/appmaptile?lang=zh_cn&size=1&scale=1&style=8&x={x}&y={y}&z={z}",
        satellite: "https://webst0{s}.is.autonavi.com/appmaptile?style=6&x={x}&y={y}&z={z}",
        attribution: "© 高德地图")
}

/// 瓦片：内存缓存 + 磁盘缓存（Caches/mini-map-tiles），URLSession 下载，下好了回主线程通知重画
final class MapTiles {
    private let disk: URL
    private let mem = NSCache<NSString, UIImage>()
    private var loading = Set<String>()
    private var failed: [String: Date] = [:]
    var onLoaded: (() -> Void)?
    private static let session: URLSession = {
        let c = URLSessionConfiguration.default
        c.httpMaximumConnectionsPerHost = 4
        c.timeoutIntervalForRequest = 8
        return URLSession(configuration: c)
    }()

    init() {
        disk = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0].appendingPathComponent("mini-map-tiles")
        try? FileManager.default.createDirectory(at: disk, withIntermediateDirectories: true)
        mem.totalCostLimit = 48 * 1024 * 1024
    }

    private func key(_ x: Int, _ y: Int, _ z: Int, _ sat: Bool) -> String {
        let n = 1 << z
        return "\(sat ? "s" : "r")_\(z)_\(((x % n) + n) % n)_\(y)"
    }

    /// 主线程：有就给，没有就排下载
    func get(_ x: Int, _ y: Int, _ z: Int, _ sat: Bool) -> UIImage? {
        let n = 1 << z
        if y < 0 || y >= n { return nil }
        let k = key(x, y, z, sat)
        if let img = mem.object(forKey: k as NSString) { return img }
        if loading.contains(k) { return nil }
        if let t = failed[k], Date().timeIntervalSince(t) < 5 { return nil }
        loading.insert(k)
        let file = disk.appendingPathComponent(k + ".img")
        let url = MiniMapView.tileSource.url(((x % n) + n) % n, y, z, sat)
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            if let d = try? Data(contentsOf: file), let img = Self.decode(d) {
                DispatchQueue.main.async { self?.done(k, img) }
                return
            }
            guard let url else { DispatchQueue.main.async { self?.done(k, nil) }; return }
            Self.session.dataTask(with: url) { data, resp, _ in
                var img: UIImage?
                if let data, (resp as? HTTPURLResponse)?.statusCode == 200, let i = Self.decode(data) {
                    img = i
                    try? data.write(to: file)
                }
                DispatchQueue.main.async { self?.done(k, img) }
            }.resume()
        }
        return nil
    }

    /// 解码成位图（后台线程上把 PNG 解开：主线程上画的时候不再解）
    static func decode(_ d: Data) -> UIImage? {
        guard let img = UIImage(data: d), let cg = img.cgImage else { return nil }
        let w = cg.width, h = cg.height
        guard let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                  bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue) else { return img }
        ctx.draw(cg, in: CGRect(x: 0, y: 0, width: w, height: h))
        return ctx.makeImage().map { UIImage(cgImage: $0) } ?? img
    }

    private func done(_ k: String, _ img: UIImage?) {
        loading.remove(k)
        if let img {
            mem.setObject(img, forKey: k as NSString, cost: (img.cgImage?.bytesPerRow ?? 1024) * (img.cgImage?.height ?? 256))
            onLoaded?()
        } else {
            failed[k] = Date()
        }
    }

    /// 同一级没下好时拿上几级里覆盖它的那块顶着：(瓦片, 往上几级)
    func parent(_ x: Int, _ y: Int, _ z: Int, _ sat: Bool) -> (UIImage, Int)? {
        var zz = z, xx = x, yy = y, d = 0
        while zz > 0 && d < 4 {
            zz -= 1; d += 1
            xx = Int(floor(Double(xx) / 2)); yy = Int(floor(Double(yy) / 2))
            if let img = mem.object(forKey: key(xx, yy, zz, sat) as NSString) { return (img, d) }
        }
        return nil
    }

    // ── 标记图标 ──
    private static let icons = NSCache<NSString, UIImage>()
    private static var iconLoading = Set<String>()

    static func icon(_ path: String, file: URL?, done: @escaping () -> Void) -> UIImage? {
        if let i = icons.object(forKey: path as NSString) { return i }
        if file == nil && !path.hasPrefix("http") { return nil }
        if iconLoading.contains(path) { return nil }
        iconLoading.insert(path)
        DispatchQueue.global(qos: .userInitiated).async {
            let data = file.flatMap { try? Data(contentsOf: $0) } ?? URL(string: path).flatMap { try? Data(contentsOf: $0) }
            let img = data.flatMap { UIImage(data: $0) }
            DispatchQueue.main.async {
                iconLoading.remove(path)
                if let img { icons.setObject(img, forKey: path as NSString); done() }
            }
        }
        return nil
    }
}
