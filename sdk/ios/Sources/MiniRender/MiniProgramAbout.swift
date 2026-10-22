import Foundation

/// 小程序的「关于」信息：引擎启动后第一帧的 `app` 事件带上来（字段见引擎的 `src/app_about.rs`）。
///
/// 胶囊「•••」的默认行为就是弹框展示它；宿主要换成自己的页面，设 `MiniProgramView.onAbout`。
public struct MiniProgramAbout {
    /// `project.config.json` 的 `projectname`，没有就是 app.json 的导航栏标题 / 目录名
    public let name: String
    /// 没设置（或开发者工具的游客 `touristappid`）时为空
    public let appId: String
    /// 开发者工具写的默认描述不算
    public let description: String
    public let entryPage: String
    /// 主包 + 分包的页面数
    public let pageCount: Int
    /// `mini-render 0.1.0`
    public let engine: String

    init(json o: [String: Any]) {
        name = o["name"] as? String ?? ""
        appId = o["appId"] as? String ?? ""
        description = o["description"] as? String ?? ""
        entryPage = o["entryPage"] as? String ?? ""
        pageCount = (o["pageCount"] as? NSNumber)?.intValue ?? 0
        engine = o["engine"] as? String ?? ""
    }

    /// 演示用的弹框文案（与桌面窗体、H5 的同一份）
    public var dialogTitle: String { name.isEmpty ? "关于小程序" : "关于\(name)" }

    public var dialogMessage: String {
        var lines: [String] = []
        if !description.isEmpty { lines.append(description) }
        lines.append("AppID：\(appId.isEmpty ? "未设置" : appId)")
        lines.append("入口：\(entryPage)（共 \(pageCount) 页）")
        lines.append("引擎：\(engine)")
        return lines.joined(separator: "\n")
    }
}

/// 点了右上角胶囊的哪一半（引擎事件 `menuButton` 的 `action`）
public enum MiniMenuButtonAction: String {
    /// 左半「•••」：查看关于
    case menu
    /// 右半「⊙」：返回宿主
    case close
}
