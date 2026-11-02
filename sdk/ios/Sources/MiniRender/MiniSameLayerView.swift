import CoreGraphics
import Foundation
import MiniRenderFFI
import UIKit

/// 同层渲染的原生组件（`<map>` 这类）：宿主**画**它，引擎把画好的那一帧画进页面（[MiniProgramView.sameLayerViews]）。
///
/// 与 Android 的 `MiniSameLayerView.kt` 同一套约定：滚动时它就是页面画布里的一块像素（不像盖上去的 UIView 一滑就错开），
/// 宿主只在内容变了时重画（[invalidate]）。都在主线程上调。
///
/// 手势：默认拖动从框里开始照样滚页面，只把点击交给 [onTap]；[wantsGestures] 为真的（SDK 自带的地图 [MiniMapView]）
/// 按在框里的整串触摸都交给 [onTouch]（拖地图、捏合缩放，页面不滚）。
open class MiniSameLayerView {
    /// 节点的键（`id`，没有就是标签名）
    public internal(set) var key = ""
    /// 框的逻辑尺寸（[onTap] / [onTouch] 的坐标系；[draw] 拿到的是设备像素）
    public internal(set) var width: CGFloat = 0
    public internal(set) var height: CGFloat = 0
    /// 节点属性（模板求值之后；数组 / 对象属性是标准 JSON 字符串）
    public internal(set) var attrs: [String: Any] = [:]

    weak var host: SameLayerViews?

    public init() {}

    /// 属性变了（含第一次挂上）。默认重画
    open func onAttrs(_ attrs: [String: Any]) { invalidate() }

    /// 画一帧：上下文已按 [scale]（dpr）缩放、原点在左上，下面用逻辑像素画；[pixelWidth]×[pixelHeight] 是设备像素
    open func draw(in ctx: CGContext, pixelWidth: Int, pixelHeight: Int, scale: CGFloat) {}

    /// 点在框里（视图本地的逻辑像素）
    open func onTap(x: CGFloat, y: CGFloat) {}

    /// 自己处理手势：建引擎时这个标签进 `sameLayerGestures`
    open var wantsGestures: Bool { false }

    /// 按在框里的一串触摸：`phase` 是 start / move / end / cancel，`pointer` 0 / 1，坐标是框里的逻辑像素，`t` 引擎时钟毫秒
    open func onTouch(phase: String, pointer: Int, x: CGFloat, y: CGFloat, t: Double) {}

    /// 上下文对象（`MapContext`）的方法：认得就回包并返回 true
    open func onContextCall(_ method: String, args: [String: Any], reply: MiniHostReply) -> Bool { false }
    open var contextClass: String { "" }
    open var contextMethods: [String] { [] }

    /// 从页面上拿掉了
    open func onDetach() {}

    /// 内容变了：下一帧重画、交给引擎
    public func invalidate() { host?.invalidate(self) }

    /// 回传组件事件：派发给节点上的 `bind<name>`
    public func emit(_ name: String, _ detail: [String: Any] = [:]) { host?.emit(self, name, detail) }

    /// 小程序里的路径 → 本地文件（`/images/a.png` 包根、`images/a.png` 当前页目录）。网络地址返回 nil
    public func resolvePath(_ path: String) -> URL? { host?.resolve(path) }
}

/// [MiniProgramView] 的同层组件那一片：按 `sameLayerViews` 事件建 / 改 / 放实例，主线程上画进 RGBA 位图，
/// 交给引擎（`mr_native_view_frame_rgba`，任何线程都行），再让引擎线程出一帧。
final class SameLayerViews {
    var factories: [String: () -> MiniSameLayerView] = [:]
    var appDir: URL?
    var route = ""
    /// 在引擎线程上执行（EngineSession.run）
    private let run: (@escaping (OpaquePointer) -> Void) -> Void
    private let scale: () -> CGFloat

    private final class Slot {
        let view: MiniSameLayerView
        var w: Int
        var h: Int
        var ctx: CGContext?
        init(_ v: MiniSameLayerView, _ w: Int, _ h: Int) { view = v; self.w = w; self.h = h }
    }
    private var slots: [String: Slot] = [:]
    private var dirty = Set<String>()
    private var scheduled = false
    private var probes: [String: MiniSameLayerView] = [:]

    init(run: @escaping (@escaping (OpaquePointer) -> Void) -> Void, scale: @escaping () -> CGFloat) {
        self.run = run
        self.scale = scale
    }

    private func probe(_ tag: String) -> MiniSameLayerView? {
        if let p = probes[tag] { return p }
        let p = factories[tag]?()
        probes[tag] = p
        return p
    }

    var tags: [String] { Array(factories.keys) }
    var gestureTags: [String] { factories.keys.filter { probe($0)?.wantsGestures == true } }
    var contextApis: [String] {
        factories.keys.flatMap { t -> [String] in
            guard let p = probe(t) else { return [] }
            return p.contextMethods.map { "\(p.contextClass).\($0)" }
        }
    }

    func resolve(_ path: String) -> URL? {
        guard let dir = appDir else { return nil }
        let p = path.trimmingCharacters(in: .whitespaces)
        if p.isEmpty || p.contains("://") { return nil }
        let fm = FileManager.default
        if p.hasPrefix("/") {
            let u = dir.appendingPathComponent(String(p.drop(while: { $0 == "/" })))
            return fm.fileExists(atPath: u.path) ? u : nil
        }
        if let slash = route.lastIndex(of: "/") {
            let u = dir.appendingPathComponent(String(route[..<slash])).appendingPathComponent(p)
            if fm.fileExists(atPath: u.path) { return u }
        }
        let u = dir.appendingPathComponent(p)
        return fm.fileExists(atPath: u.path) ? u : nil
    }

    /// `hostCall` 里的 `<类>.<方法>`：按 `contextId`（`id@实例键` 取 `@` 前面）找实例。不是上下文方法返回 false
    func contextCall(_ call: MiniHostCall, _ reply: MiniHostReply) -> Bool {
        guard let dot = call.api.firstIndex(of: ".") else { return false }
        let cls = String(call.api[..<dot])
        let method = String(call.api[call.api.index(after: dot)...])
        let id = (call.args["contextId"] as? String ?? "").split(separator: "@", maxSplits: 1).first.map(String.init) ?? ""
        let views = slots.values.map { $0.view }.filter { $0.contextClass == cls }
        guard let v = views.first(where: { $0.key == id || id.isEmpty }) ?? (views.count == 1 ? views[0] : nil) else {
            if !probes.values.contains(where: { $0.contextClass == cls }) { return false }
            reply.fail("\(method):fail 页面上没有 id 为 \"\(id)\" 的组件")
            return true
        }
        if !v.onContextCall(method, args: call.args, reply: reply) { reply.fail("\(method):fail 当前宿主不支持") }
        return true
    }

    /// 主线程：`sameLayerViews` 事件
    func onViews(_ ev: [String: Any]) {
        let list = ev["views"] as? [[String: Any]] ?? []
        var seen = Set<String>()
        for o in list {
            let key = o["key"] as? String ?? ""
            let box = o["box"] as? [NSNumber] ?? []
            let w = box.count > 0 ? box[0].intValue : 0
            let h = box.count > 1 ? box[1].intValue : 0
            guard !key.isEmpty, w > 0, h > 0 else { continue }
            seen.insert(key)
            var slot = slots[key]
            let fresh = slot == nil
            if slot == nil {
                guard let make = factories[o["tag"] as? String ?? ""] else { continue }
                let v = make()
                v.key = key
                v.host = self
                slot = Slot(v, w, h)
                slots[key] = slot
            }
            guard let s = slot else { continue }
            s.view.width = CGFloat((o["width"] as? NSNumber)?.doubleValue ?? 0)
            s.view.height = CGFloat((o["height"] as? NSNumber)?.doubleValue ?? 0)
            let attrs = o["attrs"] as? [String: Any] ?? [:]
            if fresh || !NSDictionary(dictionary: attrs).isEqual(to: s.view.attrs) {
                s.view.attrs = attrs
                s.view.onAttrs(attrs)
            }
            if s.w != w || s.h != h {
                s.w = w
                s.h = h
                s.ctx = nil
                s.view.invalidate()
            }
        }
        for k in Array(slots.keys) where !seen.contains(k) {
            if let s = slots.removeValue(forKey: k) {
                s.view.host = nil
                s.view.onDetach()
            }
            dirty.remove(k)
        }
    }

    func onTap(_ ev: [String: Any]) {
        slots[ev["key"] as? String ?? ""]?.view.onTap(x: cg(ev["x"]), y: cg(ev["y"]))
    }

    func onTouch(_ ev: [String: Any]) {
        slots[ev["key"] as? String ?? ""]?.view.onTouch(
            phase: ev["phase"] as? String ?? "", pointer: (ev["pointer"] as? NSNumber)?.intValue ?? 0,
            x: cg(ev["x"]), y: cg(ev["y"]), t: (ev["t"] as? NSNumber)?.doubleValue ?? 0)
    }

    private func cg(_ v: Any?) -> CGFloat { CGFloat((v as? NSNumber)?.doubleValue ?? 0) }

    func invalidate(_ v: MiniSameLayerView) {
        guard slots[v.key]?.view === v else { return }
        dirty.insert(v.key)
        if scheduled { return }
        scheduled = true
        // 同一轮里多次 invalidate 只画一次
        DispatchQueue.main.async { [weak self] in self?.drawDirty() }
    }

    func emit(_ v: MiniSameLayerView, _ name: String, _ detail: [String: Any]) {
        let key = v.key
        let json = (try? JSONSerialization.data(withJSONObject: detail)).flatMap { String(data: $0, encoding: .utf8) } ?? "{}"
        run { _ = mr_app_native_view_event($0, key, name, json) }
    }

    private func drawDirty() {
        scheduled = false
        let keys = dirty
        dirty.removeAll()
        let sc = scale()
        for k in keys {
            guard let s = slots[k] else { continue }
            if s.ctx == nil {
                // RGBA 字节序（premultipliedLast + 大端 32 位 = 内存里 R G B A），与引擎要的一致
                s.ctx = CGContext(
                    data: nil, width: s.w, height: s.h, bitsPerComponent: 8, bytesPerRow: s.w * 4,
                    space: CGColorSpace(name: CGColorSpace.sRGB) ?? CGColorSpaceCreateDeviceRGB(),
                    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue)
            }
            guard let ctx = s.ctx, let data = ctx.data else { continue }
            ctx.saveGState()
            ctx.clear(CGRect(x: 0, y: 0, width: s.w, height: s.h))
            // UIKit 的坐标（左上原点、y 向下）+ 按 dpr 缩放：下面全是逻辑像素
            ctx.translateBy(x: 0, y: CGFloat(s.h))
            ctx.scaleBy(x: sc, y: -sc)
            UIGraphicsPushContext(ctx)
            s.view.draw(in: ctx, pixelWidth: s.w, pixelHeight: s.h, scale: sc)
            UIGraphicsPopContext()
            ctx.restoreGState()
            let ok = mr_native_view_frame_rgba(k, UInt32(s.w), UInt32(s.h), UInt32(ctx.bytesPerRow), data.assumingMemoryBound(to: UInt8.self))
            if ok == 1 { run { _ in } }
        }
    }
}
