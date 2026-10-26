package dev.minirender.example

import android.content.Context
import android.os.Handler
import android.os.Looper
import org.json.JSONArray
import org.json.JSONObject

/**
 * 本机记着的三样（与微信一样只存在手机上，与 Flutter 示例 mini_store.dart 同一套规则）：
 * - 最近使用的小程序（入口页那排、下拉面板，最新在前，最多 [MAX_RECENT] 个，存整条 [MiniStore.App]：离线也画得出来）；
 * - 我的小程序（收藏，后收藏的在前，最多 [MAX_FAVORITES] 个）；
 * - 搜索历史（搜索页没输入时显示，最多 [MAX_HISTORY] 条，可清空）。
 * 每改一次通知 [listeners]（主线程）：在面板 / 胶囊菜单 / 搜索页里收藏、删除，入口页跟着变。
 */
class StoreLocal(ctx: Context) {
    private val prefs = ctx.getSharedPreferences("store-local", Context.MODE_PRIVATE)

    fun recent(): List<MiniStore.App> = apps("recent")

    fun touch(app: MiniStore.App) {
        putApps("recent", (listOf(app) + recent().filter { it.id != app.id }).take(MAX_RECENT), notify = false)
        // 收藏里存的是收藏那一刻的版本：打开时顺手换成最新的（名字 / 图标改了，面板上跟着变）
        val fav = favorites()
        if (fav.any { it.id == app.id }) putApps("favorites", fav.map { if (it.id == app.id) app else it }, notify = false)
        changed()
    }

    /** 从「最近使用」里删掉（长按 → 删除）；收藏不受影响 */
    fun removeRecent(id: String) = putApps("recent", recent().filter { it.id != id })

    fun favorites(): List<MiniStore.App> = apps("favorites")

    fun isFavorite(id: String) = favorites().any { it.id == id }

    /** 收藏 / 取消收藏；返回之后的状态（true = 已在「我的小程序」里） */
    fun toggleFavorite(app: MiniStore.App): Boolean {
        val fav = favorites()
        val had = fav.any { it.id == app.id }
        putApps("favorites", if (had) fav.filter { it.id != app.id } else (listOf(app) + fav).take(MAX_FAVORITES))
        return !had
    }

    fun history(): List<String> = arr("history").let { a -> (0 until a.length()).map { a.optString(it) } }

    fun addHistory(q: String) {
        val w = q.trim()
        if (w.isEmpty()) return
        put("history", JSONArray((listOf(w) + history().filter { it != w }).take(MAX_HISTORY)))
    }

    fun clearHistory() = put("history", JSONArray())

    private fun apps(key: String) = arr(key).let { a ->
        (0 until a.length()).mapNotNull { runCatching { MiniStore.App.fromJson(a.getJSONObject(it)) }.getOrNull() }
    }

    private fun putApps(key: String, list: List<MiniStore.App>, notify: Boolean = true) {
        prefs.edit().putString(key, JSONArray(list.map { JSONObject(it.toJson()) }).toString()).apply()
        if (notify) changed()
    }

    private fun arr(key: String) = runCatching { JSONArray(prefs.getString(key, "[]")) }.getOrDefault(JSONArray())
    private fun put(key: String, a: JSONArray) {
        prefs.edit().putString(key, a.toString()).apply()
        changed()
    }

    private fun changed() = main.post { listeners.toList().forEach { it() } }

    companion object {
        const val MAX_RECENT = 8
        const val MAX_HISTORY = 12
        const val MAX_FAVORITES = 50

        private val main = Handler(Looper.getMainLooper())

        /** 本机记录变了（主线程回调）。在 onCreate 里加、onDestroy 里删 */
        val listeners = mutableSetOf<() -> Unit>()
    }
}
