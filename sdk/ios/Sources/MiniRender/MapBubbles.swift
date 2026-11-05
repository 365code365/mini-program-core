import CoreGraphics
import Foundation
import UIKit

/// [MapDraw] 里「贴在地图上的小块」：气泡（callout / title）、标签、自定义气泡（customCallout）、控件（controls）、定位蓝点。
/// 与 Android `MapBubbles.kt` 同一套：气泡 / 标签按 dpr 画成一张图挂在 [MapBubble.image] 上，拖动 / 缩放的每一帧只贴图
/// （从前每帧每个气泡量一遍字、画一遍阴影和字）；控件与自定义气泡本来就是图（[MapTiles.icon] 的缓存）。
enum MapBubbles {
    /// 图四周给阴影留的边（逻辑像素）
    private static let shadow: CGFloat = 4
    private static let tail: CGFloat = 6

    private static func ensure(_ b: MapBubble, _ scale: CGFloat, tail withTail: Bool) -> UIImage? {
        if let img = b.image, b.imageScale == scale { return img }
        let attrs: [NSAttributedString.Key: Any] = [.font: UIFont.systemFont(ofSize: b.fontSize), .foregroundColor: b.color]
        let lines = b.content.components(separatedBy: "\n")
        let widths = lines.map { ($0 as NSString).size(withAttributes: attrs).width }
        let pad = max(b.padding, 4)
        let w = (widths.max() ?? 0) + pad * 2
        let h = b.fontSize * 1.3 * CGFloat(lines.count) + pad * 2
        let size = CGSize(width: w + shadow * 2, height: h + shadow * 2 + (withTail ? tail : 0))
        // 一个气泡最多 2000×2000 设备像素：再大是写错了的内容，不画
        guard size.width * scale * size.height * scale <= 4_000_000 else { return nil }
        let fmt = UIGraphicsImageRendererFormat()
        fmt.scale = scale
        fmt.opaque = false
        let img = UIGraphicsImageRenderer(size: size, format: fmt).image { rc in
            let c = rc.cgContext
            let r = CGRect(x: shadow, y: shadow, width: w, height: h)
            c.saveGState()
            c.setShadow(offset: CGSize(width: 0, height: 1), blur: 3, color: UIColor(white: 0, alpha: 0.2).cgColor)
            let shape = UIBezierPath(roundedRect: r, cornerRadius: b.radius)
            if withTail {
                shape.move(to: CGPoint(x: r.midX - 6, y: r.maxY - 0.5))
                shape.addLine(to: CGPoint(x: r.midX, y: r.maxY + tail))
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
            for (i, s) in lines.enumerated() {
                let x: CGFloat
                switch b.textAlign {
                case "center": x = r.minX + (w - widths[i]) / 2
                case "right": x = r.maxX - pad - widths[i]
                default: x = r.minX + pad
                }
                (s as NSString).draw(at: CGPoint(x: x, y: r.minY + pad + b.fontSize * 1.3 * CGFloat(i)), withAttributes: attrs)
            }
        }
        b.image = img
        b.imageScale = scale
        b.boxSize = CGSize(width: w, height: h)
        return img
    }

    private static func blit(_ img: UIImage, _ origin: CGPoint) {
        img.draw(in: CGRect(x: origin.x - shadow, y: origin.y - shadow, width: img.size.width, height: img.size.height))
    }

    private static func markerH(_ mk: MapMarker) -> CGFloat { mk.height ?? 36 }

    private static func offscreen(_ m: MiniMapView, _ r: CGRect) -> Bool {
        r.maxX < -20 || r.maxY < -20 || r.minX > m.width + 20 || r.minY > m.height + 20
    }

    /// callout / title：标记上方居中，再按 anchorX / anchorY 偏移
    static func callout(_ m: MiniMapView, _ mk: MapMarker, _ b: MapBubble, _ scale: CGFloat) {
        guard !b.content.isEmpty, let img = ensure(b, scale, tail: true) else { return }
        let s = m.toScreen(mk.pos)
        let r = CGRect(x: s.x - b.boxSize.width / 2 + b.anchorX, y: s.y - markerH(mk) * mk.anchorY - 8 - b.boxSize.height + b.anchorY,
                       width: b.boxSize.width, height: b.boxSize.height)
        if offscreen(m, r) { return }
        blit(img, r.origin)
        m.hits.append(MapHit(kind: .callout, id: mk.id, rect: r))
    }

    /// label：官方 anchorX / anchorY 是标签左上角相对标记锚点的偏移
    static func label(_ m: MiniMapView, _ mk: MapMarker, _ b: MapBubble, _ scale: CGFloat) {
        guard !b.content.isEmpty, let img = ensure(b, scale, tail: false) else { return }
        let s = m.toScreen(mk.pos)
        let r = CGRect(x: s.x + b.anchorX, y: s.y + b.anchorY, width: b.boxSize.width, height: b.boxSize.height)
        if offscreen(m, r) { return }
        blit(img, r.origin)
        m.hits.append(MapHit(kind: .label, id: mk.id, rect: r))
    }

    /// customCallout：引擎把 slot="callout" 里 marker-id 对上的 cover-view 画成了 PNG（[MiniMapView.customImages]）
    static func custom(_ m: MiniMapView, _ mk: MapMarker, _ cc: MapCustomCallout) {
        guard let ci = m.customImages[mk.id],
              let img = MapTiles.icon(ci.path, file: URL(fileURLWithPath: ci.path), done: { [weak m] in m?.invalidate() }) else { return }
        let s = m.toScreen(mk.pos)
        let r = CGRect(x: s.x - ci.size.width / 2 + cc.anchorX, y: s.y - markerH(mk) * mk.anchorY - 4 - ci.size.height + cc.anchorY,
                       width: ci.size.width, height: ci.size.height)
        if offscreen(m, r) { return }
        img.draw(in: r)
        m.hits.append(MapHit(kind: .callout, id: mk.id, rect: r))
    }

    /// controls：按 position 钉在框里（不随地图动），可点的记进 hits
    static func controls(_ m: MiniMapView) {
        for k in m.controls {
            let icon = k.iconPath.isEmpty ? nil : MapTiles.icon(k.iconPath, file: m.resolvePath(k.iconPath)) { [weak m] in m?.invalidate() }
            guard let w = k.width ?? icon.map({ $0.size.width / 2 }), let h = k.height ?? icon.map({ $0.size.height / 2 }) else { continue }
            let r = CGRect(x: k.left, y: k.top, width: w, height: h)
            icon?.draw(in: r)
            if k.clickable { m.hits.append(MapHit(kind: .control, id: k.id, rect: r)) }
        }
    }

    private static let blue = UIColor(red: 0x1A / 255.0, green: 0x73 / 255.0, blue: 0xE8 / 255.0, alpha: 1)

    /// show-location：精度圈 + 白边蓝点（有方向时加一个朝向的小三角）
    static func location(_ m: MiniMapView, _ c: CGContext) {
        guard let loc = m.location, let f = loc.fix else { return }
        let s = m.toScreen(f)
        if s.x < -60 || s.y < -60 || s.x > m.width + 60 || s.y > m.height + 60 { return }
        let r = CGFloat(loc.accuracy / MapGeo.metersPerPx(f.lat, m.zoom))
        if r > 10 {
            let e = CGRect(x: s.x - r, y: s.y - r, width: r * 2, height: r * 2)
            c.setFillColor(blue.withAlphaComponent(0.13).cgColor); c.fillEllipse(in: e)
            c.setStrokeColor(blue.withAlphaComponent(0.4).cgColor); c.setLineWidth(1); c.strokeEllipse(in: e)
        }
        if let b = loc.bearing {
            c.saveGState()
            c.translateBy(x: s.x, y: s.y); c.rotate(by: CGFloat(b) * .pi / 180)
            c.move(to: CGPoint(x: 0, y: -15)); c.addLine(to: CGPoint(x: -6, y: -7)); c.addLine(to: CGPoint(x: 6, y: -7)); c.closePath()
            c.setFillColor(blue.cgColor); c.fillPath()
            c.restoreGState()
        }
        c.saveGState()
        c.setShadow(offset: CGSize(width: 0, height: 0.5), blur: 2, color: UIColor(white: 0, alpha: 0.27).cgColor)
        c.setFillColor(UIColor.white.cgColor); c.fillEllipse(in: CGRect(x: s.x - 8, y: s.y - 8, width: 16, height: 16))
        c.restoreGState()
        c.setFillColor(blue.cgColor); c.fillEllipse(in: CGRect(x: s.x - 6, y: s.y - 6, width: 12, height: 12))
    }
}
