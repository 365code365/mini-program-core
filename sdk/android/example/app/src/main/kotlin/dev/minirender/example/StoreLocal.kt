package dev.minirender.example

import android.content.Context
import org.json.JSONArray

/**
 * 本机记着的两样（与微信一样只存在手机上）：
 * - 最近使用的小程序（入口页顶上那排，最新在前，最多 [MAX_RECENT] 个，存整条 [MiniStore.App]：离线也画得出来）；
 * - 搜索历史（搜索页没输入时显示，最多 [MAX_HISTORY] 条，可清空）。
 */
class StoreLocal(ctx: Context) {
    private val prefs = ctx.getSharedPreferences("store-local", Context.MODE_PRIVATE)

    fun recent(): List<MiniStore.App> = arr("recent").let { a ->
        (0 until a.length()).mapNotNull { runCatching { MiniStore.App.fromJson(a.getJSONObject(it)) }.getOrNull() }
    }

    fun touch(app: MiniStore.App) {
        val list = listOf(app) + recent().filter { it.id != app.id }
        put("recent", JSONArray(list.take(MAX_RECENT).map { org.json.JSONObject(it.toJson()) }))
    }

    fun history(): List<String> = arr("history").let { a -> (0 until a.length()).map { a.optString(it) } }

    fun addHistory(q: String) {
        val w = q.trim()
        if (w.isEmpty()) return
        put("history", JSONArray((listOf(w) + history().filter { it != w }).take(MAX_HISTORY)))
    }

    fun clearHistory() = put("history", JSONArray())

    private fun arr(key: String) = runCatching { JSONArray(prefs.getString(key, "[]")) }.getOrDefault(JSONArray())
    private fun put(key: String, a: JSONArray) = prefs.edit().putString(key, a.toString()).apply()

    companion object {
        const val MAX_RECENT = 8
        const val MAX_HISTORY = 12
    }
}
