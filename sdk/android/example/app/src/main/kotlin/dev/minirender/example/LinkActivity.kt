package dev.minirender.example

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.util.Log

/**
 * 外部 App / H5 拉起小程序的入口（清单里 `minirender://open` 的 intent-filter 在这一页上）：
 * 解析链接，转给 [MainActivity]，自己立刻关掉（不画任何东西）。
 *
 * 为什么单独一页而不是把 intent-filter 挂在 MainActivity 上：验收脚本用 `am start -n …/.MainActivity` 起内置示例，
 * MainActivity 的启动模式与任务归属不能动；这里用 CLEAR_TOP | SINGLE_TOP 送过去 ——
 * MainActivity 已经开着就走它的 onNewIntent（就地换小程序），没开就新起一个
 */
class LinkActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val url = intent?.dataString.orEmpty()
        val link = StoreLink.parse(url)
        Log.i(MainActivity.TAG, "[商店] 外部链接 $url → ${link?.let { "${it.key} ${it.route ?: "首页"}" } ?: "认不出"}")
        if (link != null) {
            startActivity(
                Intent(this, MainActivity::class.java)
                    .putExtra(MainActivity.EXTRA_LINK, link.toJson())
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP),
            )
        }
        finish()
    }
}
