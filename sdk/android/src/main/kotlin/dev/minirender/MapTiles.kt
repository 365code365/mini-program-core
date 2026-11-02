package dev.minirender

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.util.LruCache
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.Executors

/**
 * [MiniMapView] 的瓦片：按 `{x}/{y}/{z}` 模板下载，内存 LRU + 磁盘缓存（`cacheDir/mini-map-tiles`，
 * 重开 App 不用再下）。下载在后台线程池，下好了回主线程通知重画。
 *
 * 默认是高德的公开瓦片（GCJ-02，与微信 map 的坐标一致，国内直连）。别的瓦片源（自建、OSM）改 [MiniMapView.tileSource]：
 * 注意 OSM 是 WGS-84，在国内与 GCJ-02 的坐标差几百米。iOS 那份在 `MapGeo.swift` 里。
 */
class MiniMapTileSource(
    /** 普通地图：`{s}` 子域（1~4）、`{x}` `{y}` `{z}` */
    val road: String,
    /** 卫星图（`enable-satellite`） */
    val satellite: String,
    /** 有瓦片的最大级别（再放大就拿这一级拉伸） */
    val maxZoom: Int = 18,
    /** 右下角的版权说明（瓦片源的使用条款一般要求标注） */
    val attribution: String = "",
) {
    fun url(x: Int, y: Int, z: Int, sat: Boolean): String {
        val s = ((x + y) % 4 + 4) % 4 + 1
        return (if (sat) satellite else road)
            .replace("{s}", s.toString()).replace("{x}", x.toString()).replace("{y}", y.toString()).replace("{z}", z.toString())
    }

    companion object {
        val AUTONAVI = MiniMapTileSource(
            road = "https://webrd0{s}.is.autonavi.com/appmaptile?lang=zh_cn&size=1&scale=1&style=8&x={x}&y={y}&z={z}",
            satellite = "https://webst0{s}.is.autonavi.com/appmaptile?style=6&x={x}&y={y}&z={z}",
            attribution = "© 高德地图",
        )
    }
}

internal class MapTiles(context: Context, private val source: () -> MiniMapTileSource) {
    private val disk = File(context.cacheDir, "mini-map-tiles").apply { mkdirs() }
    private val main = Handler(Looper.getMainLooper())
    /** 解码后的瓦片（每张 256² × 4 = 256KB）：按字节算，留 48MB */
    private val mem = object : LruCache<String, Bitmap>(48 * 1024 * 1024) {
        override fun sizeOf(key: String, value: Bitmap) = value.byteCount
    }
    private val loading = HashSet<String>()
    private val failed = HashMap<String, Long>()
    private var listener: (() -> Unit)? = null

    fun onLoaded(l: () -> Unit) {
        listener = l
    }

    /** 主线程：有就给，没有就排下载（下好了 [onLoaded]） */
    fun get(x: Int, y: Int, z: Int, sat: Boolean): Bitmap? {
        val n = 1 shl z
        if (y < 0 || y >= n) return null
        val wx = ((x % n) + n) % n
        val key = "${if (sat) "s" else "r"}/$z/$wx/$y"
        mem.get(key)?.let { return it }
        if (key in loading) return null
        failed[key]?.let { if (System.currentTimeMillis() - it < RETRY_MS) return null }
        loading += key
        val url = source().url(wx, y, z, sat)
        pool.execute {
            val bmp = fetch(key, url)
            main.post {
                loading -= key
                if (bmp != null) {
                    mem.put(key, bmp)
                    listener?.invoke()
                } else {
                    failed[key] = System.currentTimeMillis()
                }
            }
        }
        return null
    }

    /** 按屏上像素缩好的瓦片（静止 / 平移时整张原样贴，软件画布上比每帧带滤波缩放快一个数量级），按字节算留 24MB */
    private val scaled = object : LruCache<String, Bitmap>(24 * 1024 * 1024) {
        override fun sizeOf(key: String, value: Bitmap) = value.byteCount
    }

    /** 主线程：缩到 [px]×[px] 的那一张（原图还没下好返回 null，同时排下载） */
    fun getScaled(x: Int, y: Int, z: Int, sat: Boolean, px: Int): Bitmap? {
        val base = get(x, y, z, sat) ?: return null
        if (base.width == px) return base
        val n = 1 shl z
        val key = "${if (sat) "s" else "r"}/$z/${((x % n) + n) % n}/$y@$px"
        return scaled.get(key) ?: Bitmap.createScaledBitmap(base, px, px, true).also { it.setHasAlpha(false); scaled.put(key, it) }
    }

    /** 主线程：同一级别没下好时，拿上几级里覆盖它的那块先顶着（放大时不白屏）。返回 (瓦片, 往上几级) */
    fun parent(x: Int, y: Int, z: Int, sat: Boolean): Pair<Bitmap, Int>? {
        var zz = z
        var xx = x
        var yy = y
        var d = 0
        while (zz > 0 && d < 4) {
            zz--
            d++
            xx = Math.floorDiv(xx, 2)
            yy = Math.floorDiv(yy, 2)
            val n = 1 shl zz
            val key = "${if (sat) "s" else "r"}/$zz/${((xx % n) + n) % n}/$yy"
            mem.get(key)?.let { return it to d }
        }
        return null
    }

    private fun fetch(key: String, url: String): Bitmap? {
        val f = File(disk, key.replace('/', '_') + ".img")
        if (f.isFile && f.length() > 0) {
            BitmapFactory.decodeFile(f.path, OPAQUE)?.let { return it.apply { setHasAlpha(false) } }
        }
        return try {
            val c = URL(url).openConnection() as HttpURLConnection
            c.connectTimeout = 8000
            c.readTimeout = 8000
            c.setRequestProperty("User-Agent", "mini-render-map/1.0")
            try {
                if (c.responseCode != 200) return null
                val bytes = c.inputStream.use { it.readBytes() }
                // 瓦片是不透明的：标成不透明，Skia 贴它走整行拷贝（带透明通道的要逐像素混合，软件画布上慢好几倍）
                val bmp = BitmapFactory.decodeByteArray(bytes, 0, bytes.size, OPAQUE)?.apply { setHasAlpha(false) } ?: return null
                runCatching { f.writeBytes(bytes) }
                bmp
            } finally {
                c.disconnect()
            }
        } catch (e: Exception) {
            Log.w("mini-render", "🗺 瓦片下载失败 $url：$e")
            null
        }
    }

    companion object {
        private const val RETRY_MS = 5000L
        /**
         * 解成 sRGB 的 ARGB_8888：瓦片 PNG 带 gAMA / cHRM 时系统会给它挂一个自定义色彩空间，往 sRGB 的画布上贴
         * 就是逐像素做色彩转换（一加 PLY110 上一张 383² 的瓦片 1.9ms，一屏 20 张 38ms）
         */
        private val OPAQUE = BitmapFactory.Options().apply {
            inPreferredConfig = Bitmap.Config.ARGB_8888
            inScaled = false
            if (android.os.Build.VERSION.SDK_INT >= 26) inPreferredColorSpace = android.graphics.ColorSpace.get(android.graphics.ColorSpace.Named.SRGB)
        }
        private val pool = Executors.newFixedThreadPool(4) { r -> Thread(r, "mini-map-tile").apply { isDaemon = true } }

        /** 标记图标：包内文件 / 网络图（下好了回主线程），按路径缓存 */
        private val icons = LruCache<String, Bitmap>(64)
        private val iconLoading = HashSet<String>()

        fun icon(path: String, file: File?, done: () -> Unit): Bitmap? {
            icons.get(path)?.let { return it }
            if (file == null && !path.startsWith("http")) return null
            if (!iconLoading.add(path)) return null
            pool.execute {
                val bmp = try {
                    if (file != null) BitmapFactory.decodeFile(file.path, OPAQUE)
                    else URL(path).openStream().use { BitmapFactory.decodeStream(it, null, OPAQUE) }
                } catch (e: Exception) {
                    null
                }
                Handler(Looper.getMainLooper()).post {
                    iconLoading.remove(path)
                    if (bmp != null) {
                        icons.put(path, bmp)
                        done()
                    }
                }
            }
            return null
        }
    }
}
