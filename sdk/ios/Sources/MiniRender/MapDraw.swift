import CoreGraphics
import Foundation
import UIKit

/// [MiniMapView] 画一帧（与 Android `MapDraw.kt` 同一个顺序）：底色 → 瓦片（缺的拿上一级顶着）→ 多边形 → 圆 → 折线 →
/// 标记 → 标签 → 气泡 → 比例尺 → 版权。上下文已是 UIKit 坐标（左上原点）并按 dpr 缩放，下面用逻辑像素。
enum MapDraw {
    /// 瓦片没到时的底色（tools/android-native-align.py 认地图也认这个色）
    static let background = UIColor(red: 0xE8 / 255.0, green: 0xEE / 255.0, blue: 0xF5 / 255.0, alpha: 1)

    /// 返回 true：这一帧在缩放中（插值降一档），调用方过一会儿再画一遍
    static func draw(_ m: MiniMapView, _ ctx: CGContext, scale: CGFloat) -> Bool {
        let moving = m.zoom != m.lastDrawZoom && !m.lastDrawZoom.isNaN
        m.lastDrawZoom = m.zoom
        ctx.setFillColor(background.cgColor)
        ctx.fill(CGRect(x: 0, y: 0, width: m.width, height: m.height))
        ctx.interpolationQuality = moving ? .low : .medium
        tiles(m, ctx, scale)
        m.hits.removeAll()
        for p in m.polygons { polygon(m, ctx, p) }
        for c in m.circles { circle(m, ctx, c) }
        for l in m.lines { line(m, ctx, l) }
        let all = (m.markers + m.extraMarkers).sorted { $0.zIndex < $1.zIndex }
        for mk in all { marker(m, ctx, mk) }
        for mk in all { if let b = mk.label { label(m, ctx, mk, b) } }
        for mk in all { if let b = mk.callout, b.always || m.openCallouts.contains(mk.id) { callout(m, ctx, mk, b) } }
        if m.showScale { scaleBar(m, ctx) }
        attribution(m, ctx)
        return moving
    }

    private static func tiles(_ m: MiniMapView, _ ctx: CGContext, _ scale: CGFloat) {
        let src = MiniMapView.tileSource
        let tz = min(max(Int(floor(m.zoom + log2(Double(scale)))), 0), src.maxZoom)
        let size = MapGeo.tile * pow(2, m.zoom - Double(tz))
        let cx = MapGeo.x(m.centerLng, m.zoom) - Double(m.width) * m.offsetX
        let cy = MapGeo.y(m.centerLat, m.zoom) - Double(m.height) * m.offsetY
        let x0 = Int(floor(cx / size)), y0 = Int(floor(cy / size))
        let x1 = Int(floor((cx + Double(m.width)) / size)), y1 = Int(floor((cy + Double(m.height)) / size))
        // 按设备像素取整摆放，多画半个逻辑像素盖住接缝
        let snap = { (v: Double) -> CGFloat in (CGFloat(v) * scale).rounded() / scale }
        var missing = false
        for ty in y0...max(y0, y1) {
            for tx in x0...max(x0, x1) {
                let l = snap(Double(tx) * size - cx), t = snap(Double(ty) * size - cy)
                let r = CGRect(x: l, y: t, width: snap(Double(tx + 1) * size - cx) - l + 0.5 / scale, height: snap(Double(ty + 1) * size - cy) - t + 0.5 / scale)
                if let img = m.tiles.get(tx, ty, tz, m.satellite) {
                    img.draw(in: r)
                    continue
                }
                missing = true
                guard let (pb, d) = m.tiles.parent(tx, ty, tz, m.satellite), let cg = pb.cgImage else { continue }
                let n = 1 << d
                let part = cg.width / n
                let ox = ((tx % n) + n) % n * part, oy = ((ty % n) + n) % n * part
                if let sub = cg.cropping(to: CGRect(x: ox, y: oy, width: part, height: part)) { UIImage(cgImage: sub).draw(in: r) }
            }
        }
        m.missing = missing
    }

    private static func path(_ m: MiniMapView, _ pts: [LatLng], close: Bool) -> CGPath {
        let p = CGMutablePath()
        for (i, ll) in pts.enumerated() {
            let s = m.toScreen(ll)
            if i == 0 { p.move(to: s) } else { p.addLine(to: s) }
        }
        if close { p.closeSubpath() }
        return p
    }

    private static func polygon(_ m: MiniMapView, _ c: CGContext, _ g: MapPolygon) {
        guard g.points.count >= 3 else { return }
        let p = path(m, g.points, close: true)
        c.addPath(p); c.setFillColor(g.fill.cgColor); c.fillPath()
        if g.stroke > 0 { c.addPath(p); c.setStrokeColor(g.strokeColor.cgColor); c.setLineWidth(g.stroke); c.strokePath() }
    }

    private static func circle(_ m: MiniMapView, _ c: CGContext, _ k: MapCircle) {
        guard k.center.valid, k.radius > 0 else { return }
        let s = m.toScreen(k.center)
        let r = CGFloat(k.radius / MapGeo.metersPerPx(k.center.lat, m.zoom))
        let rect = CGRect(x: s.x - r, y: s.y - r, width: r * 2, height: r * 2)
        c.setFillColor(k.fill.cgColor); c.fillEllipse(in: rect)
        if k.stroke > 0 { c.setStrokeColor(k.color.cgColor); c.setLineWidth(k.stroke); c.strokeEllipse(in: rect) }
    }

    private static func line(_ m: MiniMapView, _ c: CGContext, _ l: MapLine) {
        guard l.points.count >= 2 else { return }
        let p = path(m, l.points, close: false)
        c.setLineCap(.round); c.setLineJoin(.round)
        if l.borderWidth > 0 {
            c.addPath(p); c.setStrokeColor(l.borderColor.cgColor); c.setLineWidth(l.width + l.borderWidth * 2); c.strokePath()
        }
        c.addPath(p); c.setStrokeColor(l.color.cgColor); c.setLineWidth(l.width)
        if l.dotted { c.setLineDash(phase: 0, lengths: [l.width * 1.5, l.width * 1.5]) }
        c.strokePath()
        c.setLineDash(phase: 0, lengths: [])
        guard l.arrow else { return }
        c.setStrokeColor(UIColor.white.cgColor)
        c.setLineWidth(max(1.2, l.width / 3))
        let s = l.width * 0.6
        for (a, b) in zip(l.points, l.points.dropFirst()) {
            let pa = m.toScreen(a), pb = m.toScreen(b)
            let len = hypot(pb.x - pa.x, pb.y - pa.y)
            let ang = atan2(pb.y - pa.y, pb.x - pa.x)
            var d: CGFloat = 30
            while d < len {
                let x = pa.x + (pb.x - pa.x) * d / len, y = pa.y + (pb.y - pa.y) * d / len
                c.saveGState()
                c.translateBy(x: x, y: y); c.rotate(by: ang)
                c.move(to: CGPoint(x: -s / 2, y: -s)); c.addLine(to: .zero); c.addLine(to: CGPoint(x: -s / 2, y: s))
                c.strokePath()
                c.restoreGState()
                d += 60
            }
        }
    }

    private static func marker(_ m: MiniMapView, _ c: CGContext, _ mk: MapMarker) {
        let s = m.toScreen(mk.pos)
        let icon = mk.iconPath.isEmpty ? nil : MapTiles.icon(mk.iconPath, file: m.resolvePath(mk.iconPath)) { [weak m] in m?.invalidate() }
        let w = mk.width ?? icon.map { $0.size.width / 2 } ?? 28
        let h = mk.height ?? icon.map { $0.size.height / 2 } ?? 36
        let r = CGRect(x: s.x - w * mk.anchorX, y: s.y - h * mk.anchorY, width: w, height: h)
        if r.maxX < -50 || r.minX > m.width + 50 || r.maxY < -50 || r.minY > m.height + 50 { return }
        c.saveGState()
        if mk.rotate != 0 { c.translateBy(x: s.x, y: s.y); c.rotate(by: mk.rotate * .pi / 180); c.translateBy(x: -s.x, y: -s.y) }
        if let icon { icon.draw(in: r, blendMode: .normal, alpha: mk.alpha) } else { pin(c, r, mk.alpha) }
        c.restoreGState()
        m.hits.append(MapHit(kind: .marker, id: mk.id, rect: r.insetBy(dx: -6, dy: -6)))
    }

    /// 默认标记：红色水滴 + 白点
    private static func pin(_ c: CGContext, _ r: CGRect, _ alpha: CGFloat) {
        let cx = r.midX, rad = r.width / 2
        let p = CGMutablePath()
        p.move(to: CGPoint(x: cx, y: r.maxY))
        p.addCurve(to: CGPoint(x: r.minX, y: r.minY + rad), control1: CGPoint(x: cx - rad * 0.3, y: r.maxY - r.height * 0.3), control2: CGPoint(x: r.minX, y: r.minY + rad * 1.4))
        p.addArc(center: CGPoint(x: cx, y: r.minY + rad), radius: rad, startAngle: .pi, endAngle: 0, clockwise: false)
        p.addCurve(to: CGPoint(x: cx, y: r.maxY), control1: CGPoint(x: r.maxX, y: r.minY + rad * 1.4), control2: CGPoint(x: cx + rad * 0.3, y: r.maxY - r.height * 0.3))
        c.addPath(p)
        c.setFillColor(UIColor(red: 0xF5 / 255.0, green: 0x3B / 255.0, blue: 0x3B / 255.0, alpha: alpha).cgColor)
        c.fillPath()
        c.setFillColor(UIColor(white: 1, alpha: alpha).cgColor)
        c.fillEllipse(in: CGRect(x: cx - rad * 0.38, y: r.minY + rad * 0.62, width: rad * 0.76, height: rad * 0.76))
    }

    private static func bubbleSize(_ b: MapBubble) -> (CGSize, [String], [NSAttributedString.Key: Any]) {
        let attrs: [NSAttributedString.Key: Any] = [.font: UIFont.systemFont(ofSize: b.fontSize), .foregroundColor: b.color]
        let lines = b.content.components(separatedBy: "\n")
        let tw = lines.map { ($0 as NSString).size(withAttributes: attrs).width }.max() ?? 0
        let pad = max(b.padding, 4)
        return (CGSize(width: tw + pad * 2, height: b.fontSize * 1.3 * CGFloat(lines.count) + pad * 2), lines, attrs)
    }

    private static func bubble(_ c: CGContext, _ b: MapBubble, _ r: CGRect, _ lines: [String], _ attrs: [NSAttributedString.Key: Any], tail: Bool) {
        c.saveGState()
        c.setShadow(offset: CGSize(width: 0, height: 1), blur: 3, color: UIColor(white: 0, alpha: 0.2).cgColor)
        let shape = UIBezierPath(roundedRect: r, cornerRadius: b.radius)
        if tail {
            shape.move(to: CGPoint(x: r.midX - 6, y: r.maxY - 0.5))
            shape.addLine(to: CGPoint(x: r.midX, y: r.maxY + 6))
            shape.addLine(to: CGPoint(x: r.midX + 6, y: r.maxY - 0.5))
            shape.close()
        }
        b.bg.setFill()
        shape.fill()
        c.restoreGState()
        if b.borderWidth > 0 {
            b.borderColor.setStroke()
            let e = UIBezierPath(roundedRect: r, cornerRadius: b.radius)
            e.lineWidth = b.borderWidth
            e.stroke()
        }
        let pad = max(b.padding, 4)
        for (i, s) in lines.enumerated() {
            (s as NSString).draw(at: CGPoint(x: r.minX + pad, y: r.minY + pad + b.fontSize * 1.3 * CGFloat(i)), withAttributes: attrs)
        }
    }

    private static func callout(_ m: MiniMapView, _ c: CGContext, _ mk: MapMarker, _ b: MapBubble) {
        guard !b.content.isEmpty else { return }
        let s = m.toScreen(mk.pos)
        let h = mk.height ?? 36
        let (size, lines, attrs) = bubbleSize(b)
        let r = CGRect(x: s.x - size.width / 2, y: s.y - h * mk.anchorY - 8 - size.height, width: size.width, height: size.height)
        bubble(c, b, r, lines, attrs, tail: true)
        m.hits.append(MapHit(kind: .callout, id: mk.id, rect: r))
    }

    private static func label(_ m: MiniMapView, _ c: CGContext, _ mk: MapMarker, _ b: MapBubble) {
        guard !b.content.isEmpty else { return }
        let s = m.toScreen(mk.pos)
        let (size, lines, attrs) = bubbleSize(b)
        // 官方：anchorX / anchorY 是标签左上角相对标记锚点的偏移
        let r = CGRect(x: s.x + b.anchorX, y: s.y + b.anchorY, width: size.width, height: size.height)
        bubble(c, b, r, lines, attrs, tail: false)
        m.hits.append(MapHit(kind: .label, id: mk.id, rect: r))
    }

    private static let nice: [Double] = [5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000, 200000, 500000, 1000000]

    private static func scaleBar(_ m: MiniMapView, _ c: CGContext) {
        let mpp = MapGeo.metersPerPx(m.centerLat, m.zoom)
        let meters = nice.last { $0 / mpp <= 90 } ?? nice[0]
        let w = CGFloat(meters / mpp)
        let x: CGFloat = 12, y = m.height - 14
        c.setStrokeColor(UIColor(white: 0.2, alpha: 1).cgColor); c.setLineWidth(2)
        c.move(to: CGPoint(x: x, y: y - 4)); c.addLine(to: CGPoint(x: x, y: y)); c.addLine(to: CGPoint(x: x + w, y: y)); c.addLine(to: CGPoint(x: x + w, y: y - 4))
        c.strokePath()
        let t = meters >= 1000 ? "\(Int(meters / 1000)) 公里" : "\(Int(meters)) 米"
        (t as NSString).draw(at: CGPoint(x: x, y: y - 18), withAttributes: [.font: UIFont.systemFont(ofSize: 11), .foregroundColor: UIColor(white: 0.2, alpha: 1)])
    }

    private static func attribution(_ m: MiniMapView, _ c: CGContext) {
        let s = MiniMapView.tileSource.attribution
        guard !s.isEmpty else { return }
        let attrs: [NSAttributedString.Key: Any] = [.font: UIFont.systemFont(ofSize: 9), .foregroundColor: UIColor(white: 0.2, alpha: 0.6)]
        let w = (s as NSString).size(withAttributes: attrs).width
        (s as NSString).draw(at: CGPoint(x: m.width - w - 6, y: m.height - 15), withAttributes: attrs)
    }
}
