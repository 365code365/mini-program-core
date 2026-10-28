package dev.minirender.example

import android.content.Context
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileNotFoundException
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.zip.ZipInputStream

/**
 * 小程序商店客户端：从 mini-store 服务（tools/mini-store/server.py）拉列表、下载包、解到本地目录。
 * 与 Flutter 示例的 lib/store/mini_store.dart 同一套规则：
 *
 * - **每次打开前都问一次服务器最新版**（[prepare]，`GET /api/apps/<AppID>`）：sha256 与本地一致就直接打开，
 *   不一致就先下新包。列表里的版本信息可能是几分钟前拉的，不能拿来判断；
 * - 解到 `filesDir/mini-apps/<id>/`：目录名就是 id —— 引擎的 storage 按目录名分文件，各小程序不串数据；
 *   旁边 `<id>.json` 记着本地这一版（AppID、版本、sha256）；
 * - 服务器连不上：本地有就打开本地那版（离线可用），没有才报错；服务器明确说没有（404，已下架）就不打开；
 * - 先解到临时目录再整个换上去，解一半断掉不会留下半个小程序。
 *
 * 全部是阻塞调用，放后台线程跑。
 */
class MiniStore(ctx: Context, val baseUrl: String = DEFAULT_URL) {
    /** 图标加载要用（application context，不拿着 Activity） */
    val context: Context = ctx.applicationContext
    data class App(
        val id: String,
        val appId: String,
        val name: String,
        val description: String,
        val version: Int,
        val sha256: String,
        val size: Long,
        val packageUrl: String,
        /** 商店展示与搜索用（服务器 store.meta.json / 管理页填的）；老服务器没有这些字段 */
        val category: String = "",
        val tags: List<String> = emptyList(),
        val developer: String = "",
        val iconUrl: String = "",
        val downloads: Int = 0,
        /** "miniprogram" | "game"（服务器 meta 的 kind；老服务器没有 = 小程序）；小游戏另带 orientation */
        val kind: String = KIND_MINI,
        val orientation: String = "",
    ) {
        /** 深链 / 跳转用的 key：有 AppID 用 AppID */
        val key: String get() = appId.ifEmpty { id }
        val isGame: Boolean get() = kind == KIND_GAME
        /** 包根上的清单：装没装好就看它在不在 */
        val manifest: String get() = if (isGame) "game.json" else "app.json"

        fun toJson(): String = JSONObject()
            .put("id", id).put("appId", appId).put("name", name).put("description", description).put("version", version)
            .put("sha256", sha256).put("size", size).put("packageUrl", packageUrl)
            .put("category", category).put("tags", org.json.JSONArray(tags)).put("developer", developer)
            .put("iconUrl", iconUrl).put("downloads", downloads).put("kind", kind).put("orientation", orientation).toString()

        companion object {
            fun fromJson(j: JSONObject) = App(
                id = j.getString("id"),
                appId = j.optString("appId"),
                name = j.optString("name", j.getString("id")),
                description = j.optString("description"),
                version = j.optInt("version"),
                sha256 = j.optString("sha256"),
                size = j.optLong("size"),
                packageUrl = j.optString("packageUrl", "/api/apps/${j.getString("id")}/package"),
                category = j.optString("category"),
                tags = j.optJSONArray("tags")?.let { a -> (0 until a.length()).map { a.optString(it) } }.orEmpty(),
                developer = j.optString("developer"),
                iconUrl = j.optString("iconUrl"),
                downloads = j.optInt("downloads"),
                kind = j.optString("kind").ifEmpty { KIND_MINI },
                orientation = j.optString("orientation"),
            )
        }
    }

    /** [offline] = 服务器连不上，用的是上次缓存的列表 */
    data class Listing(val apps: List<App>, val offline: Boolean, val error: String?)

    /** [stale] 不为 null：没拿到最新版，打开的是本地旧版（值是原因）；[updated] = 这次下了新包 */
    data class Prepared(val dir: File, val app: App, val updated: Boolean, val stale: Exception?)

    /** 服务器明确回了 404：没有这个小程序（AppID 不对 / 已下架） */
    class NotFound(msg: String) : IOException(msg)

    private val root = File(context.filesDir, "mini-apps").apply { mkdirs() }

    fun list(): Listing {
        val cache = File(root, "index.json")
        return try {
            val body = get("$baseUrl/api/apps?kind=all").toString(Charsets.UTF_8)
            val apps = parse(body)
            cache.writeText(body)
            Listing(apps, false, null)
        } catch (e: Exception) {
            val apps = if (cache.isFile) runCatching { parse(cache.readText()) }.getOrDefault(emptyList()) else emptyList()
            Listing(apps, true, e.message ?: e.javaClass.simpleName)
        }
    }

    /** 搜索（服务器 `/api/apps?q=&category=`）：关键词每段都要命中，按相关度 + 热度排。返回 (结果, 总数) */
    fun search(q: String, category: String = ""): Pair<List<App>, Int> {
        val url = "$baseUrl/api/apps?q=${enc(q)}&category=${enc(category)}&limit=100&kind=all"
        val body = JSONObject(get(url, connectMs = 6000).toString(Charsets.UTF_8))
        return parse(body.toString()) to body.optInt("total")
    }

    /** 热搜词（服务器按搜索次数排，冷启动时用热门小程序补齐） */
    fun hotKeywords(): List<String> = strings(JSONObject(get("$baseUrl/api/search/hot", connectMs = 6000).toString(Charsets.UTF_8)), "keywords")

    /** 输入联想 */
    fun suggest(q: String): List<String> =
        strings(JSONObject(get("$baseUrl/api/search/suggest?q=${enc(q)}", connectMs = 4000).toString(Charsets.UTF_8)), "words")

    /** 分类与数量（`name to count`），服务器的固定顺序 */
    fun categories(): List<Pair<String, Int>> {
        val a = JSONObject(get("$baseUrl/api/categories?kind=all", connectMs = 6000).toString(Charsets.UTF_8)).getJSONArray("categories")
        return (0 until a.length()).map { a.getJSONObject(it).let { o -> o.getString("name") to o.optInt("count") } }
    }

    /** 商店图标的字节（`filesDir/mini-apps/icons/<id>-v<版本>.png` 缓存，换版本换文件）；没有图标返回 null */
    fun iconBytes(app: App): ByteArray? {
        if (app.iconUrl.isEmpty()) return null
        val dir = File(root, "icons").apply { mkdirs() }
        val f = File(dir, "${app.id}-v${app.version}.png")
        if (f.isFile) return f.readBytes()
        val bytes = runCatching { get(URL(URL(baseUrl), app.iconUrl).toString(), connectMs = 6000) }.getOrNull() ?: return null
        dir.listFiles { x -> x.name.startsWith("${app.id}-v") }?.forEach { it.delete() }
        f.writeBytes(bytes)
        return bytes
    }

    /** 服务器上的最新版（key = AppID 或 id） */
    fun latest(key: String): App {
        if (!APPID_RE.matches(key) && !ID_RE.matches(key)) throw NotFound("非法的 AppID：$key")
        val app = App.fromJson(JSONObject(get("$baseUrl/api/apps/$key", connectMs = 6000).toString(Charsets.UTF_8)))
        if (!ID_RE.matches(app.id)) throw IOException("服务器给的 id 不合法")
        return app
    }

    /** 本地装着的那一版（key = AppID 或 id）；没装返回 null */
    fun local(key: String): App? = root.listFiles { f -> f.isFile && f.name.endsWith(".json") && f.name != "index.json" }
        .orEmpty()
        .mapNotNull { f -> runCatching { App.fromJson(JSONObject(f.readText())) }.getOrNull() }
        .firstOrNull { (it.id == key || it.appId == key) && File(root, "${it.id}/${it.manifest}").isFile }

    /**
     * 打开前的准备：问服务器最新版 → 本地不是这一版就下载解包 → 返回目录。
     * [status] 给 loading 页的阶段文字，[progress] 给（已收, 总字节）
     */
    fun prepare(key: String, status: (String) -> Unit = {}, progress: (Long, Long) -> Unit = { _, _ -> }): Prepared {
        status("检查更新…")
        val app = try {
            latest(key)
        } catch (e: NotFound) {
            throw e // 服务器明确说没有：不拿本地旧版糊弄
        } catch (e: Exception) {
            val have = local(key) ?: throw e
            return Prepared(File(root, have.id), have, false, e)
        }
        val installed = local(app.id)
        if (installed != null && installed.sha256 == app.sha256) return Prepared(File(root, app.id), app, false, null)
        status(if (installed == null) "下载中…" else "发现新版本 v${app.version}，更新中…")
        return try {
            val bytes = get(URL(URL(baseUrl), app.packageUrl).toString(), app.size, progress = progress)
            if (app.sha256.isNotEmpty() && sha256(bytes) != app.sha256) throw IOException("包校验失败（sha256 与服务器给的不一致）")
            status("解包中…")
            unpack(bytes, app)
            Prepared(File(root, app.id), app, true, null)
        } catch (e: Exception) {
            if (installed == null) throw e
            Prepared(File(root, installed.id), installed, false, e)
        }
    }

    private fun get(url: String, sizeHint: Long = 0, connectMs: Int = 8000, progress: (Long, Long) -> Unit = { _, _ -> }): ByteArray {
        val c = URL(url).openConnection() as HttpURLConnection
        c.connectTimeout = connectMs
        // 单次 read 20 秒没动静就算断了（整包不设上限：几 MB 的包在慢网上也要能下完）
        c.readTimeout = 20000
        try {
            val code = c.responseCode
            if (code == 404) {
                val msg = runCatching { JSONObject(c.errorStream.reader().readText()).optString("error") }.getOrNull()
                throw NotFound(msg?.ifEmpty { null } ?: "没有这个小程序")
            }
            if (code != 200) throw IOException("HTTP $code")
            val total = c.contentLengthLong.takeIf { it > 0 } ?: sizeHint
            val out = ByteArrayOutputStream(total.coerceIn(0, 64L shl 20).toInt())
            val buf = ByteArray(64 shl 10)
            c.inputStream.use { input ->
                while (true) {
                    val n = input.read(buf)
                    if (n < 0) break
                    out.write(buf, 0, n)
                    progress(out.size().toLong(), total)
                }
            }
            return out.toByteArray()
        } catch (e: FileNotFoundException) {
            throw NotFound("没有这个小程序")
        } finally {
            c.disconnect()
        }
    }

    private fun unpack(bytes: ByteArray, app: App) {
        val tmp = File(root, ".${app.id}.tmp")
        tmp.deleteRecursively()
        tmp.mkdirs()
        val base = tmp.canonicalPath + File.separator
        ZipInputStream(bytes.inputStream()).use { zip ->
            while (true) {
                val e = zip.nextEntry ?: break
                if (e.isDirectory) continue
                val out = File(tmp, e.name)
                // 服务器已经挡过一次；客户端照样再挡（路径穿越 = 写到沙盒别处）
                if (!out.canonicalPath.startsWith(base)) throw IOException("包里有非法路径：${e.name}")
                out.parentFile?.mkdirs()
                out.outputStream().use { zip.copyTo(it) }
            }
        }
        if (!File(tmp, app.manifest).isFile) throw IOException("包里没有 ${app.manifest}")
        if (app.isGame && !File(tmp, "game.js").isFile) throw IOException("小游戏包里没有 game.js")
        val dir = File(root, app.id)
        dir.deleteRecursively()
        if (!tmp.renameTo(dir)) throw IOException("换上新版失败")
        File(root, "${app.id}.json").writeText(app.toJson())
    }

    companion object {
        const val KIND_MINI = "miniprogram"
        const val KIND_GAME = "game"

        /** 测试服务器（明文 HTTP，见 res/xml/network_security_config.xml） */
        const val DEFAULT_URL = "http://64.83.22.217:9630"

        /** 与服务器同一条规则：id 要当目录名用 */
        private val ID_RE = Regex("^[a-z0-9][a-z0-9._-]{0,63}$")

        /** 微信 AppID 的形状（服务器分配的也是这个形状） */
        val APPID_RE = Regex("^wx[0-9a-f]{16}$")

        private fun parse(body: String): List<App> {
            val arr = JSONObject(body).getJSONArray("apps")
            return (0 until arr.length()).map { arr.getJSONObject(it) }
                .filter { ID_RE.matches(it.optString("id")) }
                .map { App.fromJson(it) }
        }

        private fun enc(s: String) = java.net.URLEncoder.encode(s, "UTF-8")

        private fun strings(o: JSONObject, key: String): List<String> =
            o.optJSONArray(key)?.let { a -> (0 until a.length()).map { a.optString(it) } }.orEmpty().filter { it.isNotEmpty() }

        private fun sha256(b: ByteArray): String =
            MessageDigest.getInstance("SHA-256").digest(b).joinToString("") { "%02x".format(it) }
    }
}
