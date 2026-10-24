package dev.minirender.example

import android.net.Uri
import org.json.JSONObject

/**
 * 一次「打开小程序」：外部 App / H5 的链接、商店列表的点击、`wx.navigateToMiniProgram` 都归到这里。
 * 与 Flutter 示例的 lib/store/store_links.dart 同一套规则。
 *
 * 链接的形状仿微信的 `weixin://dl/business/?appid=&path=&query=`：
 *     minirender://open?appid=wx3b7d9f1a2c4e6b80&path=pages/detail/detail&query=id%3D3
 * 也认 mini-store 的 H5 落地页地址（`http(s)://<商店>/open?appid=…`、`/a/<AppID>?path=…`）。
 */
data class StoreLink(
    /** AppID（或 id） */
    val key: String,
    /** 页面路径（不带前导 `/`）；空 = 首页 */
    val path: String = "",
    val query: Map<String, String> = emptyMap(),
    /** 微信的场景值：1001 小程序列表、1065 URL Scheme（H5 / 外部 App）、1037 小程序打开小程序 */
    val scene: Int = 1065,
    /** 从别的小程序跳来时：`{appId, extraData}` */
    val referrer: JSONObject? = null,
) {
    /** 引擎的起始页（`pages/x/x?a=1`）；null = 首页 */
    val route: String?
        get() {
            if (path.isEmpty()) return null
            if (query.isEmpty()) return path
            val b = Uri.Builder()
            query.forEach { (k, v) -> b.appendQueryParameter(k, v) }
            return path + "?" + b.build().encodedQuery
        }

    /** 交给引擎的 launchOptions（`wx.getLaunchOptionsSync()` / `App.onLaunch(options)`） */
    fun launchOptions(): JSONObject = JSONObject()
        .put("path", path)
        .put("query", JSONObject(query))
        .put("scene", scene)
        .apply { referrer?.let { put("referrerInfo", it) } }

    fun toJson(): String = JSONObject().put("key", key).put("path", path).put("query", JSONObject(query))
        .put("scene", scene).put("referrer", referrer ?: JSONObject.NULL).toString()

    companion object {
        fun fromJson(s: String): StoreLink {
            val j = JSONObject(s)
            val q = j.optJSONObject("query")
            return StoreLink(
                key = j.getString("key"),
                path = j.optString("path"),
                query = q?.keys()?.asSequence()?.associateWith { q.optString(it) } ?: emptyMap(),
                scene = j.optInt("scene", 1065),
                referrer = j.optJSONObject("referrer"),
            )
        }

        /** `minirender://open?...` / `http(s)://…/open?appid=…` / `…/a/<AppID>`；认不出返回 null */
        fun parse(url: String): StoreLink? {
            val u = runCatching { Uri.parse(url.trim()) }.getOrNull() ?: return null
            val key = when (u.scheme) {
                "minirender" -> if (u.host == "open") u.getQueryParameter("appid") ?: u.getQueryParameter("id") else null
                "http", "https" -> {
                    val seg = u.pathSegments
                    when {
                        seg.size == 1 && seg[0] == "open" -> u.getQueryParameter("appid")
                        seg.size == 2 && seg[0] == "a" -> seg[1]
                        else -> null
                    }
                }
                else -> null
            }
            if (key.isNullOrEmpty()) return null
            var path = (u.getQueryParameter("path") ?: "").trimStart('/')
            val query = linkedMapOf<String, String>()
            // path 里可以直接带 ?a=1（微信也允许），query 参数另给的再合进去
            val qi = path.indexOf('?')
            if (qi >= 0) {
                query.putAll(splitQuery(path.substring(qi + 1)))
                path = path.substring(0, qi)
            }
            u.getQueryParameter("query")?.takeIf { it.isNotEmpty() }?.let { query.putAll(splitQuery(it)) }
            return StoreLink(key, path, query)
        }

        private fun splitQuery(q: String): Map<String, String> {
            val u = Uri.parse("x://x?$q")
            return u.queryParameterNames.associateWith { u.getQueryParameter(it) ?: "" }
        }
    }
}
