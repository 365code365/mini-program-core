package dev.minirender.example

import android.app.Activity
import android.content.Context
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.text.Editable
import android.text.TextWatcher
import android.view.Gravity
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import kotlin.concurrent.thread

/**
 * 搜索小程序（照微信「搜一搜 · 小程序」）：
 * - 没输入：搜索历史（可清空）、热门搜索（服务器按搜索次数排，前三名标色）、按分类浏览；
 * - 输入中：联想词（停 200ms 再问服务器，过期的回包丢掉）；
 * - 回车 / 点联想词 / 点历史：结果列表（名称里命中的字标绿），上面一排分类可以再筛；
 * - 从入口页点某个分类进来（[EXTRA_CATEGORY]）：直接是这个分类的结果。
 * 搜索走服务器的 `/api/apps?q=&category=`（tools/mini-store/store_search.py），与管理页同一套排序。
 */
class SearchActivity : Activity() {
    private lateinit var store: MiniStore
    private lateinit var local: StoreLocal
    private lateinit var input: EditText
    private lateinit var body: LinearLayout
    private val main = Handler(Looper.getMainLooper())
    private var category = ""
    private var categories: List<Pair<String, Int>> = emptyList()
    /** 每发一次请求 +1：回包对不上就是过期的（用户又打了字） */
    private var seq = 0
    private val suggestTask = Runnable { suggest(input.text.toString().trim()) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        store = MiniStore(this, intent.getStringExtra(MainActivity.EXTRA_STORE_URL) ?: MiniStore.DEFAULT_URL)
        local = StoreLocal(this)
        category = intent.getStringExtra(EXTRA_CATEGORY).orEmpty()
        input = EditText(this).apply {
            hint = "搜索小程序 / 小游戏"
            textSize = 15f
            isSingleLine = true
            imeOptions = EditorInfo.IME_ACTION_SEARCH
            background = null
            setPadding(dp(8f), 0, dp(8f), 0)
            setOnEditorActionListener { _, id, ev ->
                if (id == EditorInfo.IME_ACTION_SEARCH || ev?.keyCode == KeyEvent.KEYCODE_ENTER) {
                    submit(text.toString())
                    true
                } else false
            }
            addTextChangedListener(object : TextWatcher {
                override fun beforeTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}
                override fun onTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}
                override fun afterTextChanged(s: Editable?) {
                    main.removeCallbacks(suggestTask)
                    if (s.isNullOrBlank()) idle() else main.postDelayed(suggestTask, 200)
                }
            })
        }
        val box = LinearLayout(this).apply {
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(10f), 0, dp(4f), 0)
            background = GradientDrawable().apply { cornerRadius = dp(8f).toFloat(); setColor(0xFFEDEDED.toInt()) }
            addView(TextView(context).apply { text = "🔍"; textSize = 13f })
            addView(input, LinearLayout.LayoutParams(0, dp(36f), 1f))
        }
        val bar = LinearLayout(this).apply {
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(12f), dp(8f), dp(4f), dp(8f))
            addView(box, LinearLayout.LayoutParams(0, -2, 1f))
            addView(TextView(context).apply {
                text = "取消"
                textSize = 15f
                setTextColor(0xFF576B95.toInt())
                setPadding(dp(12f), dp(8f), dp(12f), dp(8f))
                setOnClickListener { finish() }
            })
        }
        body = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.WHITE)
            addView(bar)
            addView(ScrollView(context).apply { addView(body) }, LinearLayout.LayoutParams(-1, 0, 1f))
            setOnApplyWindowInsetsListener { v, insets ->
                @Suppress("DEPRECATION")
                v.setPadding(0, insets.systemWindowInsetTop, 0, insets.systemWindowInsetBottom)
                insets
            }
        }
        setContentView(root)
        thread(name = "store-cats") {
            val c = runCatching { store.categories() }.getOrDefault(emptyList())
            runOnUiThread { categories = c; if (category.isNotEmpty()) results("") else if (input.text.isBlank()) idle() }
        }
        if (category.isNotEmpty()) results("") else {
            idle()
            input.requestFocus()
            main.postDelayed({ (getSystemService(INPUT_METHOD_SERVICE) as InputMethodManager).showSoftInput(input, 0) }, 150)
        }
    }

    private fun dp(v: Float) = StoreViews.dp(this, v)

    private fun submit(q: String) {
        val w = q.trim()
        if (w.isEmpty() && category.isEmpty()) return
        main.removeCallbacks(suggestTask)
        if (w.isNotEmpty()) local.addHistory(w)
        if (input.text.toString() != w) input.setText(w)
        input.setSelection(input.text.length)
        (getSystemService(INPUT_METHOD_SERVICE) as InputMethodManager).hideSoftInputFromWindow(input.windowToken, 0)
        results(w)
    }

    /** 没输入：历史、热搜、分类 */
    private fun idle() {
        val my = ++seq
        body.removeAllViews()
        val hist = local.history()
        if (hist.isNotEmpty()) {
            body.addView(StoreWidgets.header(this, "搜索历史", "清空") { local.clearHistory(); idle() })
            body.addView(flow(hist.map { w -> StoreWidgets.chip(this, w) { submit(w) } }))
        }
        val hotSlot = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        body.addView(hotSlot)
        if (categories.isNotEmpty()) {
            body.addView(StoreWidgets.header(this, "按分类浏览"))
            body.addView(flow(categories.map { (c, n) -> StoreWidgets.chip(this, "$c $n") { category = c; results(input.text.toString().trim()) } }))
        }
        thread(name = "store-hot") {
            val hot = runCatching { store.hotKeywords() }.getOrDefault(emptyList())
            runOnUiThread {
                if (my != seq || hot.isEmpty()) return@runOnUiThread
                hotSlot.addView(StoreWidgets.header(this, "热门搜索"))
                hotSlot.addView(hotGrid(hot))
            }
        }
    }

    /** 热搜两列，前三名的序号标红橙黄（微信的样子） */
    private fun hotGrid(words: List<String>): View {
        val col = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(dp(16f), 0, dp(16f), 0) }
        words.chunked(2).forEachIndexed { r, pair ->
            val row = LinearLayout(this)
            pair.forEachIndexed { c, w ->
                val rank = r * 2 + c + 1
                row.addView(LinearLayout(this).apply {
                    gravity = Gravity.CENTER_VERTICAL
                    setPadding(0, dp(9f), dp(8f), dp(9f))
                    isClickable = true
                    setOnClickListener { submit(w) }
                    addView(TextView(context).apply {
                        text = "$rank"
                        textSize = 14f
                        setTextColor(when (rank) { 1 -> 0xFFFA5151.toInt(); 2 -> 0xFFFA9D3B.toInt(); 3 -> 0xFFF7C543.toInt(); else -> 0x61000000 })
                        typeface = android.graphics.Typeface.DEFAULT_BOLD
                    }, LinearLayout.LayoutParams(dp(22f), -2))
                    addView(TextView(context).apply { text = w; textSize = 14f; maxLines = 1; setTextColor(0xDE000000.toInt()) })
                }, LinearLayout.LayoutParams(0, -2, 1f))
            }
            if (pair.size == 1) row.addView(View(this), LinearLayout.LayoutParams(0, 1, 1f))
            col.addView(row)
        }
        return col
    }

    /** 输入中：联想词 */
    private fun suggest(q: String) {
        if (q.isEmpty()) return
        val my = ++seq
        thread(name = "store-suggest") {
            val words = runCatching { store.suggest(q) }.getOrDefault(emptyList())
            runOnUiThread {
                if (my != seq) return@runOnUiThread
                body.removeAllViews()
                // 第一行永远是「搜索 xxx」：联想不到也能直接搜
                (listOf(q) + words.filter { it != q }).forEach { w ->
                    body.addView(TextView(this).apply {
                        text = StoreWidgets.highlight(w, q)
                        textSize = 15f
                        setTextColor(Color.BLACK)
                        setPadding(dp(16f), dp(13f), dp(16f), dp(13f))
                        setCompoundDrawablePadding(dp(10f))
                        setBackgroundResource(android.R.drawable.list_selector_background)
                        setOnClickListener { submit(w) }
                    })
                }
            }
        }
    }

    private fun results(q: String) {
        val my = ++seq
        body.removeAllViews()
        body.addView(categoryBar(q))
        val status = TextView(this).apply {
            text = "搜索中…"
            textSize = 13f
            setTextColor(StoreWidgets.SUB)
            setPadding(dp(16f), dp(10f), dp(16f), dp(4f))
        }
        body.addView(status)
        thread(name = "store-search") {
            val r = runCatching { store.search(q, category) }
            runOnUiThread {
                if (my != seq) return@runOnUiThread
                r.onSuccess { (apps, total) ->
                    status.text = when {
                        total == 0 && q.isNotEmpty() -> "没有找到「$q」相关的小程序，换个词试试"
                        total == 0 -> "这个分类下还没有小程序"
                        else -> "共 $total 个结果"
                    }
                    apps.forEach { body.addView(StoreWidgets.resultRow(this, store, it, q)) }
                }.onFailure { status.text = "搜索失败：${it.message ?: it.javaClass.simpleName}" }
            }
        }
    }

    /** 结果上方一排分类：「全部」+ 各分类，点了就在当前关键词下重新筛 */
    private fun categoryBar(q: String): View {
        val row = LinearLayout(this).apply { setPadding(dp(12f), dp(6f), dp(12f), dp(4f)) }
        (listOf("" to 0) + categories).forEach { (c, _) ->
            row.addView(StoreWidgets.chip(this, c.ifEmpty { "全部" }, selected = c == category) { category = c; results(q) },
                LinearLayout.LayoutParams(-2, -2).apply { marginEnd = dp(8f) })
        }
        return HorizontalScrollView(this).apply { isHorizontalScrollBarEnabled = false; addView(row) }
    }

    private fun flow(views: List<View>) = FlowLayout(this).apply {
        setPadding(dp(16f), dp(4f), dp(16f), dp(4f))
        views.forEach { addView(it) }
    }

    companion object {
        const val EXTRA_CATEGORY = "category"

        fun start(ctx: Context, storeUrl: String, category: String = "") {
            ctx.startActivity(android.content.Intent(ctx, SearchActivity::class.java)
                .putExtra(MainActivity.EXTRA_STORE_URL, storeUrl)
                .putExtra(EXTRA_CATEGORY, category))
        }
    }
}
