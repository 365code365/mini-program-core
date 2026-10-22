package dev.minirender

import org.json.JSONObject

/**
 * 小程序的「关于」信息：引擎启动后第一帧的 `app` 事件带上来（字段见引擎的 `src/app_about.rs`）。
 *
 * 胶囊「•••」的默认行为就是弹框展示它；宿主要换成自己的页面，设 [MiniProgramView.onAbout]。
 */
data class MiniProgramAbout(
    /** `project.config.json` 的 `projectname`，没有就是 app.json 的导航栏标题 / 目录名 */
    val name: String,
    /** 没设置（或开发者工具的游客 `touristappid`）时为空 */
    val appId: String,
    /** 开发者工具写的默认描述不算 */
    val description: String,
    val entryPage: String,
    /** 主包 + 分包的页面数 */
    val pageCount: Int,
    /** `mini-render 0.1.0` */
    val engine: String,
) {
    /** 演示用的弹框标题（与桌面窗体、H5 的同一份） */
    val dialogTitle: String get() = if (name.isEmpty()) "关于小程序" else "关于$name"

    val dialogMessage: String
        get() = buildList {
            if (description.isNotEmpty()) add(description)
            add("AppID：" + appId.ifEmpty { "未设置" })
            add("入口：$entryPage（共 $pageCount 页）")
            add("引擎：$engine")
        }.joinToString("\n")

    companion object {
        fun fromJson(o: JSONObject?) = MiniProgramAbout(
            name = o?.optString("name") ?: "",
            appId = o?.optString("appId") ?: "",
            description = o?.optString("description") ?: "",
            entryPage = o?.optString("entryPage") ?: "",
            pageCount = o?.optInt("pageCount") ?: 0,
            engine = o?.optString("engine") ?: "",
        )
    }
}
