package dev.minirender.example

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Outline
import android.os.Handler
import android.os.Looper
import android.util.LruCache
import android.view.View
import android.view.ViewOutlineProvider
import android.widget.FrameLayout
import android.widget.ImageView
import java.util.concurrent.Executors

/**
 * 商店图标：先画名字首字的圆（[StoreViews.icon]），服务器有 icon.png 就后台拉下来盖上去（圆形裁剪，与微信一样）。
 * 内存里留一份解码好的位图（LRU，按字节算），磁盘缓存在 [MiniStore.iconBytes]。
 */
object StoreIcons {
    private val pool = Executors.newFixedThreadPool(3) { r -> Thread(r, "store-icon").apply { isDaemon = true } }
    private val main = Handler(Looper.getMainLooper())
    private val mem = object : LruCache<String, Bitmap>(8 shl 20) {
        override fun sizeOf(key: String, value: Bitmap) = value.byteCount
    }

    private val circle = object : ViewOutlineProvider() {
        override fun getOutline(view: View, outline: Outline) = outline.setOval(0, 0, view.width, view.height)
    }

    fun view(store: MiniStore, app: MiniStore.App, sizeDp: Float): FrameLayout {
        val ctx = store.context
        val px = StoreViews.dp(ctx, sizeDp)
        val slot = FrameLayout(ctx)
        slot.layoutParams = android.widget.LinearLayout.LayoutParams(px, px)
        slot.addView(StoreViews.icon(ctx, app.id, app.name, sizeDp), FrameLayout.LayoutParams(px, px))
        if (app.iconUrl.isEmpty()) return slot
        val img = ImageView(ctx).apply {
            scaleType = ImageView.ScaleType.CENTER_CROP
            outlineProvider = circle
            clipToOutline = true
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        }
        slot.addView(img, FrameLayout.LayoutParams(px, px))
        val key = "${app.id}@${app.version}"
        mem.get(key)?.let { img.setImageBitmap(it); return slot }
        img.tag = key
        pool.execute {
            val bm = store.iconBytes(app)?.let { BitmapFactory.decodeByteArray(it, 0, it.size) } ?: return@execute
            mem.put(key, bm)
            // 列表刷新得快：视图已经换给别的图标就不贴了
            main.post { if (img.tag == key) img.setImageBitmap(bm) }
        }
        return slot
    }
}
