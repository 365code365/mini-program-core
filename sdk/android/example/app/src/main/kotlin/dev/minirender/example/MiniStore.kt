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
    data class App(
        val id: String,
        val appId: String,
        val name: String,
        val description: String,
        val version: Int,
        val sha256: String,
        val size: Long,
        val packageUrl: String,
    ) {
        /** 深链 / 跳转用的 key：有 AppID 用 AppID */
        val key: String get() = appId.ifEmpty { id }

        fun toJson(): String = JSONObject()
            .put("id", id).put("appId", appId).put("name", name).put("description", description).put("version", version)
            .put("sha256", sha256).put("size", size).put("packageUrl", packageUrl).toString()

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
            )
        }
    }

    /** [offline] = 服务器连不上，用的是上次缓存的列表 */
    data class Listing(val apps: List<App>, val offline: Boolean, val error: String?)

    /** [stale] 不为 null：没拿到最新版，打开的是本地旧版（值是原因）；[updated] = 这次下了新包 */
    data class Prepared(val dir: File, val app: App, val updated: Boolean, val stale: Exception?)

    /** 服务器明确回了 404：没有这个小程序（AppID 不对 / 已下架） */
    class NotFound(msg: String) : IOException(msg)

    private val root = File(ctx.filesDir, "mini-apps").apply { mkdirs() }

    fun list(): Listing {
        val cache = File(root, "index.json")
        return try {
            val body = get("$baseUrl/api/apps").toString(Charsets.UTF_8)
            val apps = parse(body)
            cache.writeText(body)
            Listing(apps, false, null)
        } catch (e: Exception) {
            val apps = if (cache.isFile) runCatching { parse(cache.readText()) }.getOrDefault(emptyList()) else emptyList()
            Listing(apps, true, e.message ?: e.javaClass.simpleName)
        }
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
        .firstOrNull { (it.id == key || it.appId == key) && File(root, "${it.id}/app.json").isFile }

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
        if (!File(tmp, "app.json").isFile) throw IOException("包里没有 app.json")
        val dir = File(root, app.id)
        dir.deleteRecursively()
        if (!tmp.renameTo(dir)) throw IOException("换上新版失败")
        File(root, "${app.id}.json").writeText(app.toJson())
    }

    companion object {
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

        private fun sha256(b: ByteArray): String =
            MessageDigest.getInstance("SHA-256").digest(b).joinToString("") { "%02x".format(it) }
    }
}
