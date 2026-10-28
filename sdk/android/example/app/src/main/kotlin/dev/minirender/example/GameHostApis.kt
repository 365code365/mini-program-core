package dev.minirender.example

import android.app.Activity
import android.app.AlertDialog
import android.graphics.Color
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import dev.minirender.MiniGameView
import org.json.JSONObject

/**
 * 示例 App 给小游戏接的「宿主能力」（创建选项 hostApis 里点名的那些）：虚拟支付、激励视频、分享、登录。
 *
 * 都是**演示实现**：支付弹一个确认框（不扣钱）、广告是一个 5 秒倒计时的全屏页、分享弹提示、登录给一个假 code。
 * 正式接入时把这几个分支换成米大师 / 广告 SDK / 微信 OpenSDK 的真实调用，游戏侧代码不用改 ——
 * 游戏只认 wx.requestMidasPayment 的 success / fail 与广告的 onLoad / onClose({isEnded})。
 *
 * 协议：引擎发宿主事件 {"type":"hostCall","id","api","args"}；宿主处理完 [MiniGameView.hostReply] 回 success / fail；
 * 对象事件（广告的 load / close）用 [MiniGameView.dispatchEvent]("contextEvent", {context, contextId, event, data}) 推。
 */
class GameHostApis(private val activity: Activity, private val view: () -> MiniGameView?) {
    companion object {
        val APIS = listOf("requestMidasPayment", "checkIsSupportMidasPayment", "shareAppMessage", "login",
            "RewardedVideoAd.create", "RewardedVideoAd.load", "RewardedVideoAd.show", "RewardedVideoAd.destroy")
    }
    private val main = Handler(Looper.getMainLooper())

    /** GameActivity 的 onHostEvent 里收到 hostCall 就交给这里；不认识的返回 false */
    fun handle(call: JSONObject): Boolean {
        val id = call.optInt("id")
        val args = call.optJSONObject("args") ?: JSONObject()
        when (call.optString("api")) {
            "checkIsSupportMidasPayment" -> ok(id, JSONObject().put("isSupport", true))
            "requestMidasPayment" -> pay(id, args)
            "shareAppMessage" -> { toast("（演示）已分享：" + args.optString("title")); ok(id) }
            "login" -> ok(id, JSONObject().put("code", "demo_game_" + System.currentTimeMillis().toString(36)))
            "RewardedVideoAd.create" -> { ok(id); adEvent(args.optString("contextId"), "load", JSONObject()) }
            "RewardedVideoAd.load" -> { ok(id); adEvent(args.optString("contextId"), "load", JSONObject()) }
            "RewardedVideoAd.show" -> showAd(id, args.optString("contextId"))
            "RewardedVideoAd.destroy" -> ok(id)
            else -> return false
        }
        return true
    }

    private fun ok(id: Int, res: JSONObject = JSONObject()) { view()?.hostReply(id, true, res.toString()) }
    private fun fail(id: Int, msg: String, code: Int) { view()?.hostReply(id, false, JSONObject().put("errMsg", msg).put("errCode", code).toString()) }
    private fun adEvent(ctx: String, event: String, data: JSONObject) {
        view()?.dispatchEvent("contextEvent", JSONObject().put("context", "RewardedVideoAd").put("contextId", ctx).put("event", event).put("data", data).toString())
    }
    private fun toast(s: String) = main.post { android.widget.Toast.makeText(activity, s, android.widget.Toast.LENGTH_SHORT).show() }

    // 支付：buyQuantity 是游戏币数（1 元 = 10 游戏币，与游戏的 MIDAS.rate 一致）
    private fun pay(id: Int, args: JSONObject) = main.post {
        val yuan = args.optInt("buyQuantity") / 10.0
        AlertDialog.Builder(activity)
            .setTitle("模拟支付")
            .setMessage("示例 App 没有接入米大师。\n模拟支付 ¥%.2f？（不会真的扣款）".format(yuan))
            .setPositiveButton("支付") { _, _ -> ok(id, JSONObject().put("errMsg", "requestMidasPayment:ok")) }
            .setNegativeButton("取消") { _, _ -> fail(id, "requestMidasPayment:fail cancel", 1) }
            .setOnCancelListener { fail(id, "requestMidasPayment:fail cancel", 1) }
            .show()
    }

    // 激励视频：全屏 5 秒倒计时，到点才能「关闭并领奖」；提前关 = isEnded false
    private fun showAd(id: Int, ctx: String) = main.post {
        val root = activity.window.decorView as FrameLayout
        var left = 5
        var done = false
        val panel = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER
            setBackgroundColor(Color.argb(240, 20, 24, 40))
            isClickable = true
        }
        val title = TextView(activity).apply { text = "（演示）激励视频广告"; textSize = 20f; setTextColor(Color.WHITE); gravity = Gravity.CENTER }
        val count = TextView(activity).apply { textSize = 48f; setTextColor(Color.rgb(255, 209, 102)); gravity = Gravity.CENTER; setPadding(0, 40, 0, 40) }
        val close = TextView(activity).apply {
            textSize = 16f; setTextColor(Color.WHITE); gravity = Gravity.CENTER; setPadding(60, 30, 60, 30)
            setBackgroundColor(Color.rgb(90, 90, 110))
        }
        panel.addView(title); panel.addView(count); panel.addView(close)
        root.addView(panel, FrameLayout.LayoutParams(-1, -1))
        fun finish(ended: Boolean) {
            if (done) return
            done = true
            root.removeView(panel)
            adEvent(ctx, "close", JSONObject().put("isEnded", ended))
        }
        val tick = object : Runnable {
            override fun run() {
                if (done) return
                count.text = if (left > 0) "$left" else "✓"
                close.text = if (left > 0) "跳过（不发奖励）" else "关闭并领取奖励"
                if (left > 0) { left--; main.postDelayed(this, 1000) }
            }
        }
        close.setOnClickListener { finish(left <= 0 && count.text == "✓") }
        tick.run()
        ok(id)
    }
}
