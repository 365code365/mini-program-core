import CoreGraphics
import Foundation
import QuartzCore
import UIKit

/// SDK 自带的 `<map>`：瓦片地图 + 标记 / 气泡 / 标签 / 折线 / 圆 / 多边形 / 比例尺，**同层渲染**（画进位图交给引擎）。
/// 手势、事件、MapContext 与 Android 的 `MiniMapView.kt` 同一套（改一边对照另一边）：
/// - 单指拖动、惯性、双指捏合缩放、双击放大；`enable-scroll` / `enable-zoom` 都关的不接手势；
/// - `tap {latitude, longitude}`、`markertap` / `callouttap` / `labeltap {markerId}`、`regionchange`（begin / end + causedBy）、`updated`；
/// - 没做：定位蓝点、旋转 / 倾斜、路况、个性化样式、点聚合。
/// 为什么不用 MKMapView：它是 Metal 画的 UIView，离屏截不到画面，只能盖在引擎画面上 —— 一滑页面就跟页面错开。
public final class MiniMapView: MiniSameLayerView {
    /// 瓦片源（全局，建第一张地图之前设）
    public static var tileSource = MiniMapTileSource.autonavi

    var centerLat = 39.908823
    var centerLng = 116.39747
    var zoom = 16.0
    var minZoom = 3.0
    var maxZoom = 20.0
    var satellite = false
    var showScale = false
    var offsetX = 0.5
    var offsetY = 0.5
    var markers: [MapMarker] = []
    var extraMarkers: [MapMarker] = []
    var lines: [MapLine] = []
    var circles: [MapCircle] = []
    var polygons: [MapPolygon] = []
    var openCallouts = Set<Int64>()
    var hits: [MapHit] = []
    var missing = true
    var lastDrawZoom = Double.nan
    lazy var tiles: MapTiles = {
        let t = MapTiles()
        t.onLoaded = { [weak self] in self?.invalidate() }
        return t
    }()
    private var gestures = true
    private var prev: [String: String] = [:]
    private var updatedSent = false

    public override init() { super.init() }

    public override var wantsGestures: Bool { true }
    public override var contextClass: String { "MapContext" }
    public override var contextMethods: [String] { Self.methods }
    static let methods = ["getCenterLocation", "getRegion", "getScale", "getRotate", "getSkew", "moveToLocation", "includePoints",
                          "translateMarker", "moveAlong", "addMarkers", "removeMarkers", "toScreenLocation", "fromScreenLocation",
                          "setCenterOffset", "openMapApp"]

    public override func onAttrs(_ attrs: [String: Any]) {
        let s = { (k: String) -> String in attrs[k].map { "\($0)" } ?? "\u{0}" }
        let changed = { (k: String) -> Bool in s(k) != (self.prev[k] ?? "\u{1}") }
        minZoom = min(max(MapGeo.num(attrs["min-scale"]) ?? 3, 3), 20)
        maxZoom = min(max(MapGeo.num(attrs["max-scale"]) ?? 20, minZoom), 20)
        if changed("latitude") || changed("longitude"),
           let lat = MapGeo.num(attrs["latitude"]), let lng = MapGeo.num(attrs["longitude"]), LatLng(lat, lng).valid {
            centerLat = lat
            centerLng = lng
        }
        if changed("scale") { zoom = MapGeo.num(attrs["scale"]) ?? 16 }
        zoom = min(max(zoom, minZoom), maxZoom)
        satellite = MapGeo.flag(attrs, "enable-satellite", false)
        showScale = MapGeo.flag(attrs, "show-scale", false)
        gestures = MapGeo.flag(attrs, "enable-scroll", true) || MapGeo.flag(attrs, "enable-zoom", true)
        if changed("markers") { markers = MapGeo.array(attrs, "markers").compactMap(MapMarker.init) }
        if changed("polyline") { lines = MapGeo.array(attrs, "polyline").map(MapLine.init) }
        if changed("circles") { circles = MapGeo.array(attrs, "circles").map(MapCircle.init) }
        if changed("polygons") { polygons = MapGeo.array(attrs, "polygons").map(MapPolygon.init) }
        if changed("include-points") {
            let pts = MapGeo.points((MapGeo.array(attrs, "include-points")) as [Any])
            if !pts.isEmpty { fit(pts, pad: [30, 30, 30, 30], animate: false) }
        }
        prev = [:]
        for k in ["latitude", "longitude", "scale", "markers", "polyline", "circles", "polygons", "include-points"] { prev[k] = s(k) }
        invalidate()
    }

    public override func draw(in ctx: CGContext, pixelWidth: Int, pixelHeight: Int, scale: CGFloat) {
        settle?.invalidate()
        if MapDraw.draw(self, ctx, scale: scale) {
            // 缩放中那几帧插值降一档（快）；停下 120ms 后按正常画质再画一遍
            settle = Timer.scheduledTimer(withTimeInterval: 0.12, repeats: false) { [weak self] _ in self?.invalidate() }
        }
        if !missing && !updatedSent { updatedSent = true; emit("updated") }
        if missing { updatedSent = false }
    }
    private var settle: Timer?

    public override func onDetach() { stopAnim() }

    // ───────────────────────── 投影 ─────────────────────────

    func toScreen(_ p: LatLng) -> CGPoint {
        CGPoint(x: MapGeo.x(p.lng, zoom) - MapGeo.x(centerLng, zoom) + Double(width) * offsetX,
                y: MapGeo.y(p.lat, zoom) - MapGeo.y(centerLat, zoom) + Double(height) * offsetY)
    }

    func fromScreen(_ x: CGFloat, _ y: CGFloat) -> LatLng {
        let wx = MapGeo.x(centerLng, zoom) + (Double(x) - Double(width) * offsetX)
        let wy = MapGeo.y(centerLat, zoom) + (Double(y) - Double(height) * offsetY)
        return LatLng(MapGeo.lat(wy, zoom), MapGeo.lng(wx, zoom))
    }

    private func panBy(_ dx: CGFloat, _ dy: CGFloat) {
        let c = fromScreen(width * CGFloat(offsetX) - dx, height * CGFloat(offsetY) - dy)
        centerLat = min(max(c.lat, -85), 85)
        centerLng = (c.lng + 540).truncatingRemainder(dividingBy: 360) - 180
    }

    private func zoomAround(_ z: Double, _ ax: CGFloat, _ ay: CGFloat) {
        let anchor = fromScreen(ax, ay)
        zoom = min(max(z, minZoom), maxZoom)
        let s = toScreen(anchor)
        panBy(ax - s.x, ay - s.y)
    }

    func region() -> [String: Any] {
        ["southwest": fromScreen(0, height).json, "northeast": fromScreen(width, 0).json]
    }

    // ───────────────────────── 事件 ─────────────────────────

    private var regionOpen = false

    private func regionBegin(_ cause: String) {
        if regionOpen { return }
        regionOpen = true
        emit("regionchange", ["type": "begin", "causedBy": cause])
    }

    private func regionEnd(_ cause: String) {
        if !regionOpen { return }
        regionOpen = false
        emit("regionchange", ["type": "end", "causedBy": cause, "rotate": 0, "skew": 0, "scale": (zoom * 100).rounded() / 100,
                              "centerLocation": LatLng(centerLat, centerLng).json, "region": region()])
    }

    public override func onTap(x: CGFloat, y: CGFloat) { tapAt(x, y) }

    private func tapAt(_ x: CGFloat, _ y: CGFloat) {
        let hit = hits.last { $0.rect.contains(CGPoint(x: x, y: y)) }
        switch hit?.kind {
        case .callout?: emit("callouttap", ["markerId": hit!.id])
        case .label?: emit("labeltap", ["markerId": hit!.id])
        case .marker?:
            if let m = (markers + extraMarkers).first(where: { $0.id == hit!.id }), let c = m.callout, !c.always {
                if openCallouts.contains(m.id) { openCallouts.remove(m.id) } else { openCallouts.insert(m.id) }
            }
            emit("markertap", ["markerId": hit!.id])
            invalidate()
        case nil: emit("tap", fromScreen(x, y).json)
        }
    }

    // ───────────────────────── 手势 ─────────────────────────

    private enum G { case idle, pending, drag, pinch }
    private var g = G.idle
    private var p0 = CGPoint.zero, p1 = CGPoint.zero, down = CGPoint.zero
    private var lastTapT = -1000.0
    private var lastTap = CGPoint.zero
    private var pinchDist: CGFloat = 1
    private var pinchZoom = 16.0
    private var pinched = false
    private var samples: [(Double, CGPoint)] = []
    private static let slop: CGFloat = 8

    public override func onTouch(phase: String, pointer: Int, x: CGFloat, y: CGFloat, t: Double) {
        let p = CGPoint(x: x, y: y)
        if !gestures {
            if pointer == 0 && phase == "start" { down = p }
            if pointer == 0 && phase == "end" && hypot(x - down.x, y - down.y) < Self.slop { tapAt(x, y) }
            return
        }
        if pointer == 1 { return onSecond(phase, p) }
        switch phase {
        case "start":
            stopAnim()
            regionEnd("drag")
            g = .pending
            p0 = p; down = p
            pinched = false
            samples = [(t, p)]
        case "move":
            if g == .pending && hypot(x - down.x, y - down.y) > Self.slop {
                g = .drag
                regionBegin("gesture")
            }
            if g == .drag {
                panBy(x - p0.x, y - p0.y)
                invalidate()
            } else if g == .pinch {
                p0 = p
                applyPinch()
                return
            }
            p0 = p
            samples.append((t, p))
            while samples.count > 2 && t - samples[0].0 > 100 { samples.removeFirst() }
        case "end":
            switch g {
            case .pending: if !pinched { tapOrDouble(p, t) }
            case .drag: fling(t)
            case .pinch: regionEnd("scale")
            case .idle: break
            }
            g = .idle
        default:
            regionEnd(pinched ? "scale" : "drag")
            g = .idle
        }
    }

    private func onSecond(_ phase: String, _ p: CGPoint) {
        switch phase {
        case "start":
            if g == .idle { return }
            stopAnim()
            p1 = p
            g = .pinch
            pinched = true
            pinchDist = max(hypot(p1.x - p0.x, p1.y - p0.y), 1)
            pinchZoom = zoom
            regionBegin("gesture")
        case "move":
            if g == .pinch { p1 = p; applyPinch() }
        default:
            if g == .pinch { g = .drag; samples = [] }
        }
    }

    private func applyPinch() {
        let d = max(hypot(p1.x - p0.x, p1.y - p0.y), 1)
        zoomAround(pinchZoom + log2(Double(d / pinchDist)), (p0.x + p1.x) / 2, (p0.y + p1.y) / 2)
        invalidate()
    }

    private func tapOrDouble(_ p: CGPoint, _ t: Double) {
        if t - lastTapT < 300 && hypot(p.x - lastTap.x, p.y - lastTap.y) < 30 {
            lastTapT = -1000
            regionBegin("scale")
            animate(0.3, onEnd: { [weak self] in self?.regionEnd("scale") }) { [weak self] f, from in
                self?.zoomAround(from.zoom + f, p.x, p.y)
            }
            return
        }
        lastTapT = t
        lastTap = p
        tapAt(p.x, p.y)
    }

    private func fling(_ t: Double) {
        guard let first = samples.first, let last = samples.last, last.0 - first.0 >= 8, t - last.0 <= 80 else { return regionEnd("drag") }
        let dt = CGFloat(last.0 - first.0)
        let vx = (last.1.x - first.1.x) / dt, vy = (last.1.y - first.1.y) / dt
        if hypot(vx, vy) < 0.25 { return regionEnd("drag") }
        let tau: CGFloat = 325
        var done: CGFloat = 0
        animate(1.1, ease: false, onEnd: { [weak self] in self?.regionEnd("drag") }) { [weak self] f, _ in
            let k = 1 - exp(-CGFloat(f) * 1100 / tau)
            let step = k - done
            done = k
            self?.panBy(vx * tau * step, vy * tau * step)
        }
    }

    // ───────────────────────── 动画（主线程 CADisplayLink） ─────────────────────────

    struct From { let lat: Double; let lng: Double; let zoom: Double }
    private var link: CADisplayLink?
    private var animStart = 0.0
    private var animDur = 0.3
    private var animEase = true
    private var animStep: ((Double, From) -> Void)?
    private var animEnd: (() -> Void)?
    private var animFrom = From(lat: 0, lng: 0, zoom: 0)

    func animate(_ dur: Double, ease: Bool = true, onEnd: @escaping () -> Void = {}, step: @escaping (Double, From) -> Void) {
        stopAnim()
        animFrom = From(lat: centerLat, lng: centerLng, zoom: zoom)
        animStart = CACurrentMediaTime()
        animDur = dur
        animEase = ease
        animStep = step
        animEnd = onEnd
        let l = CADisplayLink(target: self, selector: #selector(tick))
        l.add(to: .main, forMode: .common)
        link = l
    }

    func stopAnim() {
        guard let l = link else { return }
        l.invalidate()
        link = nil
        let e = animEnd
        animEnd = nil
        e?()
    }

    @objc private func tick() {
        let raw = min((CACurrentMediaTime() - animStart) / animDur, 1)
        let f = animEase ? 1 - pow(1 - raw, 2) : raw
        animStep?(f, animFrom)
        invalidate()
        if raw >= 1 { stopAnim() }
    }

    func fit(_ pts: [LatLng], pad: [CGFloat], animate: Bool, done: @escaping () -> Void = {}) {
        guard !pts.isEmpty, width > 0, height > 0 else { return done() }
        let minLat = pts.map(\.lat).min()!, maxLat = pts.map(\.lat).max()!
        let minLng = pts.map(\.lng).min()!, maxLng = pts.map(\.lng).max()!
        let aw = Double(max(width - pad[1] - pad[3], 20)), ah = Double(max(height - pad[0] - pad[2], 20))
        let dx = max(abs(MapGeo.x(maxLng, 0) - MapGeo.x(minLng, 0)), 1e-9)
        let dy = max(abs(MapGeo.y(minLat, 0) - MapGeo.y(maxLat, 0)), 1e-9)
        let z = min(max(min(log2(aw / dx), log2(ah / dy)), minZoom), min(maxZoom, 18))
        let cx = (MapGeo.x(minLng, z) + MapGeo.x(maxLng, z)) / 2 + Double(pad[1] - pad[3]) / 2
        let cy = (MapGeo.y(maxLat, z) + MapGeo.y(minLat, z)) / 2 + Double(pad[2] - pad[0]) / 2
        moveTo(LatLng(MapGeo.lat(cy, z), MapGeo.lng(cx, z)), z, animate: animate, done: done)
    }

    func moveTo(_ t: LatLng, _ z: Double, animate: Bool, done: @escaping () -> Void = {}) {
        if !animate {
            centerLat = t.lat; centerLng = t.lng; zoom = min(max(z, minZoom), maxZoom)
            invalidate()
            return done()
        }
        regionBegin("update")
        self.animate(0.5, onEnd: { [weak self] in self?.regionEnd("update"); done() }) { [weak self] f, from in
            guard let s = self else { return }
            s.centerLat = from.lat + (t.lat - from.lat) * f
            s.centerLng = from.lng + (t.lng - from.lng) * f
            s.zoom = min(max(from.zoom + (z - from.zoom) * f, s.minZoom), s.maxZoom)
        }
    }

    public override func onContextCall(_ method: String, args: [String: Any], reply: MiniHostReply) -> Bool {
        MapContextCalls.handle(self, method, args, reply)
    }
}

/// `wx.createMapContext(id)` 的方法（与 Android `MapContextCalls.kt` 同一套回包字段）
enum MapContextCalls {
    static func handle(_ m: MiniMapView, _ method: String, _ a: [String: Any], _ reply: MiniHostReply) -> Bool {
        switch method {
        case "getCenterLocation": reply.success(LatLng(m.centerLat, m.centerLng).json)
        case "getScale": reply.success(["scale": (m.zoom * 100).rounded() / 100])
        case "getRegion": reply.success(m.region())
        case "getRotate": reply.success(["rotate": 0])
        case "getSkew": reply.success(["skew": 0])
        case "moveToLocation":
            var t = LatLng(MapGeo.num(a["latitude"]) ?? .nan, MapGeo.num(a["longitude"]) ?? .nan)
            if !t.valid { t = LatLng(MapGeo.num(m.attrs["latitude"]) ?? .nan, MapGeo.num(m.attrs["longitude"]) ?? .nan) }
            guard t.valid else { reply.fail("moveToLocation:fail 没有可挪去的位置"); return true }
            m.moveTo(t, m.zoom, animate: true)
            reply.success()
        case "includePoints":
            let pts = MapGeo.points(a["points"])
            guard !pts.isEmpty else { reply.fail("includePoints:fail points 为空"); return true }
            let p = (a["padding"] as? [Any] ?? []).compactMap { MapGeo.num($0) }.map { CGFloat($0) }
            let pad: [CGFloat] = p.count >= 4 ? Array(p.prefix(4)) : Array(repeating: p.first ?? 0, count: 4)
            m.fit(pts, pad: pad, animate: true)
            reply.success()
        case "translateMarker", "moveAlong":
            let id = Int64(MapGeo.num(a["markerId"]) ?? -1)
            guard let mk = (m.extraMarkers + m.markers).first(where: { $0.id == id }) else {
                reply.fail("\(method):fail 没有这个 markerId"); return true
            }
            var path: [LatLng]
            if method == "translateMarker" {
                let d = a["destination"] as? [String: Any]
                let to = LatLng(MapGeo.num(d?["latitude"]) ?? .nan, MapGeo.num(d?["longitude"]) ?? .nan)
                guard to.valid else { reply.fail("translateMarker:fail destination 不对"); return true }
                if let r = MapGeo.num(a["rotate"]) { mk.rotate = CGFloat(r) }
                path = [mk.pos, to]
            } else {
                path = MapGeo.points(a["path"])
                guard path.count >= 2 else { reply.fail("moveAlong:fail path 至少两个点"); return true }
            }
            slide(m, mk, path, (MapGeo.num(a["duration"]) ?? 1000) / 1000)
            reply.success()
        case "addMarkers":
            if (a["clear"] as? Bool) == true { m.markers = []; m.extraMarkers = [] }
            for o in (a["markers"] as? [[String: Any]] ?? []) {
                guard let mk = MapMarker(o) else { continue }
                m.extraMarkers.removeAll { $0.id == mk.id }
                m.extraMarkers.append(mk)
            }
            m.invalidate()
            reply.success()
        case "removeMarkers":
            let ids = Set((a["markerIds"] as? [Any] ?? []).compactMap { MapGeo.num($0) }.map { Int64($0) })
            m.extraMarkers.removeAll { ids.contains($0.id) }
            m.markers.removeAll { ids.contains($0.id) }
            m.invalidate()
            reply.success()
        case "toScreenLocation":
            let p = m.toScreen(LatLng(MapGeo.num(a["latitude"]) ?? .nan, MapGeo.num(a["longitude"]) ?? .nan))
            reply.success(["x": Double(p.x), "y": Double(p.y)])
        case "fromScreenLocation":
            reply.success(m.fromScreen(CGFloat(MapGeo.num(a["x"]) ?? 0), CGFloat(MapGeo.num(a["y"]) ?? 0)).json)
        case "setCenterOffset":
            let o = (a["offset"] as? [Any] ?? []).compactMap { MapGeo.num($0) }
            m.offsetX = min(max(o.first ?? 0.5, 0.25), 0.75)
            m.offsetY = min(max(o.count > 1 ? o[1] : 0.5, 0.25), 0.75)
            m.invalidate()
            reply.success()
        default: return false
        }
        return true
    }

    /// 标记沿折线匀速走（按段长分配时间）
    private static func slide(_ m: MiniMapView, _ mk: MapMarker, _ path: [LatLng], _ dur: Double) {
        let seg = zip(path, path.dropFirst()).map { hypot($1.lat - $0.lat, $1.lng - $0.lng) }
        let total = seg.reduce(0, +)
        guard total > 0 else { return }
        let start = CACurrentMediaTime()
        Timer.scheduledTimer(withTimeInterval: 1.0 / 60, repeats: true) { timer in
            let f = min((CACurrentMediaTime() - start) / max(dur, 0.016), 1)
            var d = f * total, i = 0
            while i < seg.count - 1 && d > seg[i] { d -= seg[i]; i += 1 }
            let k = seg[i] > 0 ? min(max(d / seg[i], 0), 1) : 1
            mk.pos = LatLng(path[i].lat + (path[i + 1].lat - path[i].lat) * k, path[i].lng + (path[i + 1].lng - path[i].lng) * k)
            m.invalidate()
            if f >= 1 { timer.invalidate() }
        }
    }
}
