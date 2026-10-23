package dev.minirender.example

import android.app.Activity
import android.app.AlertDialog
import dev.minirender.MiniHostApi
import dev.minirender.MiniHostCall
import dev.minirender.MiniHostReply
import org.json.JSONArray
import org.json.JSONObject

/**
 * 示例的 hostApis：引擎做不了、SDK 也没自带，要 App 自己接的那些（登录、支付、定位、扫码……）。
 * 与 Flutter 示例的 `example/lib/host_apis.dart` 一一对应（同一组 API、同样的演示数据），demo 小程序的「宿主」tab
 * 在两个示例里结果一样。
 *
 * 真实 App 在这里接自己的账号体系、支付 SDK、地图 / 定位 SDK、扫码库。要用户点头的（支付、拨号、授权、选位置……）
 * 弹宿主自己的对话框 —— 表明调用确实走到了宿主、结果是宿主给的。用户取消一律 `<api>:fail cancel`（与微信一致）。
 * 选图 / 拍照 / 选视频 / 选文件 / 存相册 / 打开文档**不在这里**：SDK 自带（`MiniProgramView.builtinHostApis`）。
 */
class DemoHostApis(private val activity: Activity) {
    private val granted = mutableSetOf("scope.writePhotosAlbum", "scope.camera")

    /** api 名 → 处理器（登记进 `MiniProgramView.hostApis`） */
    val all: Map<String, MiniHostApi> = linkedMapOf(
        // ── 账号与授权 ──
        "login" to sync { JSONObject().put("code", "demo_" + System.currentTimeMillis().toString(36)) },
        "checkSession" to sync { JSONObject() },
        "getUserProfile" to ask({ "授权个人信息" to "小程序申请获取你的昵称、头像\n用途：${it.args.optString("desc")}" }, "允许", "auth deny") {
            JSONObject().put("userInfo", user())
        },
        "getUserInfo" to sync {
            JSONObject().put("userInfo", user()).put("rawData", user().toString()).put("signature", "demo_signature")
                .put("encryptedData", "ZGVtbw==").put("iv", "ZGVtbw==")
        },
        "authorize" to ask({ "授权" to "小程序申请使用：${scopeName(it.args.optString("scope"))}（${it.args.optString("scope")}）" }, "允许", "auth deny") {
            granted += it.args.optString("scope")
            JSONObject()
        },
        "openSetting" to sync { JSONObject().put("authSetting", JSONObject(SCOPES.keys.associateWith { granted.contains(it) })) },
        // ── 位置与地址 ──
        "getLocation" to sync {
            JSONObject().put("latitude", 22.5431).put("longitude", 114.0579).put("speed", 0).put("accuracy", 30)
                .put("altitude", 0).put("verticalAccuracy", 0).put("horizontalAccuracy", 30)
                .put("type", it.args.optString("type", "wgs84"))
        },
        "chooseLocation" to MiniHostApi(::chooseLocation),
        "openLocation" to show({
            val a = it.args
            "查看位置" to "${a.optString("name", "位置")}\n${a.optString("address")}\n${a.opt("latitude")}, ${a.opt("longitude")}（缩放 ${a.optInt("scale", 18)}）"
        }),
        "chooseAddress" to ask({ "选择收货地址" to "张三 13800000000\n广东省深圳市南山区科技园 1 号" }, "使用这个地址") {
            JSONObject().put("userName", "张三").put("postalCode", "518057").put("provinceName", "广东省").put("cityName", "深圳市")
                .put("countyName", "南山区").put("streetName", "粤海街道").put("detailInfo", "科技园 1 号")
                .put("detailInfoNew", "科技园 1 号").put("nationalCode", "440305").put("telNumber", "13800000000")
        },
        // ── 支付与消息 ──
        "requestPayment" to ask({ "模拟支付" to "小程序请求支付 ¥%.2f".format(it.args.optDouble("total", 0.0) / 100) }, "支付") { JSONObject() },
        "requestSubscribeMessage" to MiniHostApi { call, reply ->
            val ids = call.args.optJSONArray("tmplIds")?.let { a -> (0 until a.length()).map { a.optString(it) } }.orEmpty()
            confirm("订阅消息", "小程序想给你发送 ${ids.size} 类通知", "允许") { ok ->
                reply.success(JSONObject(ids.associateWith { if (ok) "accept" else "reject" }))
            }
        },
        "chooseInvoice" to ask({ "选择发票抬头" to "示例科技有限公司\n税号 91440300000000000X" }, "使用") {
            JSONObject().put("invoiceInfo", JSONObject().put("title", "示例科技有限公司").put("taxNumber", "91440300000000000X").put("type", 0).toString())
        },
        "openCustomerServiceChat" to show({
            "客服会话" to "（演示）宿主在这里打开企业微信客服：${it.args.optJSONObject("extInfo")?.optString("url").orEmpty()}"
        }),
        // ── 设备 ──
        "scanCode" to sync {
            JSONObject().put("result", "https://example.com/qr/demo").put("scanType", "QR_CODE").put("charSet", "UTF-8").put("path", "")
        },
        "makePhoneCall" to ask({ "拨打电话" to it.args.optString("phoneNumber") }, "呼叫") { JSONObject() },
        "chooseContact" to sync {
            JSONObject().put("phoneNumber", "13800000000").put("displayName", "李四")
                .put("phoneNumberList", JSONArray(listOf("13800000000", "010-12345678")))
        },
        "addPhoneContact" to ask({
            "添加到通讯录" to "${it.args.optString("lastName")}${it.args.optString("firstName")} ${it.args.optString("mobilePhoneNumber")}"
        }, "添加") { JSONObject() },
        "startSoterAuthentication" to MiniHostApi { call, reply ->
            val a = call.args
            confirm("生物认证", "${a.optString("authContent", "请验证")}（演示：点「验证」当作通过）", "验证") { ok ->
                if (!ok) return@confirm reply.fail("startSoterAuthentication:fail cancel", JSONObject().put("errCode", 90008))
                val mode = a.optJSONArray("requestAuthModes")?.optString(0)?.ifEmpty { null } ?: "fingerPrint"
                reply.success(
                    JSONObject().put("authMode", mode)
                        .put("resultJSON", JSONObject().put("raw", a.optString("challenge")).put("fid", "demo").toString())
                        .put("resultJSONSignature", "demo_signature").put("errCode", 0)
                )
            }
        },
        "getWeRunData" to sync { JSONObject().put("encryptedData", "ZGVtbw==").put("iv", "ZGVtbw==").put("cloudID", "demo_cloud_id") },
        // ── 跳转与分享 ──
        "navigateToMiniProgram" to ask({
            "打开小程序" to "即将打开 ${it.args.optString("appId")}\n路径 ${it.args.optString("path").ifEmpty { "首页" }}"
        }, "允许") { JSONObject() },
        // 只有被别的小程序打开时才回得去（与微信一致）；示例里小程序是宿主直接打开的
        "navigateBackMiniProgram" to MiniHostApi { _, reply ->
            reply.fail("navigateBackMiniProgram:fail 当前小程序不是由其它小程序打开的")
        },
        "getShareInfo" to sync { JSONObject().put("encryptedData", "ZGVtbw==").put("iv", "ZGVtbw==").put("cloudID", "demo_cloud_id") },
    )

    /** 选位置：宿主的界面列几个附近的地点（真实 App 用地图 SDK 的选点页） */
    private fun chooseLocation(call: MiniHostCall, reply: MiniHostReply) {
        val lat = call.args.optDouble("latitude", 39.908823)
        val lng = call.args.optDouble("longitude", 116.39747)
        val pois = listOf(
            listOf("天安门广场", "北京市东城区东长安街", 0.0, 0.0),
            listOf("故宫博物院", "北京市东城区景山前街 4 号", 0.0094, 0.0),
            listOf("国家博物馆", "北京市东城区东长安街 16 号", -0.0012, 0.0043),
            listOf("人民大会堂", "北京市西城区西长安街", -0.0012, -0.0044),
        )
        var picked = false
        AlertDialog.Builder(activity)
            .setTitle("选择位置（宿主界面，演示数据）")
            .setItems(pois.map { "${it[0]}\n${it[1]}" }.toTypedArray()) { _, i ->
                picked = true
                val p = pois[i]
                reply.success(
                    JSONObject().put("name", p[0]).put("address", p[1])
                        .put("latitude", lat + p[2] as Double).put("longitude", lng + p[3] as Double)
                )
            }
            .setOnDismissListener { if (!picked) reply.fail("chooseLocation:fail cancel") }
            .show()
    }

    // ── 三种写法：直接回 / 问一下再回 / 只展示 ──

    private fun sync(make: (MiniHostCall) -> JSONObject) = MiniHostApi { call, reply -> reply.success(make(call)) }

    /** 弹框问；点了确定才回 [make]，取消回 `<api>:fail <deny>` */
    private fun ask(
        text: (MiniHostCall) -> Pair<String, String>,
        okText: String,
        deny: String = "cancel",
        make: (MiniHostCall) -> JSONObject,
    ) = MiniHostApi { call, reply ->
        val (title, msg) = text(call)
        confirm(title, msg, okText) { ok -> if (ok) reply.success(make(call)) else reply.fail("${call.api}:fail $deny") }
    }

    /** 只有一个按钮的展示框（点了就算 ok） */
    private fun show(text: (MiniHostCall) -> Pair<String, String>) = MiniHostApi { call, reply ->
        val (title, msg) = text(call)
        AlertDialog.Builder(activity).setTitle(title).setMessage(msg)
            .setPositiveButton("好", null)
            .setOnDismissListener { reply.success() }
            .show()
    }

    /** 宿主自己的对话框；点外面 / 返回键当作取消 */
    private fun confirm(title: String, msg: String, okText: String, done: (Boolean) -> Unit) {
        if (activity.isFinishing) return done(false)
        var ok = false
        AlertDialog.Builder(activity).setTitle(title).setMessage(msg)
            .setNegativeButton("取消", null)
            .setPositiveButton(okText) { _, _ -> ok = true }
            .setOnDismissListener { done(ok) }
            .show()
    }

    private fun user() = JSONObject().put("nickName", "原生 Android 用户").put("avatarUrl", "").put("gender", 0)
        .put("language", "zh_CN").put("country", "").put("province", "").put("city", "")

    private fun scopeName(scope: String) = SCOPES[scope] ?: scope

    private companion object {
        val SCOPES = linkedMapOf(
            "scope.userLocation" to "精确地理位置",
            "scope.userFuzzyLocation" to "模糊地理位置",
            "scope.record" to "麦克风",
            "scope.camera" to "摄像头",
            "scope.writePhotosAlbum" to "保存到相册",
            "scope.address" to "通讯地址",
            "scope.invoiceTitle" to "发票抬头",
            "scope.werun" to "微信运动步数",
        )
    }
}
